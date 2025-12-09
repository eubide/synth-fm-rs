use crate::algorithms;
use crate::command_queue::{
    create_command_queue, CommandReceiver, CommandSender, EffectParam, EffectType, EnvelopeParam,
    LfoParam, OperatorParam, SynthCommand,
};
use crate::effects::EffectsChain;
use crate::lfo::{LFOWaveform, LFO};
use crate::operator::Operator;
use crate::optimization::OPTIMIZATION_TABLES;
use crate::presets::{get_dx7_presets, Dx7Preset};
use crate::state_snapshot::{
    create_snapshot_channel, ChorusSnapshot, DelaySnapshot, OperatorSnapshot, ReverbSnapshot,
    SnapshotReceiver, SnapshotSender, SynthSnapshot,
};
use std::collections::HashMap;

const MAX_VOICES: usize = 16;

#[derive(Clone)]
pub struct Voice {
    pub operators: [Operator; 6],
    pub note: u8,
    pub frequency: f32,
    pub velocity: f32,
    pub active: bool,
    pub current_frequency: f32,
    pub target_frequency: f32,
    sample_rate: f32,
    fade_state: VoiceFadeState,
    fade_gain: f32,
    fade_rate: f32,
    note_on_id: u64,
}

#[derive(Clone, Debug, PartialEq)]
enum VoiceFadeState {
    Normal,
    FadeOut,
    FadeIn,
}

impl Voice {
    pub fn new_with_sample_rate(sample_rate: f32) -> Self {
        let mut operators = [
            Operator::new(sample_rate),
            Operator::new(sample_rate),
            Operator::new(sample_rate),
            Operator::new(sample_rate),
            Operator::new(sample_rate),
            Operator::new(sample_rate),
        ];

        for op in &mut operators {
            op.frequency_ratio = 1.0;
            op.output_level = 99.0;
            op.feedback = 0.0;
            op.detune = 0.0;
            op.envelope.rate1 = 99.0;
            op.envelope.rate2 = 50.0;
            op.envelope.rate3 = 50.0;
            op.envelope.rate4 = 50.0;
            op.envelope.level1 = 99.0;
            op.envelope.level2 = 75.0;
            op.envelope.level3 = 50.0;
            op.envelope.level4 = 0.0;
        }

        Self {
            operators,
            note: 0,
            frequency: 0.0,
            velocity: 0.0,
            active: false,
            current_frequency: 0.0,
            target_frequency: 0.0,
            sample_rate,
            fade_state: VoiceFadeState::Normal,
            fade_gain: 1.0,
            fade_rate: 0.001,
            note_on_id: 0,
        }
    }

    pub fn steal_voice(&mut self) {
        self.fade_state = VoiceFadeState::FadeOut;
        self.fade_rate = 1.0 / (self.sample_rate * 0.002);
    }

    pub fn trigger(&mut self, note: u8, velocity: f32, master_tune: f32, portamento_enable: bool) {
        self.note = note;
        let base_frequency = OPTIMIZATION_TABLES.get_midi_frequency(note);
        let new_frequency = base_frequency * 2.0_f32.powf((master_tune / 100.0) / 12.0);

        let use_portamento = portamento_enable
            && self.active
            && self.current_frequency > 0.0
            && (self.current_frequency - new_frequency).abs() > 0.1;

        self.frequency = new_frequency;

        if use_portamento {
            self.target_frequency = new_frequency;
        } else {
            self.current_frequency = new_frequency;
            self.target_frequency = new_frequency;
        }

        self.velocity = velocity;
        self.active = true;
        self.fade_state = VoiceFadeState::FadeIn;
        self.fade_gain = 0.0;
        self.fade_rate = 1.0 / (self.sample_rate * 0.005);

        for op in &mut self.operators {
            op.trigger(new_frequency, velocity, note);
        }
    }

    pub fn release(&mut self) {
        for op in &mut self.operators {
            op.release();
        }
    }

    pub fn stop(&mut self) {
        self.active = false;
        for op in &mut self.operators {
            op.reset();
        }
    }

