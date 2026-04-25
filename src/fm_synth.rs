use crate::algorithms;
use crate::command_queue::{
    create_command_queue, CommandReceiver, CommandSender, EffectParam, EffectType, EnvelopeParam,
    LfoParam, OperatorParam, PitchEgParam, SynthCommand,
};
use crate::effects::EffectsChain;
use crate::lfo::{LFOWaveform, LFO};
use crate::operator::{KeyScaleCurve, Operator};
use crate::optimization::OPTIMIZATION_TABLES;
use crate::pitch_eg::PitchEg;
use crate::presets::Dx7Preset;
use crate::state_snapshot::{
    create_snapshot_channel, ChorusSnapshot, DelaySnapshot, OperatorSnapshot, PitchEgSnapshot,
    ReverbSnapshot, SnapshotReceiver, SnapshotSender, SynthSnapshot, VoiceMode,
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

    /// Retarget the active voice to a new MIDI note without re-triggering envelopes.
    /// Used by mono-legato to glide back to a held note when the topmost note is released.
    /// Honours portamento when `portamento` is true.
    pub fn retarget(&mut self, note: u8, master_tune: f32, portamento: bool) {
        self.note = note;
        let base_frequency = OPTIMIZATION_TABLES.get_midi_frequency(note);
        let new_frequency = base_frequency * 2.0_f32.powf((master_tune / 100.0) / 12.0);
        self.frequency = new_frequency;
        if portamento && self.current_frequency > 0.0 {
            self.target_frequency = new_frequency;
        } else {
            self.current_frequency = new_frequency;
            self.target_frequency = new_frequency;
        }
    }

    pub fn stop(&mut self) {
        self.active = false;
        for op in &mut self.operators {
            op.reset();
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn process(
        &mut self,
        algorithm_number: u8,
        pitch_bend: f32,
        pitch_bend_range: f32,
        portamento_time: f32,
        glissando: bool,
        lfo_pitch_mod: f32,
        lfo_amp_mod: f32,
        pitch_eg_semitones: f32,
    ) -> f32 {
        if !self.active {
            return 0.0;
        }

        if self.current_frequency != self.target_frequency {
            let portamento_rate = if portamento_time > 0.0 {
                // Authentic DX7-style portamento: 5ms to 2.5s range
                let time_seconds = 0.005 + (portamento_time / 99.0).powf(2.0) * 2.5;
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

        // Glissando quantises the live pitch to the nearest semitone, producing
        // a stepped glide instead of a continuous one.
        let played_frequency = if glissando {
            quantize_to_semitone(self.current_frequency)
        } else {
            self.current_frequency
        };

        let bend_semitones = pitch_bend * pitch_bend_range;
        let bent_frequency = played_frequency * 2.0_f32.powf(bend_semitones / 12.0);
        let lfo_pitch_semitones = lfo_pitch_mod * 0.5;
        let total_pitch_offset = lfo_pitch_semitones + pitch_eg_semitones;
        let final_frequency = bent_frequency * 2.0_f32.powf(total_pitch_offset / 12.0);

        for op in &mut self.operators {
            op.update_frequency_only(final_frequency);
            op.set_lfo_amp_mod(lfo_amp_mod);
        }

        let output = algorithms::process_algorithm(algorithm_number, &mut self.operators);

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
                output * self.fade_gain
            }
            VoiceFadeState::FadeOut => {
                self.fade_gain -= self.fade_rate;
                if self.fade_gain <= 0.0 {
                    self.fade_gain = 0.0;
                    self.active = false;
                }
                output * self.fade_gain
            }
            VoiceFadeState::Normal => output,
        }
    }
}

/// Round a frequency to the nearest equal-tempered semitone (relative to A4 = 440 Hz).
fn quantize_to_semitone(freq: f32) -> f32 {
    if freq <= 0.0 {
        return freq;
    }
    let semitones_from_a4 = (freq / 440.0).log2() * 12.0;
    let rounded = semitones_from_a4.round();
    440.0 * 2.0_f32.powf(rounded / 12.0)
}

/// SynthEngine - runs on the audio thread, processes commands and generates audio
pub struct SynthEngine {
    voices: Vec<Voice>,
    held_notes: HashMap<u8, usize>,
    /// Order in which currently-held notes were pressed (front = oldest, back = newest).
    /// Used by mono modes to fall back to the previous held note when the active one is released.
    mono_held_order: Vec<u8>,
    pub preset_name: String,
    lfo: LFO,
    pub pitch_eg: PitchEg,
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
    portamento_glissando: bool,
    voice_mode: VoiceMode,
    transpose_semitones: i8,
    pitch_mod_sensitivity: u8,
    sustain_pedal: bool,
    #[allow(dead_code)]
    sample_rate: f32,
    // Preset storage for MIDI program change
    presets: Vec<Dx7Preset>,
    current_preset_index: usize,
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
            mono_held_order: Vec::with_capacity(8),
            preset_name: "Init Voice".to_string(),
            lfo: LFO::new(sample_rate),
            pitch_eg: PitchEg::new(sample_rate),
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
            portamento_glissando: false,
            voice_mode: VoiceMode::Poly,
            transpose_semitones: 0,
            pitch_mod_sensitivity: 0,
            sustain_pedal: false,
            sample_rate,
            presets: Vec::new(),
            current_preset_index: 0,
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
            SynthCommand::SetVoiceMode(mode) => {
                let new_mode = match mode {
                    1 => VoiceMode::Mono,
                    2 => VoiceMode::MonoLegato,
                    _ => VoiceMode::Poly,
                };
                self.voice_mode = new_mode;
                if new_mode != VoiceMode::Poly {
                    // Switching to mono: silence all but voice 0, clear hold map.
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
                    self.held_notes.clear();
                    self.mono_held_order.clear();
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
            SynthCommand::SetPortamentoGlissando(on) => {
                self.portamento_glissando = on;
            }
            SynthCommand::SetTranspose(st) => {
                self.transpose_semitones = st.clamp(-24, 24);
            }
            SynthCommand::SetPitchModSensitivity(pms) => {
                self.pitch_mod_sensitivity = pms.min(7);
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
            SynthCommand::SetPitchEgParam { param, value } => {
                self.set_pitch_eg_param(param, value);
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
            SynthCommand::LoadPreset(preset_idx) => {
                self.load_preset(preset_idx);
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

        // Mono-Legato suppresses LFO/PEG retrigger while another note is held —
        // matching DX7 behaviour where a tied note keeps the previous envelope alive.
        let suppress_retrigger =
            self.voice_mode == VoiceMode::MonoLegato && !self.mono_held_order.is_empty();
        if !suppress_retrigger {
            self.lfo.trigger();
            self.pitch_eg.trigger();
        }

        let effective_note = self.apply_transpose(note);

        match self.voice_mode {
            VoiceMode::Mono => {
                // Full portamento: glide from previous note whenever portamento is enabled.
                self.mono_trigger(note, effective_note, velocity_f, self.portamento_enable);
            }
            VoiceMode::MonoLegato => {
                // Legato portamento: only glide if there is a previous note still held.
                let legato = self.portamento_enable && !self.mono_held_order.is_empty();
                if suppress_retrigger {
                    // Re-target without re-triggering envelopes so the held note glides smoothly.
                    self.mono_held_order.retain(|&n| n != note);
                    self.mono_held_order.push(note);
                    self.held_notes.clear();
                    self.held_notes.insert(note, 0);
                    self.voices[0].retarget(effective_note, self.master_tune, legato);
                    self.voices[0].note_on_id = self.note_counter;
                    return;
                }
                self.mono_trigger(note, effective_note, velocity_f, legato);
            }
            VoiceMode::Poly => {
                if let Some(&voice_idx) = self.held_notes.get(&note) {
                    self.voices[voice_idx].trigger(
                        effective_note,
                        velocity_f,
                        self.master_tune,
                        false,
                    );
                    self.voices[voice_idx].note_on_id = self.note_counter;
                    return;
                }

                for (i, voice) in self.voices.iter_mut().enumerate() {
                    if !voice.active {
                        voice.trigger(effective_note, velocity_f, self.master_tune, false);
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
                self.voices[oldest_voice].trigger(
                    effective_note,
                    velocity_f,
                    self.master_tune,
                    false,
                );
                self.voices[oldest_voice].note_on_id = self.note_counter;

                self.held_notes.retain(|_, &mut v| v != oldest_voice);
                self.held_notes.insert(note, oldest_voice);
            }
        }
    }

    fn mono_trigger(&mut self, note: u8, effective_note: u8, velocity_f: f32, portamento: bool) {
        // Track ordered list of held notes so note_off can fall back to the previous one.
        self.mono_held_order.retain(|&n| n != note);
        self.mono_held_order.push(note);
        self.held_notes.clear();
        self.held_notes.insert(note, 0);

        self.voices[0].trigger(effective_note, velocity_f, self.master_tune, portamento);
        self.voices[0].note_on_id = self.note_counter;
    }

    fn note_off(&mut self, note: u8) {
        if self.sustain_pedal {
            return;
        }
        match self.voice_mode {
            VoiceMode::Mono | VoiceMode::MonoLegato => {
                self.mono_held_order.retain(|&n| n != note);
                if let Some(&prev) = self.mono_held_order.last() {
                    // Re-target voice 0 to the most recently held note still pressed.
                    // Both Mono and MonoLegato glide here when portamento is on:
                    // there's always at least one prior held note (`prev`).
                    let prev_eff = self.apply_transpose(prev);
                    let portamento = self.portamento_enable;
                    self.voices[0].retarget(prev_eff, self.master_tune, portamento);
                    self.held_notes.clear();
                    self.held_notes.insert(prev, 0);
                } else if let Some(&voice_idx) = self.held_notes.get(&note) {
                    self.voices[voice_idx].release();
                    self.pitch_eg.release();
                    self.held_notes.remove(&note);
                }
            }
            VoiceMode::Poly => {
                if let Some(&voice_idx) = self.held_notes.get(&note) {
                    self.voices[voice_idx].release();
                    self.held_notes.remove(&note);
                    if self.held_notes.is_empty() {
                        self.pitch_eg.release();
                    }
                }
            }
        }
    }

    fn apply_transpose(&self, note: u8) -> u8 {
        let shifted = note as i32 + self.transpose_semitones as i32;
        shifted.clamp(0, 127) as u8
    }

    fn set_operator_param(&mut self, op_index: usize, param: OperatorParam, value: f32) {
        if op_index >= 6 {
            return;
        }
        for voice in &mut self.voices {
            let op = &mut voice.operators[op_index];
            match param {
                OperatorParam::Ratio => op.set_frequency_ratio(value),
                OperatorParam::Level => op.output_level = value,
                OperatorParam::Detune => op.set_detune(value),
                OperatorParam::Feedback => op.feedback = value,
                OperatorParam::VelocitySensitivity => op.velocity_sensitivity = value,
                OperatorParam::KeyScaleRate => op.key_scale_rate = value,
                OperatorParam::KeyScaleBreakpoint => {
                    op.key_scale_breakpoint = value.clamp(0.0, 127.0) as u8
                }
                OperatorParam::KeyScaleLeftDepth => {
                    op.key_scale_left_depth = value.clamp(0.0, 99.0)
                }
                OperatorParam::KeyScaleRightDepth => {
                    op.key_scale_right_depth = value.clamp(0.0, 99.0)
                }
                OperatorParam::KeyScaleLeftCurve => {
                    op.key_scale_left_curve = KeyScaleCurve::from_dx7_code(value as u8)
                }
                OperatorParam::KeyScaleRightCurve => {
                    op.key_scale_right_curve = KeyScaleCurve::from_dx7_code(value as u8)
                }
                OperatorParam::AmSensitivity => {
                    op.am_sensitivity = value.clamp(0.0, 3.0) as u8
                }
                OperatorParam::OscillatorKeySync => op.oscillator_key_sync = value > 0.5,
                OperatorParam::FixedFrequency => {
                    op.fixed_frequency = value > 0.5;
                    op.update_frequency();
                }
                OperatorParam::FixedFreqHz => {
                    op.fixed_freq_hz = value.clamp(0.1, 20000.0);
                    op.update_frequency();
                }
                OperatorParam::Enabled => op.enabled = value > 0.5,
            }
        }
    }

    fn set_pitch_eg_param(&mut self, param: PitchEgParam, value: f32) {
        match param {
            PitchEgParam::Enabled => self.pitch_eg.enabled = value > 0.5,
            PitchEgParam::Rate1 => self.pitch_eg.rate1 = value.clamp(0.0, 99.0),
            PitchEgParam::Rate2 => self.pitch_eg.rate2 = value.clamp(0.0, 99.0),
            PitchEgParam::Rate3 => self.pitch_eg.rate3 = value.clamp(0.0, 99.0),
            PitchEgParam::Rate4 => self.pitch_eg.rate4 = value.clamp(0.0, 99.0),
            PitchEgParam::Level1 => self.pitch_eg.level1 = value.clamp(0.0, 99.0),
            PitchEgParam::Level2 => self.pitch_eg.level2 = value.clamp(0.0, 99.0),
            PitchEgParam::Level3 => self.pitch_eg.level3 = value.clamp(0.0, 99.0),
            PitchEgParam::Level4 => self.pitch_eg.level4 = value.clamp(0.0, 99.0),
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
        self.mono_held_order.clear();
        self.transpose_semitones = 0;
        self.pitch_mod_sensitivity = 0;
        self.pitch_eg.enabled = false;
        self.pitch_eg.reset();

        for voice in &mut self.voices {
            for op in voice.operators.iter_mut() {
                op.frequency_ratio = 1.0;
                op.output_level = 99.0;
                op.detune = 0.0;
                op.feedback = 0.0;
                op.velocity_sensitivity = 0.0;
                op.key_scale_rate = 0.0;
                op.key_scale_breakpoint = 60;
                op.key_scale_left_curve = KeyScaleCurve::default();
                op.key_scale_right_curve = KeyScaleCurve::default();
                op.key_scale_left_depth = 0.0;
                op.key_scale_right_depth = 0.0;
                op.am_sensitivity = 0;
                op.oscillator_key_sync = true;
                op.fixed_frequency = false;
                op.fixed_freq_hz = 440.0;
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

    /// Load a preset by index (for MIDI program change)
    fn load_preset(&mut self, index: usize) {
        if index >= self.presets.len() {
            return;
        }

        // Avoid double-borrow by cloning the preset (cheap: ~6 ops + 6 envs + Option fields).
        let preset = self.presets[index].clone();
        preset.apply_to_synth(self);
        self.current_preset_index = index;
        log::debug!("Loaded preset {}: {}", index, preset.name);
    }

    fn panic(&mut self) {
        for voice in &mut self.voices {
            voice.active = false;
            for op in &mut voice.operators {
                op.reset();
            }
        }
        self.held_notes.clear();
        self.mono_held_order.clear();
        self.pitch_eg.reset();
    }

    /// Process one sample of audio (mono)
    pub fn process(&mut self) -> f32 {
        let mut output = 0.0;
        let mut active_voice_count = 0;

        let (lfo_pitch_mod_raw, lfo_amp_mod) = self.lfo.process(self.mod_wheel);

        // PMS table (DX7 ROM): 0..7 → fractional pitch depth multiplier.
        // PMS=0: no LFO pitch effect; PMS=7: maximum (~1 semitone of swing).
        const PMS_TABLE: [f32; 8] = [0.0, 0.082, 0.16, 0.32, 0.5, 0.79, 1.26, 2.0];
        let pms_scale = PMS_TABLE[self.pitch_mod_sensitivity.min(7) as usize];
        let lfo_pitch_mod = lfo_pitch_mod_raw * pms_scale;

        let pitch_eg_semitones = self.pitch_eg.process();

        for voice in &mut self.voices {
            if voice.active {
                let voice_output = voice.process(
                    self.algorithm,
                    self.pitch_bend,
                    self.pitch_bend_range,
                    self.portamento_time,
                    self.portamento_glissando,
                    lfo_pitch_mod,
                    lfo_amp_mod,
                    pitch_eg_semitones,
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
            voice_mode: self.voice_mode,
            portamento_enable: self.portamento_enable,
            portamento_time: self.portamento_time,
            portamento_glissando: self.portamento_glissando,
            pitch_bend_range: self.pitch_bend_range,
            transpose_semitones: self.transpose_semitones,
            pitch_mod_sensitivity: self.pitch_mod_sensitivity,
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
            pitch_eg: PitchEgSnapshot {
                enabled: self.pitch_eg.enabled,
                rate1: self.pitch_eg.rate1,
                rate2: self.pitch_eg.rate2,
                rate3: self.pitch_eg.rate3,
                rate4: self.pitch_eg.rate4,
                level1: self.pitch_eg.level1,
                level2: self.pitch_eg.level2,
                level3: self.pitch_eg.level3,
                level4: self.pitch_eg.level4,
            },
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
                    key_scale_rate: op.key_scale_rate,
                    key_scale_breakpoint: op.key_scale_breakpoint,
                    key_scale_left_curve: op.key_scale_left_curve,
                    key_scale_right_curve: op.key_scale_right_curve,
                    key_scale_left_depth: op.key_scale_left_depth,
                    key_scale_right_depth: op.key_scale_right_depth,
                    am_sensitivity: op.am_sensitivity,
                    oscillator_key_sync: op.oscillator_key_sync,
                    fixed_frequency: op.fixed_frequency,
                    fixed_freq_hz: op.fixed_freq_hz,
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

    pub fn set_transpose_semitones(&mut self, st: i8) {
        self.transpose_semitones = st.clamp(-24, 24);
    }

    pub fn set_pitch_mod_sensitivity(&mut self, pms: u8) {
        self.pitch_mod_sensitivity = pms.min(7);
    }

    pub fn set_pitch_bend_range(&mut self, range: f32) {
        self.pitch_bend_range = range.clamp(0.0, 12.0);
    }

    pub fn pitch_eg_mut(&mut self) -> &mut PitchEg {
        &mut self.pitch_eg
    }

    pub fn set_presets(&mut self, presets: Vec<Dx7Preset>) {
        self.current_preset_index = 0;
        self.presets = presets;
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
    pub fn get_voice_mode(&self) -> VoiceMode {
        self.voice_mode
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

    pub fn set_voice_mode(&mut self, mode: VoiceMode) {
        let code = match mode {
            VoiceMode::Poly => 0,
            VoiceMode::Mono => 1,
            VoiceMode::MonoLegato => 2,
        };
        self.send(SynthCommand::SetVoiceMode(code));
    }

    pub fn set_portamento_glissando(&mut self, on: bool) {
        self.send(SynthCommand::SetPortamentoGlissando(on));
    }

    #[allow(dead_code)]
    pub fn set_transpose(&mut self, semitones: i8) {
        self.send(SynthCommand::SetTranspose(semitones));
    }

    #[allow(dead_code)]
    pub fn set_pitch_mod_sensitivity(&mut self, pms: u8) {
        self.send(SynthCommand::SetPitchModSensitivity(pms));
    }

    #[allow(dead_code)]
    pub fn set_pitch_eg_param(&mut self, param: PitchEgParam, value: f32) {
        self.send(SynthCommand::SetPitchEgParam { param, value });
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

    /// Load a preset by index (for MIDI program change 0xC0)
    pub fn load_preset(&mut self, index: usize) {
        self.send(SynthCommand::LoadPreset(index));
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
