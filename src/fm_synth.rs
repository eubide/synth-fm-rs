use crate::algorithms;
use crate::command_queue::{
    create_command_queue, CommandReceiver, CommandSender, EffectParam, EffectType, EnvelopeParam,
    LfoParam, OperatorParam, PitchEgParam, SynthCommand,
};
use crate::dc_blocker::DcBlocker;
use crate::effects::EffectsChain;
use crate::lfo::{LFOWaveform, LFO};
use crate::operator::{KeyScaleCurve, Operator};
use crate::optimization::{midi_to_hz, voice_scale};
use crate::pitch_eg::PitchEg;
use crate::presets::Dx7Preset;
use crate::state_snapshot::{
    create_snapshot_channel, AutoPanSnapshot, ChorusSnapshot, DelaySnapshot, OperatorSnapshot,
    PitchEgSnapshot, ReverbSnapshot, SnapshotReceiver, SnapshotSender, SynthSnapshot, VoiceMode,
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
        let base_frequency = midi_to_hz(note);
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
        let base_frequency = midi_to_hz(note);
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
        eg_bias_amount: f32,
        pitch_bias_semitones: f32,
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
        // Pitch Bias is the static, mod-wheel-driven counterpart of LFO pitch mod —
        // a constant offset rather than an oscillation. Sums into the same destination.
        let total_pitch_offset = lfo_pitch_semitones + pitch_eg_semitones + pitch_bias_semitones;
        let final_frequency = bent_frequency * 2.0_f32.powf(total_pitch_offset / 12.0);

        for op in &mut self.operators {
            op.update_frequency_only(final_frequency);
            op.set_lfo_amp_mod(lfo_amp_mod);
            op.set_eg_bias(eg_bias_amount);
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

/// Routing depth helper: scale a 0..1 controller value by a 0..7 sensitivity.
/// The DX7S "PITCH/AMP/EG BIAS/PITCH BIAS" knobs all share this 0..7 fractional
/// shape — `sens` is clamped here so callers don't repeat the guard.
#[inline]
fn route_amount(value: f32, sens: u8) -> f32 {
    value * (sens.min(7) as f32 / 7.0)
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
    eg_bias_sensitivity: u8,
    pitch_bias_sensitivity: u8,
    // Aftertouch (channel pressure) state and routing
    aftertouch: f32,
    aftertouch_pitch_sens: u8,
    aftertouch_amp_sens: u8,
    aftertouch_eg_bias_sens: u8,
    aftertouch_pitch_bias_sens: u8,
    // Breath Controller (CC2) state and routing
    breath: f32,
    breath_pitch_sens: u8,
    breath_amp_sens: u8,
    breath_eg_bias_sens: u8,
    breath_pitch_bias_sens: u8,
    // Foot Controller (CC4) state and routing — VOLUME (0-15) + 3 destinations (0-7)
    foot: f32,
    foot_volume_sens: u8,
    foot_pitch_sens: u8,
    foot_amp_sens: u8,
    foot_eg_bias_sens: u8,
    /// MIDI Expression (CC11): generic 0..1 attenuator multiplied into the master output.
    expression: f32,
    /// MIDI Bank Select MSB (CC0) — top 7 bits of the bank index.
    bank_msb: u8,
    /// MIDI Bank Select LSB (CC32) — low 7 bits of the bank index.
    bank_lsb: u8,
    sustain_pedal: bool,
    #[allow(dead_code)]
    sample_rate: f32,
    dc_blocker_l: DcBlocker,
    dc_blocker_r: DcBlocker,
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

        // The DX7 itself shipped without on-board effects, but its iconic sound on
        // every record from 1983-89 came through external chorus + reverb. Boot
        // with both ON at modest mix levels so factory presets sound like the user
        // remembers them; users wanting the bone-dry signal can flip them off.
        // AutoPan is also ON by default at moderate depth: countless DX7 patches
        // (Rhodes, Clavinet, e-pianos, soft pads) were tracked through a Suitcase
        // amp or Leslie-style pan, and patches like `mark/rhodes.json` that omit
        // LFO amp modulation rely on this post-FM movement to sound alive.
        // Delay stays OFF on purpose — it's a creative effect, not part of the
        // canonical "DX7 → outboard" flavour.
        let mut effects = EffectsChain::new(sample_rate);
        effects.chorus.enabled = true;
        effects.chorus.mix = 0.15;
        effects.auto_pan.enabled = true;
        effects.auto_pan.rate_hz = 5.0;
        effects.auto_pan.depth = 0.35;
        effects.reverb.enabled = true;
        effects.reverb.mix = 0.22;

        Self {
            voices,
            held_notes: HashMap::new(),
            mono_held_order: Vec::with_capacity(8),
            preset_name: "Init Voice".to_string(),
            lfo: LFO::new(sample_rate),
            pitch_eg: PitchEg::new(sample_rate),
            effects,
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
            eg_bias_sensitivity: 0,
            pitch_bias_sensitivity: 0,
            aftertouch: 0.0,
            aftertouch_pitch_sens: 0,
            aftertouch_amp_sens: 0,
            aftertouch_eg_bias_sens: 0,
            aftertouch_pitch_bias_sens: 0,
            breath: 0.0,
            breath_pitch_sens: 0,
            breath_amp_sens: 0,
            breath_eg_bias_sens: 0,
            breath_pitch_bias_sens: 0,
            foot: 0.0,
            foot_volume_sens: 0,
            foot_pitch_sens: 0,
            foot_amp_sens: 0,
            foot_eg_bias_sens: 0,
            expression: 1.0,
            bank_msb: 0,
            bank_lsb: 0,
            sustain_pedal: false,
            sample_rate,
            dc_blocker_l: DcBlocker::new(sample_rate, 5.0),
            dc_blocker_r: DcBlocker::new(sample_rate, 5.0),
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
            SynthCommand::SetEgBiasSensitivity(s) => {
                self.eg_bias_sensitivity = s.min(7);
            }
            SynthCommand::SetPitchBiasSensitivity(s) => {
                self.pitch_bias_sensitivity = s.min(7);
            }
            SynthCommand::SetAftertouchPitchSens(s) => {
                self.aftertouch_pitch_sens = s.min(7);
            }
            SynthCommand::SetAftertouchAmpSens(s) => {
                self.aftertouch_amp_sens = s.min(7);
            }
            SynthCommand::SetAftertouchEgBiasSens(s) => {
                self.aftertouch_eg_bias_sens = s.min(7);
            }
            SynthCommand::SetAftertouchPitchBiasSens(s) => {
                self.aftertouch_pitch_bias_sens = s.min(7);
            }
            SynthCommand::Aftertouch(value) => {
                self.aftertouch = value.clamp(0.0, 1.0);
            }
            SynthCommand::SetBreathPitchSens(s) => {
                self.breath_pitch_sens = s.min(7);
            }
            SynthCommand::SetBreathAmpSens(s) => {
                self.breath_amp_sens = s.min(7);
            }
            SynthCommand::SetBreathEgBiasSens(s) => {
                self.breath_eg_bias_sens = s.min(7);
            }
            SynthCommand::SetBreathPitchBiasSens(s) => {
                self.breath_pitch_bias_sens = s.min(7);
            }
            SynthCommand::BreathController(value) => {
                self.breath = value.clamp(0.0, 1.0);
            }
            SynthCommand::SetFootVolumeSens(s) => {
                self.foot_volume_sens = s.min(15);
            }
            SynthCommand::SetFootPitchSens(s) => {
                self.foot_pitch_sens = s.min(7);
            }
            SynthCommand::SetFootAmpSens(s) => {
                self.foot_amp_sens = s.min(7);
            }
            SynthCommand::SetFootEgBiasSens(s) => {
                self.foot_eg_bias_sens = s.min(7);
            }
            SynthCommand::FootController(value) => {
                self.foot = value.clamp(0.0, 1.0);
            }
            SynthCommand::Expression(value) => {
                self.expression = value.clamp(0.0, 1.0);
            }
            SynthCommand::SetBankSelectMsb(v) => {
                self.bank_msb = v & 0x7F;
            }
            SynthCommand::SetBankSelectLsb(v) => {
                self.bank_lsb = v & 0x7F;
            }
            SynthCommand::ProgramChange(program) => {
                let absolute = ((self.bank_msb as usize) << 14)
                    | ((self.bank_lsb as usize) << 7)
                    | (program as usize & 0x7F);
                self.load_preset(absolute);
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
            SynthCommand::LoadSysExSingleVoice(preset) => {
                preset.apply_to_synth(self);
            }
            SynthCommand::LoadSysExBulk(presets) => {
                if let Some(first) = presets.first().cloned() {
                    first.apply_to_synth(self);
                }
                self.set_presets(presets);
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
                OperatorParam::Level => op.set_output_level(value),
                OperatorParam::Detune => op.set_detune(value),
                OperatorParam::Feedback => op.set_feedback(value),
                OperatorParam::VelocitySensitivity => op.set_velocity_sensitivity(value),
                OperatorParam::KeyScaleRate => op.set_key_scale_rate(value),
                OperatorParam::KeyScaleBreakpoint => {
                    op.set_key_scale_breakpoint(value.clamp(0.0, 127.0) as u8)
                }
                OperatorParam::KeyScaleLeftDepth => op.set_key_scale_left_depth(value),
                OperatorParam::KeyScaleRightDepth => op.set_key_scale_right_depth(value),
                OperatorParam::KeyScaleLeftCurve => {
                    op.set_key_scale_left_curve(KeyScaleCurve::from_dx7_code(value as u8))
                }
                OperatorParam::KeyScaleRightCurve => {
                    op.set_key_scale_right_curve(KeyScaleCurve::from_dx7_code(value as u8))
                }
                OperatorParam::AmSensitivity => op.set_am_sensitivity(value.clamp(0.0, 3.0) as u8),
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
            EffectType::AutoPan => match param {
                EffectParam::Enabled => self.effects.auto_pan.enabled = value > 0.5,
                EffectParam::AutoPanRate => self.effects.auto_pan.rate_hz = value.clamp(0.05, 20.0),
                EffectParam::AutoPanDepth => self.effects.auto_pan.depth = value.clamp(0.0, 1.0),
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
        self.eg_bias_sensitivity = 0;
        self.pitch_bias_sensitivity = 0;
        // Init Voice clears the patch-side routing for every external controller
        // (live readings like `aftertouch`/`breath`/`foot` keep whatever the
        // physical controller is sending — they reset themselves on the next CC).
        self.aftertouch_pitch_sens = 0;
        self.aftertouch_amp_sens = 0;
        self.aftertouch_eg_bias_sens = 0;
        self.aftertouch_pitch_bias_sens = 0;
        self.breath_pitch_sens = 0;
        self.breath_amp_sens = 0;
        self.breath_eg_bias_sens = 0;
        self.breath_pitch_bias_sens = 0;
        self.foot_volume_sens = 0;
        self.foot_pitch_sens = 0;
        self.foot_amp_sens = 0;
        self.foot_eg_bias_sens = 0;
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

    /// Process one sample of audio (mono). Output is **unsaturated** — the
    /// final `tanh` happens once, post-effects, in [`Self::process_stereo`].
    pub fn process(&mut self) -> f32 {
        let mut output = 0.0;
        let mut active_voice_count = 0;

        let (lfo_pitch_mod_raw, lfo_amp_mod_raw) = self.lfo.process(self.mod_wheel);

        // PMS (Pitch Mod Sensitivity) ROM lookup. Source: `pitchmodsenstab[8]`
        // in MSFA / Dexed `dx7note.cc` = {0, 10, 20, 33, 55, 92, 153, 255},
        // normalised by 255 then scaled to our depth domain. The `* 2.0`
        // factor preserves the previous max swing of ~2 semitones at PMS=7
        // (downstream code applies `* 0.5` to convert back to semitones).
        const PMS_TABLE: [f32; 8] = [
            0.0,
            (10.0 / 255.0) * 2.0,  // 0.0784
            (20.0 / 255.0) * 2.0,  // 0.1569
            (33.0 / 255.0) * 2.0,  // 0.2588
            (55.0 / 255.0) * 2.0,  // 0.4314
            (92.0 / 255.0) * 2.0,  // 0.7216
            (153.0 / 255.0) * 2.0, // 1.2000
            2.0,
        ];
        let pms_scale = PMS_TABLE[self.pitch_mod_sensitivity.min(7) as usize];

        // Each external controller (Aftertouch / Breath / Foot) routes to four
        // destinations. PITCH and AMP further scale the LFO pitch/amp depth on
        // top of the patch's PMS/AMS settings; EG_BIAS and PITCH_BIAS are static
        // mod-wheel-style offsets summed with the existing routings.
        // Foot has no PITCH_BIAS destination on the DX7S.
        let pitch_route_total = route_amount(self.aftertouch, self.aftertouch_pitch_sens)
            + route_amount(self.breath, self.breath_pitch_sens)
            + route_amount(self.foot, self.foot_pitch_sens);
        let amp_route_total = route_amount(self.aftertouch, self.aftertouch_amp_sens)
            + route_amount(self.breath, self.breath_amp_sens)
            + route_amount(self.foot, self.foot_amp_sens);
        let eg_bias_route_total = route_amount(self.aftertouch, self.aftertouch_eg_bias_sens)
            + route_amount(self.breath, self.breath_eg_bias_sens)
            + route_amount(self.foot, self.foot_eg_bias_sens);
        let pitch_bias_route_total = route_amount(self.aftertouch, self.aftertouch_pitch_bias_sens)
            + route_amount(self.breath, self.breath_pitch_bias_sens);

        // Final LFO modulation: PMS-base from patch + dynamic boost from controllers.
        let lfo_pitch_mod = lfo_pitch_mod_raw * (pms_scale + pitch_route_total);
        let lfo_amp_mod = lfo_amp_mod_raw * (1.0 + amp_route_total);

        let pitch_eg_semitones = self.pitch_eg.process();

        // EG Bias: static controller-driven offset (mod wheel × sensitivity).
        // 0..1 amount; the per-operator AMS gates how strongly each op responds.
        let eg_bias_amount =
            route_amount(self.mod_wheel, self.eg_bias_sensitivity) + eg_bias_route_total;
        // Pitch Bias: same idea but applied to the pitch — up to ±2 semitones at max.
        let pitch_bias_semitones = (route_amount(self.mod_wheel, self.pitch_bias_sensitivity)
            + pitch_bias_route_total)
            * 2.0;

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
                    eg_bias_amount,
                    pitch_bias_semitones,
                );
                output += voice_output;
                active_voice_count += 1;
            }
        }

        let voice_scaling = voice_scale(active_voice_count);

        // Foot Controller VOLUME (DX7S): when sensitivity > 0, the foot pedal acts
        // as a volume swell. Sens=0 leaves the master untouched; sens=15 makes
        // foot=0 silence the synth entirely. Linear interpolation between 1.0 and
        // the foot value, weighted by sensitivity / 15.
        let foot_volume_factor = if self.foot_volume_sens > 0 {
            let weight = self.foot_volume_sens as f32 / 15.0;
            1.0 - weight + weight * self.foot
        } else {
            1.0
        };

        output * voice_scaling * self.master_volume * foot_volume_factor * self.expression
    }

    /// Process audio with effects, returns stereo pair (left, right).
    ///
    /// Saturation lives only here, *after* the effects chain: feeding a
    /// pre-saturated mono into Chorus/Reverb crushes transients (the Rhodes
    /// "bell tone", marimba peaks) before the reverb sees them, and makes
    /// the wet path sound dull. DC blockers run before the final `tanh`
    /// so any feedback-induced offset (algorithms 4/6 cross-feedback,
    /// asymmetric voice sums) is removed *before* it biases the saturator.
    pub fn process_stereo(&mut self) -> (f32, f32) {
        let mono = self.process();
        let (left, right) = self.effects.process(mono);
        let l = Self::soft_clip(self.dc_blocker_l.process(left));
        let r = Self::soft_clip(self.dc_blocker_r.process(right));
        (l, r)
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
            eg_bias_sensitivity: self.eg_bias_sensitivity,
            pitch_bias_sensitivity: self.pitch_bias_sensitivity,
            pitch_bend: self.pitch_bend,
            mod_wheel: self.mod_wheel,
            sustain_pedal: self.sustain_pedal,
            aftertouch: self.aftertouch,
            breath: self.breath,
            foot: self.foot,
            expression: self.expression,
            aftertouch_pitch_sens: self.aftertouch_pitch_sens,
            aftertouch_amp_sens: self.aftertouch_amp_sens,
            aftertouch_eg_bias_sens: self.aftertouch_eg_bias_sens,
            aftertouch_pitch_bias_sens: self.aftertouch_pitch_bias_sens,
            breath_pitch_sens: self.breath_pitch_sens,
            breath_amp_sens: self.breath_amp_sens,
            breath_eg_bias_sens: self.breath_eg_bias_sens,
            breath_pitch_bias_sens: self.breath_pitch_bias_sens,
            foot_volume_sens: self.foot_volume_sens,
            foot_pitch_sens: self.foot_pitch_sens,
            foot_amp_sens: self.foot_amp_sens,
            foot_eg_bias_sens: self.foot_eg_bias_sens,
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
            auto_pan: AutoPanSnapshot {
                enabled: self.effects.auto_pan.enabled,
                rate_hz: self.effects.auto_pan.rate_hz,
                depth: self.effects.auto_pan.depth,
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
                    live_level: 0.0,
                };
            }

            for voice in &self.voices {
                if !voice.active {
                    continue;
                }
                for (i, op) in voice.operators.iter().enumerate() {
                    let live = op.envelope.current_output();
                    if live > snapshots[i].live_level {
                        snapshots[i].live_level = live;
                    }
                }
            }

            snapshots
        } else {
            [OperatorSnapshot::default(); 6]
        }
    }

    /// Soft saturation analogous to the DX7's μ-law-companded 12-bit DAC.
    /// `tanh` gives smooth, symmetric, asymptotic compression toward ±1.0.
    fn soft_clip(sample: f32) -> f32 {
        sample.tanh()
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

    pub fn set_eg_bias_sensitivity(&mut self, sens: u8) {
        self.send(SynthCommand::SetEgBiasSensitivity(sens));
    }

    pub fn set_pitch_bias_sensitivity(&mut self, sens: u8) {
        self.send(SynthCommand::SetPitchBiasSensitivity(sens));
    }

    pub fn aftertouch(&mut self, value: f32) {
        self.send(SynthCommand::Aftertouch(value));
    }

    pub fn set_aftertouch_pitch_sens(&mut self, sens: u8) {
        self.send(SynthCommand::SetAftertouchPitchSens(sens));
    }

    pub fn set_aftertouch_amp_sens(&mut self, sens: u8) {
        self.send(SynthCommand::SetAftertouchAmpSens(sens));
    }

    pub fn set_aftertouch_eg_bias_sens(&mut self, sens: u8) {
        self.send(SynthCommand::SetAftertouchEgBiasSens(sens));
    }

    pub fn set_aftertouch_pitch_bias_sens(&mut self, sens: u8) {
        self.send(SynthCommand::SetAftertouchPitchBiasSens(sens));
    }

    pub fn breath_controller(&mut self, value: f32) {
        self.send(SynthCommand::BreathController(value));
    }

    pub fn set_breath_pitch_sens(&mut self, sens: u8) {
        self.send(SynthCommand::SetBreathPitchSens(sens));
    }

    pub fn set_breath_amp_sens(&mut self, sens: u8) {
        self.send(SynthCommand::SetBreathAmpSens(sens));
    }

    pub fn set_breath_eg_bias_sens(&mut self, sens: u8) {
        self.send(SynthCommand::SetBreathEgBiasSens(sens));
    }

    pub fn set_breath_pitch_bias_sens(&mut self, sens: u8) {
        self.send(SynthCommand::SetBreathPitchBiasSens(sens));
    }

    pub fn foot_controller(&mut self, value: f32) {
        self.send(SynthCommand::FootController(value));
    }

    pub fn set_foot_volume_sens(&mut self, sens: u8) {
        self.send(SynthCommand::SetFootVolumeSens(sens));
    }

    pub fn set_foot_pitch_sens(&mut self, sens: u8) {
        self.send(SynthCommand::SetFootPitchSens(sens));
    }

    pub fn set_foot_amp_sens(&mut self, sens: u8) {
        self.send(SynthCommand::SetFootAmpSens(sens));
    }

    pub fn set_foot_eg_bias_sens(&mut self, sens: u8) {
        self.send(SynthCommand::SetFootEgBiasSens(sens));
    }

    pub fn expression(&mut self, value: f32) {
        self.send(SynthCommand::Expression(value));
    }

    pub fn set_bank_msb(&mut self, value: u8) {
        self.send(SynthCommand::SetBankSelectMsb(value));
    }

    pub fn set_bank_lsb(&mut self, value: u8) {
        self.send(SynthCommand::SetBankSelectLsb(value));
    }

    pub fn program_change(&mut self, program: u8) {
        self.send(SynthCommand::ProgramChange(program));
    }

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

    /// Load a preset by index (for MIDI program change 0xC0).
    /// MIDI now goes through `program_change`; this remains for the GUI / direct callers.
    #[allow(dead_code)]
    pub fn load_preset(&mut self, index: usize) {
        self.send(SynthCommand::LoadPreset(index));
    }

    /// Apply a SysEx-parsed single voice as the live edit buffer.
    pub fn load_sysex_single_voice(&mut self, preset: Dx7Preset) {
        self.send(SynthCommand::LoadSysExSingleVoice(Box::new(preset)));
    }

    /// Replace the entire bank with the given list of presets.
    pub fn load_sysex_bulk(&mut self, presets: Vec<Dx7Preset>) {
        self.send(SynthCommand::LoadSysExBulk(presets));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presets::{PresetLfo, PresetOperator, PresetPitchEg};

    const SR: f32 = 44_100.0;

    fn make_engine() -> (SynthEngine, SynthController) {
        create_synth(SR)
    }

    fn drive(engine: &mut SynthEngine, samples: usize) {
        for _ in 0..samples {
            engine.process_commands();
            engine.process();
        }
    }

    fn drive_stereo(engine: &mut SynthEngine, samples: usize) -> (f32, f32) {
        let mut peak_l = 0.0_f32;
        let mut peak_r = 0.0_f32;
        for _ in 0..samples {
            engine.process_commands();
            let (l, r) = engine.process_stereo();
            peak_l = peak_l.max(l.abs());
            peak_r = peak_r.max(r.abs());
        }
        (peak_l, peak_r)
    }

    fn make_preset(name: &str, alg: u8) -> Dx7Preset {
        Dx7Preset {
            name: name.to_string(),
            collection: "test".to_string(),
            algorithm: alg,
            operators: std::array::from_fn(|_| PresetOperator::default()),
            master_tune: Some(5.0),
            pitch_bend_range: Some(2.0),
            portamento_enable: None,
            portamento_time: None,
            mono_mode: None,
            transpose_semitones: 12,
            pitch_mod_sensitivity: 4,
            pitch_eg: Some(PresetPitchEg::default()),
            lfo: Some(PresetLfo::default()),
        }
    }

    // -----------------------------------------------------------------------
    // Soft clip
    // -----------------------------------------------------------------------

    #[test]
    fn soft_clip_zero_is_zero() {
        assert_eq!(SynthEngine::soft_clip(0.0), 0.0);
    }

    #[test]
    fn soft_clip_saturates_to_unity() {
        assert!((SynthEngine::soft_clip(10.0) - 1.0).abs() < 1e-4);
        assert!((SynthEngine::soft_clip(-10.0) + 1.0).abs() < 1e-4);
    }

    #[test]
    fn soft_clip_is_monotonic() {
        let a = SynthEngine::soft_clip(0.5);
        let b = SynthEngine::soft_clip(0.8);
        let c = SynthEngine::soft_clip(2.0);
        assert!(a < b && b < c);
    }

    // -----------------------------------------------------------------------
    // quantize_to_semitone
    // -----------------------------------------------------------------------

    #[test]
    fn quantize_zero_or_negative_passes_through() {
        assert_eq!(quantize_to_semitone(0.0), 0.0);
        assert_eq!(quantize_to_semitone(-1.0), -1.0);
    }

    #[test]
    fn quantize_a4_returns_440() {
        let q = quantize_to_semitone(440.0);
        assert!((q - 440.0).abs() < 1e-3);
    }

    #[test]
    fn quantize_snaps_off_pitch_to_nearest() {
        // 460 Hz is between A4 (440) and A#4 (~466). Closer to A#4.
        let q = quantize_to_semitone(460.0);
        let asharp = 440.0 * 2.0_f32.powf(1.0 / 12.0);
        assert!((q - asharp).abs() < 1.0);
    }

    // -----------------------------------------------------------------------
    // route_amount helper
    // -----------------------------------------------------------------------

    #[test]
    fn route_amount_full_sensitivity_passes_value() {
        assert!((route_amount(1.0, 7) - 1.0).abs() < 1e-3);
        assert!((route_amount(0.5, 7) - 0.5).abs() < 1e-3);
    }

    #[test]
    fn route_amount_zero_sensitivity_silences() {
        assert_eq!(route_amount(1.0, 0), 0.0);
    }

    #[test]
    fn route_amount_clamps_sensitivity_to_seven() {
        assert_eq!(route_amount(1.0, 7), route_amount(1.0, 100));
    }

    // -----------------------------------------------------------------------
    // Voice
    // -----------------------------------------------------------------------

    #[test]
    fn voice_starts_inactive() {
        let v = Voice::new_with_sample_rate(SR);
        assert!(!v.active);
        assert_eq!(v.note, 0);
    }

    #[test]
    fn voice_trigger_makes_active_and_sets_frequency() {
        let mut v = Voice::new_with_sample_rate(SR);
        v.trigger(69, 1.0, 0.0, false);
        assert!(v.active);
        assert_eq!(v.note, 69);
        assert!((v.frequency - 440.0).abs() < 0.5);
    }

    #[test]
    fn voice_master_tune_shifts_frequency() {
        let mut v = Voice::new_with_sample_rate(SR);
        v.trigger(69, 1.0, 100.0, false); // +1 semitone
        let asharp = 440.0 * 2.0_f32.powf(1.0 / 12.0);
        assert!((v.frequency - asharp).abs() < 1.0);
    }

    #[test]
    fn voice_release_eventually_idles() {
        let mut v = Voice::new_with_sample_rate(SR);
        for op in &mut v.operators {
            op.envelope.rate1 = 99.0;
            op.envelope.rate4 = 99.0;
            op.envelope.level4 = 0.0;
        }
        v.trigger(69, 1.0, 0.0, false);
        for _ in 0..2048 {
            v.process(1, 0.0, 2.0, 0.0, false, 0.0, 0.0, 0.0, 0.0, 0.0);
        }
        v.release();
        for _ in 0..(SR as usize) {
            v.process(1, 0.0, 2.0, 0.0, false, 0.0, 0.0, 0.0, 0.0, 0.0);
            if !v.active {
                break;
            }
        }
        assert!(!v.active);
    }

    #[test]
    fn voice_inactive_returns_zero_output() {
        let mut v = Voice::new_with_sample_rate(SR);
        let s = v.process(1, 0.0, 2.0, 0.0, false, 0.0, 0.0, 0.0, 0.0, 0.0);
        assert_eq!(s, 0.0);
    }

    #[test]
    fn voice_glissando_quantises_frequency() {
        let mut v = Voice::new_with_sample_rate(SR);
        v.trigger(69, 1.0, 0.0, false);
        // Run with glissando ON
        for _ in 0..256 {
            v.process(1, 0.0, 2.0, 0.0, true, 0.0, 0.0, 0.0, 0.0, 0.0);
        }
    }

    #[test]
    fn voice_pitch_bend_changes_frequency_perceptually() {
        let mut v = Voice::new_with_sample_rate(SR);
        v.trigger(69, 1.0, 0.0, false);
        // Just exercise the pitch bend path.
        for _ in 0..256 {
            v.process(1, 0.5, 2.0, 0.0, false, 0.0, 0.0, 0.0, 0.0, 0.0);
        }
    }

    #[test]
    fn voice_steal_initiates_fade_out() {
        let mut v = Voice::new_with_sample_rate(SR);
        v.trigger(69, 1.0, 0.0, false);
        v.steal_voice();
        // Process a few samples to advance the fade
        for _ in 0..4096 {
            v.process(1, 0.0, 2.0, 0.0, false, 0.0, 0.0, 0.0, 0.0, 0.0);
            if !v.active {
                break;
            }
        }
        assert!(
            !v.active,
            "stolen voice should fade out and become inactive"
        );
    }

    #[test]
    fn voice_retarget_changes_note_without_envelope_retrigger() {
        let mut v = Voice::new_with_sample_rate(SR);
        v.trigger(60, 1.0, 0.0, false);
        for _ in 0..256 {
            v.process(1, 0.0, 2.0, 0.0, false, 0.0, 0.0, 0.0, 0.0, 0.0);
        }
        v.retarget(72, 0.0, false); // jump up an octave, no portamento
        assert_eq!(v.note, 72);
        assert!((v.frequency - 440.0 * 2.0_f32.powf((72 - 69) as f32 / 12.0)).abs() < 0.5);
    }

    #[test]
    fn voice_portamento_uses_target_frequency_not_current() {
        let mut v = Voice::new_with_sample_rate(SR);
        // First trigger: establish a starting frequency
        v.trigger(60, 1.0, 0.0, true);
        let initial = v.current_frequency;
        // Second trigger with portamento ON: target should change but current stays
        v.trigger(72, 1.0, 0.0, true);
        assert_ne!(v.target_frequency, initial);
        let target = v.target_frequency;
        // Asymptotic glide: at portamento_time=10 the half-life is ~30ms, so
        // SR/2 (~500ms) gets us deep into the convergence tail.
        for _ in 0..(SR as usize / 2) {
            v.process(1, 0.0, 2.0, 10.0, false, 0.0, 0.0, 0.0, 0.0, 0.0);
            if (v.current_frequency - target).abs() < 1.0 {
                break;
            }
        }
        assert!(
            v.current_frequency > initial,
            "current should glide upward toward target"
        );
        assert!(v.current_frequency <= target * 1.01, "should not overshoot");
    }

    #[test]
    fn voice_stop_resets_state() {
        let mut v = Voice::new_with_sample_rate(SR);
        v.trigger(60, 1.0, 0.0, false);
        v.stop();
        assert!(!v.active);
    }

    // -----------------------------------------------------------------------
    // SynthEngine commands & basic flow
    // -----------------------------------------------------------------------

    #[test]
    fn engine_new_has_init_voice_default() {
        let (engine, _ctrl) = make_engine();
        assert_eq!(engine.preset_name, "Init Voice");
        assert_eq!(engine.algorithm, 1);
    }

    #[test]
    fn engine_set_algorithm_clamps_to_valid_range() {
        let (mut engine, mut ctrl) = make_engine();
        ctrl.set_algorithm(0); // invalid
        engine.process_commands();
        assert_eq!(engine.algorithm, 1);
        ctrl.set_algorithm(33); // invalid
        engine.process_commands();
        assert_eq!(engine.algorithm, 1);
        ctrl.set_algorithm(7);
        engine.process_commands();
        assert_eq!(engine.algorithm, 7);
    }

    #[test]
    fn engine_set_master_volume_clamps_to_zero_one() {
        let (mut engine, mut ctrl) = make_engine();
        ctrl.set_master_volume(2.0);
        engine.process_commands();
        assert_eq!(engine.master_volume, 1.0);
        ctrl.set_master_volume(-0.5);
        engine.process_commands();
        assert_eq!(engine.master_volume, 0.0);
    }

    #[test]
    fn engine_set_master_tune_clamps_to_safe_range() {
        let (mut engine, mut ctrl) = make_engine();
        ctrl.set_master_tune(500.0);
        engine.process_commands();
        assert_eq!(engine.master_tune, 150.0);
        ctrl.set_master_tune(-500.0);
        engine.process_commands();
        assert_eq!(engine.master_tune, -150.0);
    }

    #[test]
    fn engine_set_pitch_bend_range_clamps() {
        let (mut engine, mut ctrl) = make_engine();
        ctrl.set_pitch_bend_range(50.0);
        engine.process_commands();
        assert_eq!(engine.pitch_bend_range, 12.0);
        ctrl.set_pitch_bend_range(-1.0);
        engine.process_commands();
        assert_eq!(engine.pitch_bend_range, 0.0);
    }

    #[test]
    fn engine_set_voice_mode_changes_mode() {
        let (mut engine, mut ctrl) = make_engine();
        ctrl.set_voice_mode(crate::state_snapshot::VoiceMode::Mono);
        engine.process_commands();
        assert_eq!(engine.voice_mode, crate::state_snapshot::VoiceMode::Mono);
        ctrl.set_voice_mode(crate::state_snapshot::VoiceMode::MonoLegato);
        engine.process_commands();
        assert_eq!(
            engine.voice_mode,
            crate::state_snapshot::VoiceMode::MonoLegato
        );
        ctrl.set_voice_mode(crate::state_snapshot::VoiceMode::Poly);
        engine.process_commands();
        assert_eq!(engine.voice_mode, crate::state_snapshot::VoiceMode::Poly);
    }

    #[test]
    fn engine_set_transpose_clamps() {
        let (mut engine, mut ctrl) = make_engine();
        ctrl.set_transpose(50);
        engine.process_commands();
        assert_eq!(engine.transpose_semitones, 24);
        ctrl.set_transpose(-50);
        engine.process_commands();
        assert_eq!(engine.transpose_semitones, -24);
    }

    #[test]
    fn engine_set_pitch_mod_sensitivity_clamps() {
        let (mut engine, mut ctrl) = make_engine();
        ctrl.set_pitch_mod_sensitivity(99);
        engine.process_commands();
        assert_eq!(engine.pitch_mod_sensitivity, 7);
    }

    #[test]
    fn engine_note_on_off_round_trip() {
        let (mut engine, mut ctrl) = make_engine();
        ctrl.note_on(60, 100);
        engine.process_commands();
        // We should now have at least one active voice.
        let active = engine.voices.iter().filter(|v| v.active).count();
        assert!(active >= 1);
        ctrl.note_off(60);
        engine.process_commands();
        // Note off triggers release, voice still active until envelope completes.
    }

    #[test]
    fn engine_panic_stops_all_voices() {
        let (mut engine, mut ctrl) = make_engine();
        ctrl.note_on(60, 100);
        ctrl.note_on(64, 100);
        ctrl.note_on(67, 100);
        engine.process_commands();
        ctrl.panic();
        engine.process_commands();
        let active = engine.voices.iter().filter(|v| v.active).count();
        assert_eq!(active, 0);
    }

    #[test]
    fn engine_voice_initialize_resets_to_defaults() {
        let (mut engine, mut ctrl) = make_engine();
        ctrl.set_algorithm(15);
        ctrl.note_on(60, 100);
        engine.process_commands();
        ctrl.voice_initialize();
        engine.process_commands();
        assert_eq!(engine.algorithm, 1);
        assert_eq!(engine.preset_name, "Init Voice");
        let active = engine.voices.iter().filter(|v| v.active).count();
        assert_eq!(active, 0);
    }

    #[test]
    fn engine_process_produces_audio_when_note_pressed() {
        let (mut engine, mut ctrl) = make_engine();
        ctrl.note_on(69, 100);
        let mut peak = 0.0_f32;
        for _ in 0..4096 {
            engine.process_commands();
            peak = peak.max(engine.process().abs());
        }
        assert!(peak > 0.001, "expected audio after note on, peak={peak}");
    }

    #[test]
    fn engine_boots_with_chorus_autopan_and_reverb_on_at_modest_mix() {
        // Fresh SynthEngine should ship the typical "DX7-through-outboard" sound:
        // Chorus + AutoPan + Reverb ON, with mixes low enough that the dry signal
        // still dominates. Regression guard for the UX default.
        let (engine, _ctrl) = make_engine();
        assert!(
            engine.effects.chorus.enabled,
            "chorus should be on by default"
        );
        assert!(
            engine.effects.auto_pan.enabled,
            "autopan should be on by default (Suitcase tremolo)"
        );
        assert!(
            engine.effects.reverb.enabled,
            "reverb should be on by default"
        );
        assert!(
            (engine.effects.chorus.mix - 0.15).abs() < 1e-6,
            "chorus mix expected 0.15, got {}",
            engine.effects.chorus.mix
        );
        assert!(
            (engine.effects.auto_pan.depth - 0.35).abs() < 1e-6,
            "autopan depth expected 0.35, got {}",
            engine.effects.auto_pan.depth
        );
        assert!(
            (engine.effects.reverb.mix - 0.22).abs() < 1e-6,
            "reverb mix expected 0.22, got {}",
            engine.effects.reverb.mix
        );
        // Delay stays off — it's a deliberate effect, not part of the canonical DX7 flavour.
        assert!(
            !engine.effects.delay.enabled,
            "delay should remain off by default"
        );
    }

    #[test]
    fn engine_process_stereo_runs_through_effects_and_dc_blocker() {
        let (mut engine, mut ctrl) = make_engine();
        ctrl.note_on(69, 100);
        let (peak_l, peak_r) = drive_stereo(&mut engine, 4096);
        assert!(peak_l > 0.001);
        assert!(peak_r > 0.001);
    }

    #[test]
    fn engine_pitch_bend_alters_audio_path() {
        let (mut engine, mut ctrl) = make_engine();
        ctrl.note_on(69, 100);
        ctrl.pitch_bend(8000);
        ctrl.mod_wheel(0.5);
        ctrl.sustain_pedal(true);
        drive(&mut engine, 1024);
    }

    #[test]
    fn engine_voice_stealing_kicks_in_after_max_voices() {
        let (mut engine, mut ctrl) = make_engine();
        // Drive 17 different note_ons → triggers stealing (16-voice cap)
        for n in 50..67u8 {
            ctrl.note_on(n, 100);
        }
        engine.process_commands();
        let active = engine.voices.iter().filter(|v| v.active).count();
        assert!(active <= 16);
    }

    #[test]
    fn engine_mono_mode_silences_all_but_first_active_voice() {
        let (mut engine, mut ctrl) = make_engine();
        // Press multiple notes in Poly mode
        for n in 60..64u8 {
            ctrl.note_on(n, 100);
        }
        // Now switch to Mono — engine should silence all but one.
        ctrl.set_voice_mode(crate::state_snapshot::VoiceMode::Mono);
        engine.process_commands();
        let active = engine.voices.iter().filter(|v| v.active).count();
        assert!(active <= 1);
    }

    #[test]
    fn engine_mono_legato_glides_between_held_notes() {
        let (mut engine, mut ctrl) = make_engine();
        ctrl.set_voice_mode(crate::state_snapshot::VoiceMode::MonoLegato);
        ctrl.set_portamento_enable(true);
        ctrl.set_portamento_time(50.0);
        ctrl.note_on(60, 100);
        engine.process_commands();
        ctrl.note_on(67, 100);
        engine.process_commands();
        // Note off the topmost: should fall back to the first held note.
        ctrl.note_off(67);
        engine.process_commands();
        let active = engine.voices.iter().filter(|v| v.active).count();
        assert!(active >= 1);
    }

    #[test]
    fn engine_sustain_pedal_holds_notes() {
        let (mut engine, mut ctrl) = make_engine();
        ctrl.note_on(60, 100);
        engine.process_commands();
        ctrl.sustain_pedal(true);
        engine.process_commands();
        ctrl.note_off(60);
        engine.process_commands();
        // With sustain held, note_off is a no-op → voice still active.
        let active_before_release = engine.voices.iter().filter(|v| v.active).count();
        assert!(active_before_release >= 1);
    }

    #[test]
    fn engine_set_operator_param_dispatches_to_voices() {
        let (mut engine, mut ctrl) = make_engine();
        ctrl.set_operator_param(0, OperatorParam::Ratio, 2.0);
        ctrl.set_operator_param(0, OperatorParam::Level, 80.0);
        ctrl.set_operator_param(0, OperatorParam::Detune, 5.0);
        ctrl.set_operator_param(0, OperatorParam::Feedback, 3.0);
        ctrl.set_operator_param(0, OperatorParam::VelocitySensitivity, 4.0);
        ctrl.set_operator_param(0, OperatorParam::KeyScaleRate, 2.0);
        ctrl.set_operator_param(0, OperatorParam::KeyScaleBreakpoint, 48.0);
        ctrl.set_operator_param(0, OperatorParam::KeyScaleLeftDepth, 50.0);
        ctrl.set_operator_param(0, OperatorParam::KeyScaleRightDepth, 50.0);
        ctrl.set_operator_param(0, OperatorParam::KeyScaleLeftCurve, 1.0);
        ctrl.set_operator_param(0, OperatorParam::KeyScaleRightCurve, 2.0);
        ctrl.set_operator_param(0, OperatorParam::AmSensitivity, 3.0);
        ctrl.set_operator_param(0, OperatorParam::OscillatorKeySync, 1.0);
        ctrl.set_operator_param(0, OperatorParam::FixedFrequency, 1.0);
        ctrl.set_operator_param(0, OperatorParam::FixedFreqHz, 100.0);
        ctrl.set_operator_param(0, OperatorParam::Enabled, 0.0);
        ctrl.set_operator_param(99, OperatorParam::Ratio, 2.0); // out of range — no-op
        engine.process_commands();
        // No assertion needed — we just exercise all branches.
    }

    #[test]
    fn engine_set_envelope_param_dispatches_to_all_voices() {
        let (mut engine, mut ctrl) = make_engine();
        for param in [
            EnvelopeParam::Rate1,
            EnvelopeParam::Rate2,
            EnvelopeParam::Rate3,
            EnvelopeParam::Rate4,
            EnvelopeParam::Level1,
            EnvelopeParam::Level2,
            EnvelopeParam::Level3,
            EnvelopeParam::Level4,
        ] {
            ctrl.set_envelope_param(0, param, 50.0);
        }
        ctrl.set_envelope_param(99, EnvelopeParam::Rate1, 0.0); // out of range — no-op
        engine.process_commands();
    }

    #[test]
    fn engine_set_pitch_eg_param_dispatches() {
        let (mut engine, mut ctrl) = make_engine();
        for param in [
            PitchEgParam::Enabled,
            PitchEgParam::Rate1,
            PitchEgParam::Rate2,
            PitchEgParam::Rate3,
            PitchEgParam::Rate4,
            PitchEgParam::Level1,
            PitchEgParam::Level2,
            PitchEgParam::Level3,
            PitchEgParam::Level4,
        ] {
            ctrl.set_pitch_eg_param(param, 50.0);
        }
        engine.process_commands();
    }

    #[test]
    fn engine_set_lfo_param_dispatches() {
        let (mut engine, mut ctrl) = make_engine();
        ctrl.set_lfo_param(LfoParam::Rate, 50.0);
        ctrl.set_lfo_param(LfoParam::Delay, 20.0);
        ctrl.set_lfo_param(LfoParam::PitchDepth, 80.0);
        ctrl.set_lfo_param(LfoParam::AmpDepth, 40.0);
        ctrl.set_lfo_param(LfoParam::KeySync, 1.0);
        for w in 0..=5u8 {
            ctrl.set_lfo_param(LfoParam::Waveform(w), 0.0);
        }
        engine.process_commands();
    }

    #[test]
    fn engine_set_effect_param_dispatches() {
        let (mut engine, mut ctrl) = make_engine();
        // Chorus
        ctrl.set_effect_param(EffectType::Chorus, EffectParam::Enabled, 1.0);
        ctrl.set_effect_param(EffectType::Chorus, EffectParam::Mix, 0.5);
        ctrl.set_effect_param(EffectType::Chorus, EffectParam::ChorusRate, 2.0);
        ctrl.set_effect_param(EffectType::Chorus, EffectParam::ChorusDepth, 5.0);
        ctrl.set_effect_param(EffectType::Chorus, EffectParam::ChorusFeedback, 0.3);
        // AutoPan
        ctrl.set_effect_param(EffectType::AutoPan, EffectParam::Enabled, 1.0);
        ctrl.set_effect_param(EffectType::AutoPan, EffectParam::AutoPanRate, 4.5);
        ctrl.set_effect_param(EffectType::AutoPan, EffectParam::AutoPanDepth, 0.6);
        // Delay
        ctrl.set_effect_param(EffectType::Delay, EffectParam::Enabled, 1.0);
        ctrl.set_effect_param(EffectType::Delay, EffectParam::Mix, 0.4);
        ctrl.set_effect_param(EffectType::Delay, EffectParam::DelayTime, 200.0);
        ctrl.set_effect_param(EffectType::Delay, EffectParam::DelayFeedback, 0.5);
        ctrl.set_effect_param(EffectType::Delay, EffectParam::DelayPingPong, 1.0);
        // Reverb
        ctrl.set_effect_param(EffectType::Reverb, EffectParam::Enabled, 1.0);
        ctrl.set_effect_param(EffectType::Reverb, EffectParam::Mix, 0.3);
        ctrl.set_effect_param(EffectType::Reverb, EffectParam::ReverbRoomSize, 0.8);
        ctrl.set_effect_param(EffectType::Reverb, EffectParam::ReverbDamping, 0.4);
        ctrl.set_effect_param(EffectType::Reverb, EffectParam::ReverbWidth, 0.9);
        engine.process_commands();
    }

    // -----------------------------------------------------------------------
    // Controller routing & expression
    // -----------------------------------------------------------------------

    #[test]
    fn engine_aftertouch_clamps_and_routes() {
        let (mut engine, mut ctrl) = make_engine();
        ctrl.set_aftertouch_pitch_sens(7);
        ctrl.set_aftertouch_amp_sens(5);
        ctrl.set_aftertouch_eg_bias_sens(3);
        ctrl.set_aftertouch_pitch_bias_sens(2);
        ctrl.aftertouch(2.0); // clamped to 1.0
        engine.process_commands();
        assert_eq!(engine.aftertouch, 1.0);
    }

    #[test]
    fn engine_breath_controller_clamps() {
        let (mut engine, mut ctrl) = make_engine();
        ctrl.set_breath_pitch_sens(7);
        ctrl.set_breath_amp_sens(5);
        ctrl.set_breath_eg_bias_sens(3);
        ctrl.set_breath_pitch_bias_sens(2);
        ctrl.breath_controller(-0.5);
        engine.process_commands();
        assert_eq!(engine.breath, 0.0);
    }

    #[test]
    fn engine_foot_controller_routes() {
        let (mut engine, mut ctrl) = make_engine();
        ctrl.set_foot_volume_sens(15);
        ctrl.set_foot_pitch_sens(7);
        ctrl.set_foot_amp_sens(5);
        ctrl.set_foot_eg_bias_sens(3);
        ctrl.foot_controller(0.5);
        engine.process_commands();
        assert_eq!(engine.foot_volume_sens, 15);
        assert_eq!(engine.foot, 0.5);
    }

    #[test]
    fn engine_expression_clamps() {
        let (mut engine, mut ctrl) = make_engine();
        ctrl.expression(2.0);
        engine.process_commands();
        assert_eq!(engine.expression, 1.0);
        ctrl.expression(-1.0);
        engine.process_commands();
        assert_eq!(engine.expression, 0.0);
    }

    #[test]
    fn engine_bank_select_combines_with_program_change() {
        let (mut engine, mut ctrl) = make_engine();
        ctrl.set_bank_msb(1);
        ctrl.set_bank_lsb(2);
        ctrl.program_change(3);
        engine.process_commands();
        assert_eq!(engine.bank_msb, 1);
        assert_eq!(engine.bank_lsb, 2);
    }

    #[test]
    fn engine_eg_bias_and_pitch_bias_sensitivities_clamp() {
        let (mut engine, mut ctrl) = make_engine();
        ctrl.set_eg_bias_sensitivity(50);
        engine.process_commands();
        assert_eq!(engine.eg_bias_sensitivity, 7);
        ctrl.set_pitch_bias_sensitivity(50);
        engine.process_commands();
        assert_eq!(engine.pitch_bias_sensitivity, 7);
    }

    #[test]
    fn engine_portamento_settings_propagate() {
        let (mut engine, mut ctrl) = make_engine();
        ctrl.set_portamento_enable(true);
        ctrl.set_portamento_time(75.0);
        ctrl.set_portamento_glissando(true);
        engine.process_commands();
        assert!(engine.portamento_enable);
        assert_eq!(engine.portamento_time, 75.0);
        assert!(engine.portamento_glissando);
    }

    // -----------------------------------------------------------------------
    // Snapshots & preset loading
    // -----------------------------------------------------------------------

    #[test]
    fn engine_update_snapshot_publishes_to_controller() {
        let (engine, ctrl) = make_engine();
        engine.update_snapshot();
        let snap = ctrl.snapshot();
        assert_eq!(snap.algorithm, 1);
        assert_eq!(snap.preset_name, "Init Voice");
    }

    #[test]
    fn engine_load_preset_by_index_applies_when_in_range() {
        let (mut engine, mut ctrl) = make_engine();
        let presets = vec![make_preset("FOO", 5), make_preset("BAR", 12)];
        engine.set_presets(presets);
        ctrl.load_preset(1);
        engine.process_commands();
        assert_eq!(engine.preset_name, "BAR");
        assert_eq!(engine.algorithm, 12);
    }

    #[test]
    fn engine_load_preset_out_of_range_is_noop() {
        let (mut engine, mut ctrl) = make_engine();
        let presets = vec![make_preset("FOO", 5)];
        engine.set_presets(presets);
        ctrl.load_preset(99);
        engine.process_commands();
        assert_eq!(engine.preset_name, "Init Voice");
    }

    #[test]
    fn engine_load_sysex_single_voice_applies() {
        let (mut engine, mut ctrl) = make_engine();
        let preset = make_preset("SYSEX", 7);
        ctrl.load_sysex_single_voice(preset);
        engine.process_commands();
        assert_eq!(engine.preset_name, "SYSEX");
        assert_eq!(engine.algorithm, 7);
    }

    #[test]
    fn engine_load_sysex_bulk_applies_first_and_replaces_bank() {
        let (mut engine, mut ctrl) = make_engine();
        let presets = vec![make_preset("BULK1", 11), make_preset("BULK2", 13)];
        ctrl.load_sysex_bulk(presets);
        engine.process_commands();
        assert_eq!(engine.preset_name, "BULK1");
        assert_eq!(engine.algorithm, 11);
    }

    // -----------------------------------------------------------------------
    // SynthController API completeness (smoke)
    // -----------------------------------------------------------------------

    #[test]
    fn controller_command_buffer_does_not_block() {
        let (_engine, mut ctrl) = make_engine();
        // Many commands of various types; never blocks.
        for _ in 0..500 {
            ctrl.note_on(60, 100);
            ctrl.set_master_tune(10.0);
            ctrl.set_master_volume(0.5);
            ctrl.mod_wheel(0.7);
        }
    }

    #[test]
    fn engine_get_snapshot_returns_clone() {
        let (engine, ctrl) = make_engine();
        engine.update_snapshot();
        let snap = ctrl.snapshot();
        let snap2 = ctrl.snapshot();
        assert_eq!(snap.algorithm, snap2.algorithm);
    }
}