    pub fn process(
        &mut self,
        algorithm_number: u8,
        pitch_bend: f32,
        pitch_bend_range: f32,
        portamento_time: f32,
        lfo_pitch_mod: f32,
        lfo_amp_mod: f32,
    ) -> f32 {
        if !self.active {
            return 0.0;
        }

        if self.current_frequency != self.target_frequency {
            let portamento_rate = if portamento_time > 0.0 {
                let time_seconds = 0.003 + (portamento_time / 99.0).powf(1.8) * 0.8;
                let samples_for_transition = time_seconds * self.sample_rate;
                1.0 / samples_for_transition.max(1.0)
            } else {
                1.0
            };

            let freq_ratio = self.target_frequency / self.current_frequency.max(0.001);
            let log_ratio = freq_ratio.ln();
            let step = log_ratio * portamento_rate;
            self.current_frequency *= (1.0 + step).clamp(0.5, 2.0);

            if (self.target_frequency - self.current_frequency).abs() < 0.1 {
                self.current_frequency = self.target_frequency;
            }
        }

        let bend_semitones = pitch_bend * pitch_bend_range;
        let bent_frequency = self.current_frequency * 2.0_f32.powf(bend_semitones / 12.0);
        let lfo_pitch_semitones = lfo_pitch_mod * 0.5;
        let lfo_pitch_factor = 2.0_f32.powf(lfo_pitch_semitones / 12.0);
        let final_frequency = bent_frequency * lfo_pitch_factor;

        for op in &mut self.operators {
            op.update_frequency_only(final_frequency);
        }

        let output = algorithms::process_algorithm(algorithm_number, &mut self.operators);
        let lfo_amp_factor = 1.0 + (lfo_amp_mod * 0.5);
        let modulated_output = output * lfo_amp_factor;

        let all_inactive = self.operators.iter().all(|op| !op.is_active());
        if all_inactive && self.fade_state != VoiceFadeState::FadeOut {
            self.active = false;
        }

        match self.fade_state {
            VoiceFadeState::FadeIn => {
                self.fade_gain += self.fade_rate;
                if self.fade_gain >= 1.0 {
                    self.fade_gain = 1.0;
                    self.fade_state = VoiceFadeState::Normal;
                }
                modulated_output * self.fade_gain
            }
            VoiceFadeState::FadeOut => {
                self.fade_gain -= self.fade_rate;
                if self.fade_gain <= 0.0 {
                    self.fade_gain = 0.0;
                    self.active = false;
                }
                modulated_output * self.fade_gain
            }
            VoiceFadeState::Normal => modulated_output,
        }
    }
}

/// SynthEngine - runs on the audio thread, processes commands and generates audio
pub struct SynthEngine {
    voices: Vec<Voice>,
    held_notes: HashMap<u8, usize>,
    pub preset_name: String,
    lfo: LFO,
    pub effects: EffectsChain,
    command_rx: CommandReceiver,
    snapshot_tx: SnapshotSender,
    note_counter: u64,
    // Cached parameters for real-time access
    algorithm: u8,
    master_volume: f32,
    pitch_bend: f32,
    mod_wheel: f32,
    master_tune: f32,
    pitch_bend_range: f32,
    portamento_enable: bool,
    portamento_time: f32,
    mono_mode: bool,
    sustain_pedal: bool,
    #[allow(dead_code)]
    sample_rate: f32,
}

impl SynthEngine {
    pub fn new(sample_rate: f32, command_rx: CommandReceiver, snapshot_tx: SnapshotSender) -> Self {
        let mut voices = Vec::with_capacity(MAX_VOICES);
        for _ in 0..MAX_VOICES {
            voices.push(Voice::new_with_sample_rate(sample_rate));
        }

        Self {
            voices,
            held_notes: HashMap::new(),
            preset_name: "Init Voice".to_string(),
            lfo: LFO::new(sample_rate),
            effects: EffectsChain::new(sample_rate),
            command_rx,
            snapshot_tx,
            note_counter: 0,
            algorithm: 1,
            master_volume: 0.7,
            pitch_bend: 0.0,
            mod_wheel: 0.0,
            master_tune: 0.0,
            pitch_bend_range: 2.0,
            portamento_enable: false,
            portamento_time: 50.0,
            mono_mode: false,
            sustain_pedal: false,
            sample_rate,
        }
    }

    /// Process all pending commands from GUI/MIDI
    pub fn process_commands(&mut self) {
        while let Some(cmd) = self.command_rx.try_recv() {
            self.handle_command(cmd);
        }
    }

    fn handle_command(&mut self, cmd: SynthCommand) {
        match cmd {
            SynthCommand::NoteOn { note, velocity } => self.note_on(note, velocity),
            SynthCommand::NoteOff { note } => self.note_off(note),
            SynthCommand::SetAlgorithm(alg) => {
                if (1..=32).contains(&alg) {
                    self.algorithm = alg;
                }
            }
            SynthCommand::SetMasterVolume(vol) => {
                self.master_volume = vol.clamp(0.0, 1.0);
            }
            SynthCommand::SetMasterTune(cents) => {
                self.master_tune = cents.clamp(-150.0, 150.0);
            }
            SynthCommand::SetMonoMode(mono) => {
                self.mono_mode = mono;
                if mono {
                    let mut first_active_found = false;
                    for voice in &mut self.voices {
                        if voice.active {
                            if first_active_found {
                                voice.stop();
                            } else {
                                first_active_found = true;
                            }
                        }
                    }
                }
            }
            SynthCommand::SetPitchBendRange(range) => {
                self.pitch_bend_range = range.clamp(0.0, 12.0);
            }
            SynthCommand::SetPortamentoEnable(enable) => {
                self.portamento_enable = enable;
            }
            SynthCommand::SetPortamentoTime(time) => {
                self.portamento_time = time.clamp(0.0, 99.0);
            }
            SynthCommand::PitchBend(value) => {
                self.pitch_bend = value as f32 / 8192.0;
            }
            SynthCommand::ModWheel(value) => {
                self.mod_wheel = value;
            }
            SynthCommand::SustainPedal(pressed) => {
                self.sustain_pedal = pressed;
            }
            SynthCommand::SetOperatorParam {
                operator,
                param,
                value,
            } => {
                self.set_operator_param(operator as usize, param, value);
            }
            SynthCommand::SetEnvelopeParam {
                operator,
                param,
                value,
            } => {
                self.set_envelope_param(operator as usize, param, value);
            }
            SynthCommand::SetLfoParam { param, value } => {
                self.set_lfo_param(param, value);
            }
            SynthCommand::SetEffectParam {
                effect,
                param,
                value,
            } => {
                self.set_effect_param(effect, param, value);
            }
            SynthCommand::LoadPreset(_preset_idx) => {
                // Preset loading handled by controller
            }
            SynthCommand::VoiceInitialize => {
                self.voice_initialize();
            }
            SynthCommand::Panic => {
                self.panic();
            }
        }
    }

    fn note_on(&mut self, note: u8, velocity: u8) {
        let velocity_f = velocity as f32 / 127.0;
        self.note_counter = self.note_counter.wrapping_add(1);
        self.lfo.trigger();

        if self.mono_mode {
            self.held_notes.clear();
            self.voices[0].trigger(note, velocity_f, self.master_tune, self.portamento_enable);
            self.voices[0].note_on_id = self.note_counter;
            self.held_notes.insert(note, 0);
        } else {
            if let Some(&voice_idx) = self.held_notes.get(&note) {
                self.voices[voice_idx].trigger(note, velocity_f, self.master_tune, false);
                self.voices[voice_idx].note_on_id = self.note_counter;
                return;
            }

            for (i, voice) in self.voices.iter_mut().enumerate() {
                if !voice.active {
                    voice.trigger(note, velocity_f, self.master_tune, false);
                    voice.note_on_id = self.note_counter;
                    self.held_notes.insert(note, i);
                    return;
                }
            }

            let oldest_voice = self
                .voices
                .iter()
                .enumerate()
                .min_by_key(|(_, v)| v.note_on_id)
                .map(|(i, _)| i)
                .unwrap_or(0);

            self.voices[oldest_voice].steal_voice();
            self.voices[oldest_voice].trigger(note, velocity_f, self.master_tune, false);
            self.voices[oldest_voice].note_on_id = self.note_counter;

            self.held_notes.retain(|_, &mut v| v != oldest_voice);
            self.held_notes.insert(note, oldest_voice);
        }
    }

    fn note_off(&mut self, note: u8) {
        if let Some(&voice_idx) = self.held_notes.get(&note) {
            if !self.sustain_pedal {
                self.voices[voice_idx].release();
                self.held_notes.remove(&note);
            }
        }
    }

    fn set_operator_param(&mut self, op_index: usize, param: OperatorParam, value: f32) {
        if op_index >= 6 {
            return;
        }
        for voice in &mut self.voices {
            match param {
                OperatorParam::Ratio => voice.operators[op_index].set_frequency_ratio(value),
                OperatorParam::Level => voice.operators[op_index].output_level = value,
                OperatorParam::Detune => voice.operators[op_index].set_detune(value),
                OperatorParam::Feedback => voice.operators[op_index].feedback = value,
                OperatorParam::VelocitySensitivity => {
                    voice.operators[op_index].velocity_sensitivity = value
                }
                OperatorParam::KeyScaleLevel => voice.operators[op_index].key_scale_level = value,
                OperatorParam::KeyScaleRate => voice.operators[op_index].key_scale_rate = value,
                OperatorParam::Enabled => voice.operators[op_index].enabled = value > 0.5,
            }
        }
    }

    fn set_envelope_param(&mut self, op_index: usize, param: EnvelopeParam, value: f32) {
        if op_index >= 6 {
            return;
        }
        for voice in &mut self.voices {
            match param {
                EnvelopeParam::Rate1 => voice.operators[op_index].envelope.rate1 = value,
                EnvelopeParam::Rate2 => voice.operators[op_index].envelope.rate2 = value,
                EnvelopeParam::Rate3 => voice.operators[op_index].envelope.rate3 = value,
                EnvelopeParam::Rate4 => voice.operators[op_index].envelope.rate4 = value,
                EnvelopeParam::Level1 => voice.operators[op_index].envelope.level1 = value,
                EnvelopeParam::Level2 => voice.operators[op_index].envelope.level2 = value,
                EnvelopeParam::Level3 => voice.operators[op_index].envelope.level3 = value,
                EnvelopeParam::Level4 => voice.operators[op_index].envelope.level4 = value,
            }
        }
    }

    fn set_lfo_param(&mut self, param: LfoParam, value: f32) {
        match param {
            LfoParam::Rate => self.lfo.set_rate(value),
            LfoParam::Delay => self.lfo.set_delay(value),
            LfoParam::PitchDepth => self.lfo.set_pitch_depth(value),
            LfoParam::AmpDepth => self.lfo.set_amp_depth(value),
            LfoParam::Waveform(w) => {
                let waveform = match w {
                    0 => LFOWaveform::Triangle,
                    1 => LFOWaveform::SawDown,
                    2 => LFOWaveform::SawUp,
                    3 => LFOWaveform::Square,
                    4 => LFOWaveform::Sine,
                    _ => LFOWaveform::SampleHold,
                };
                self.lfo.set_waveform(waveform);
            }
            LfoParam::KeySync => self.lfo.set_key_sync(value > 0.5),
        }
    }

    fn set_effect_param(&mut self, effect: EffectType, param: EffectParam, value: f32) {
        match effect {
            EffectType::Chorus => match param {
                EffectParam::Enabled => self.effects.chorus.enabled = value > 0.5,
                EffectParam::Mix => self.effects.chorus.mix = value,
                EffectParam::ChorusRate => self.effects.chorus.rate = value,
                EffectParam::ChorusDepth => self.effects.chorus.depth = value,
                EffectParam::ChorusFeedback => self.effects.chorus.feedback = value,
                _ => {}
            },
            EffectType::Delay => match param {
                EffectParam::Enabled => self.effects.delay.enabled = value > 0.5,
                EffectParam::Mix => self.effects.delay.mix = value,
                EffectParam::DelayTime => self.effects.delay.time_ms = value,
                EffectParam::DelayFeedback => self.effects.delay.feedback = value,
                EffectParam::DelayPingPong => self.effects.delay.ping_pong = value > 0.5,
                _ => {}
            },
            EffectType::Reverb => match param {
                EffectParam::Enabled => self.effects.reverb.enabled = value > 0.5,
                EffectParam::Mix => self.effects.reverb.mix = value,
                EffectParam::ReverbRoomSize => self.effects.reverb.room_size = value,
                EffectParam::ReverbDamping => self.effects.reverb.damping = value,
                EffectParam::ReverbWidth => self.effects.reverb.width = value,
                _ => {}
            },
        }
    }

    fn voice_initialize(&mut self) {
        self.preset_name = "Init Voice".to_string();
        self.algorithm = 1;

        for voice in &mut self.voices {
            voice.stop();
        }
        self.held_notes.clear();

        for voice in &mut self.voices {
            for op in voice.operators.iter_mut() {
                op.frequency_ratio = 1.0;
                op.output_level = 99.0;
                op.detune = 0.0;
                op.feedback = 0.0;
                op.velocity_sensitivity = 0.0;
                op.key_scale_level = 0.0;
                op.key_scale_rate = 0.0;
                op.envelope.rate1 = 99.0;
                op.envelope.rate2 = 50.0;
                op.envelope.rate3 = 50.0;
                op.envelope.rate4 = 50.0;
                op.envelope.level1 = 99.0;
                op.envelope.level2 = 75.0;
                op.envelope.level3 = 50.0;
                op.envelope.level4 = 0.0;
            }
        }
    }

    fn panic(&mut self) {
        for voice in &mut self.voices {
            voice.active = false;
            for op in &mut voice.operators {
                op.reset();
            }
        }
        self.held_notes.clear();
    }

    /// Process one sample of audio (mono)
    pub fn process(&mut self) -> f32 {
        let mut output = 0.0;
        let mut active_voice_count = 0;

        let (lfo_pitch_mod, lfo_amp_mod) = self.lfo.process(self.mod_wheel);

        for voice in &mut self.voices {
            if voice.active {
                let voice_output = voice.process(
                    self.algorithm,
                    self.pitch_bend,
                    self.pitch_bend_range,
                    self.portamento_time,
                    lfo_pitch_mod,
                    lfo_amp_mod,
                );
                output += voice_output;
                active_voice_count += 1;
            }
        }

        let voice_scaling = if active_voice_count > 0 {
            OPTIMIZATION_TABLES.get_voice_scale(active_voice_count)
        } else {
            1.0
        };

        let scaled_output = output * voice_scaling * self.master_volume;
        self.soft_limit(scaled_output)
    }

    /// Process audio with effects, returns stereo pair (left, right)
    pub fn process_stereo(&mut self) -> (f32, f32) {
        let mono = self.process();
        let (left, right) = self.effects.process(mono);
        (self.soft_limit(left), self.soft_limit(right))
    }

    /// Update and send snapshot to GUI
    pub fn update_snapshot(&self) {
        let mut active_voices = 0u8;
        for voice in &self.voices {
            if voice.active {
                active_voices += 1;
            }
        }

        let snapshot = SynthSnapshot {
            preset_name: self.preset_name.clone(),
            algorithm: self.algorithm,
            active_voices,
            master_volume: self.master_volume,
            master_tune: self.master_tune,
            mono_mode: self.mono_mode,
            portamento_enable: self.portamento_enable,
            portamento_time: self.portamento_time,
            pitch_bend_range: self.pitch_bend_range,
            pitch_bend: self.pitch_bend,
            mod_wheel: self.mod_wheel,
            sustain_pedal: self.sustain_pedal,
            lfo_rate: self.lfo.rate,
            lfo_delay: self.lfo.delay,
            lfo_pitch_depth: self.lfo.pitch_depth,
            lfo_amp_depth: self.lfo.amp_depth,
            lfo_waveform: self.lfo.waveform,
            lfo_key_sync: self.lfo.key_sync,
            lfo_frequency_hz: self.lfo.get_frequency_hz(),
            lfo_delay_seconds: self.lfo.get_delay_seconds(),
            chorus: ChorusSnapshot {
                enabled: self.effects.chorus.enabled,
                rate: self.effects.chorus.rate,
                depth: self.effects.chorus.depth,
                mix: self.effects.chorus.mix,
                feedback: self.effects.chorus.feedback,
            },
            delay: DelaySnapshot {
                enabled: self.effects.delay.enabled,
                time_ms: self.effects.delay.time_ms,
                feedback: self.effects.delay.feedback,
                mix: self.effects.delay.mix,
                ping_pong: self.effects.delay.ping_pong,
            },
            reverb: ReverbSnapshot {
                enabled: self.effects.reverb.enabled,
                room_size: self.effects.reverb.room_size,
                damping: self.effects.reverb.damping,
                mix: self.effects.reverb.mix,
                width: self.effects.reverb.width,
            },
            operators: self.get_operator_snapshots(),
        };

        self.snapshot_tx.send(snapshot);
    }

    fn get_operator_snapshots(&self) -> [OperatorSnapshot; 6] {
        if let Some(voice) = self.voices.first() {
            let mut snapshots = [OperatorSnapshot::default(); 6];
            for (i, op) in voice.operators.iter().enumerate() {
                snapshots[i] = OperatorSnapshot {
                    enabled: op.enabled,
                    frequency_ratio: op.frequency_ratio,
                    output_level: op.output_level,
                    detune: op.detune,
                    feedback: op.feedback,
                    velocity_sensitivity: op.velocity_sensitivity,
                    key_scale_level: op.key_scale_level,
                    key_scale_rate: op.key_scale_rate,
                    rate1: op.envelope.rate1,
                    rate2: op.envelope.rate2,
                    rate3: op.envelope.rate3,
                    rate4: op.envelope.rate4,
                    level1: op.envelope.level1,
                    level2: op.envelope.level2,
                    level3: op.envelope.level3,
                    level4: op.envelope.level4,
                };
            }
            snapshots
        } else {
            [OperatorSnapshot::default(); 6]
        }
    }

    fn soft_limit(&self, sample: f32) -> f32 {
        const THRESHOLD: f32 = 0.85;
        const KNEE: f32 = 0.15;

        if sample.abs() <= THRESHOLD {
            sample
        } else {
            let sign = sample.signum();
            let abs_sample = sample.abs();
            let excess = abs_sample - THRESHOLD;
            let compressed_excess = excess / (1.0 + excess / KNEE);
            let limited = THRESHOLD + compressed_excess;
            sign * limited.min(0.95)
        }
    }

    // Public getters for direct access (used by presets)
    pub fn voices_mut(&mut self) -> &mut Vec<Voice> {
        &mut self.voices
    }

    pub fn set_preset_name(&mut self, name: String) {
        self.preset_name = name;
    }

    pub fn set_algorithm(&mut self, alg: u8) {
        if (1..=32).contains(&alg) {
            self.algorithm = alg;
        }
    }

    #[allow(dead_code)]
    pub fn lfo_mut(&mut self) -> &mut LFO {
        &mut self.lfo
    }

    // Public read-only getters (kept for API completeness, GUI now uses snapshots)
    #[allow(dead_code)]
    pub fn get_algorithm(&self) -> u8 {
        self.algorithm
    }

    #[allow(dead_code)]
    pub fn get_master_volume(&self) -> f32 {
        self.master_volume
    }

    #[allow(dead_code)]
    pub fn get_master_tune(&self) -> f32 {
        self.master_tune
    }

    #[allow(dead_code)]
    pub fn get_mono_mode(&self) -> bool {
        self.mono_mode
    }

    #[allow(dead_code)]
    pub fn get_portamento_enable(&self) -> bool {
        self.portamento_enable
    }

    #[allow(dead_code)]
    pub fn get_portamento_time(&self) -> f32 {
        self.portamento_time
    }

    #[allow(dead_code)]
    pub fn get_pitch_bend_range(&self) -> f32 {
        self.pitch_bend_range
    }

    #[allow(dead_code)]
    pub fn get_mod_wheel(&self) -> f32 {
        self.mod_wheel
    }

    #[allow(dead_code)]
    pub fn get_lfo_rate(&self) -> f32 {
        self.lfo.rate
    }

    #[allow(dead_code)]
    pub fn get_lfo_delay(&self) -> f32 {
        self.lfo.delay
    }

    #[allow(dead_code)]
    pub fn get_lfo_pitch_depth(&self) -> f32 {
        self.lfo.pitch_depth
    }

    #[allow(dead_code)]
    pub fn get_lfo_amp_depth(&self) -> f32 {
        self.lfo.amp_depth
    }

    #[allow(dead_code)]
    pub fn get_lfo_waveform(&self) -> LFOWaveform {
        self.lfo.waveform
    }

    #[allow(dead_code)]
    pub fn get_lfo_key_sync(&self) -> bool {
        self.lfo.key_sync
    }

    #[allow(dead_code)]
    pub fn get_lfo_frequency_hz(&self) -> f32 {
        self.lfo.get_frequency_hz()
    }

    #[allow(dead_code)]
    pub fn get_lfo_delay_seconds(&self) -> f32 {
        self.lfo.get_delay_seconds()
    }

    #[allow(dead_code)]
    pub fn get_operator_enabled(&self, op_idx: usize) -> bool {
        if let Some(voice) = self.voices.first() {
            if op_idx < 6 {
                return voice.operators[op_idx].enabled;
            }
        }
        true
    }

    #[allow(dead_code)]
    pub fn voices(&self) -> &Vec<Voice> {
        &self.voices
    }
}

/// SynthController - interface for GUI/MIDI threads to control the synthesizer
pub struct SynthController {
    command_tx: CommandSender,
    snapshot_rx: SnapshotReceiver,
}

impl SynthController {
    pub fn new(command_tx: CommandSender, snapshot_rx: SnapshotReceiver) -> Self {
        Self {
            command_tx,
            snapshot_rx,
        }
    }

    /// Get the latest snapshot from the audio thread (reference)
    #[allow(dead_code)]
    pub fn get_snapshot(&self) -> &SynthSnapshot {
        self.snapshot_rx.get()
    }

    /// Get a copy of the latest snapshot (for GUI use)
    pub fn snapshot(&self) -> SynthSnapshot {
        self.snapshot_rx.get().clone()
    }

    /// Send a command to the audio thread
    pub fn send(&mut self, command: SynthCommand) -> bool {
        self.command_tx.send(command)
    }

    // Convenience methods for common operations
    pub fn note_on(&mut self, note: u8, velocity: u8) {
        self.send(SynthCommand::NoteOn { note, velocity });
    }

    pub fn note_off(&mut self, note: u8) {
        self.send(SynthCommand::NoteOff { note });
    }

    pub fn set_algorithm(&mut self, algorithm: u8) {
        self.send(SynthCommand::SetAlgorithm(algorithm));
    }

    pub fn set_master_volume(&mut self, volume: f32) {
        self.send(SynthCommand::SetMasterVolume(volume));
    }

    pub fn set_master_tune(&mut self, cents: f32) {
        self.send(SynthCommand::SetMasterTune(cents));
    }

    pub fn set_mono_mode(&mut self, mono: bool) {
        self.send(SynthCommand::SetMonoMode(mono));
    }

    pub fn set_pitch_bend_range(&mut self, range: f32) {
        self.send(SynthCommand::SetPitchBendRange(range));
    }

    pub fn set_portamento_enable(&mut self, enable: bool) {
        self.send(SynthCommand::SetPortamentoEnable(enable));
    }

    pub fn set_portamento_time(&mut self, time: f32) {
        self.send(SynthCommand::SetPortamentoTime(time));
    }

    pub fn pitch_bend(&mut self, value: i16) {
        self.send(SynthCommand::PitchBend(value));
    }

    pub fn mod_wheel(&mut self, value: f32) {
        self.send(SynthCommand::ModWheel(value));
    }

    pub fn sustain_pedal(&mut self, pressed: bool) {
        self.send(SynthCommand::SustainPedal(pressed));
    }

    pub fn set_operator_param(&mut self, operator: u8, param: OperatorParam, value: f32) {
        self.send(SynthCommand::SetOperatorParam {
            operator,
            param,
            value,
        });
    }

    pub fn set_envelope_param(&mut self, operator: u8, param: EnvelopeParam, value: f32) {
        self.send(SynthCommand::SetEnvelopeParam {
            operator,
            param,
            value,
        });
    }

    pub fn set_lfo_param(&mut self, param: LfoParam, value: f32) {
        self.send(SynthCommand::SetLfoParam { param, value });
    }

    pub fn set_effect_param(&mut self, effect: EffectType, param: EffectParam, value: f32) {
        self.send(SynthCommand::SetEffectParam {
            effect,
            param,
            value,
        });
    }

    pub fn voice_initialize(&mut self) {
        self.send(SynthCommand::VoiceInitialize);
    }

    pub fn panic(&mut self) {
        self.send(SynthCommand::Panic);
    }
}

/// Create a new synthesizer engine and controller pair
pub fn create_synth(sample_rate: f32) -> (SynthEngine, SynthController) {
    let (command_tx, command_rx) = create_command_queue();
    let (snapshot_tx, snapshot_rx) = create_snapshot_channel();

    let engine = SynthEngine::new(sample_rate, command_rx, snapshot_tx);
    let controller = SynthController::new(command_tx, snapshot_rx);

    (engine, controller)
}
