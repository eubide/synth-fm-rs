use crate::fm_synth::SynthEngine;
use crate::lfo::LFOWaveform;
use crate::operator::KeyScaleCurve;
use crate::state_snapshot::SynthSnapshot;

/// Per-operator parameters captured from a DX7 voice.
#[derive(Clone, Debug)]
pub struct PresetOperator {
    pub frequency_ratio: f32,
    pub output_level: f32,
    pub detune: f32,
    pub feedback: f32,
    pub velocity_sensitivity: f32,
    pub key_scale_rate: f32,
    pub key_scale_breakpoint: u8,
    pub key_scale_left_curve: KeyScaleCurve,
    pub key_scale_right_curve: KeyScaleCurve,
    pub key_scale_left_depth: f32,
    pub key_scale_right_depth: f32,
    pub am_sensitivity: u8,
    pub oscillator_key_sync: bool,
    pub fixed_frequency: bool,
    pub fixed_freq_hz: f32,
    /// Envelope: (r1, r2, r3, r4, l1, l2, l3, l4).
    pub envelope: (f32, f32, f32, f32, f32, f32, f32, f32),
}

impl Default for PresetOperator {
    fn default() -> Self {
        Self {
            frequency_ratio: 1.0,
            output_level: 99.0,
            detune: 0.0,
            feedback: 0.0,
            velocity_sensitivity: 0.0,
            key_scale_rate: 0.0,
            key_scale_breakpoint: 60,
            key_scale_left_curve: KeyScaleCurve::default(),
            key_scale_right_curve: KeyScaleCurve::default(),
            key_scale_left_depth: 0.0,
            key_scale_right_depth: 0.0,
            am_sensitivity: 0,
            oscillator_key_sync: true,
            fixed_frequency: false,
            fixed_freq_hz: 440.0,
            envelope: (99.0, 50.0, 50.0, 50.0, 99.0, 75.0, 50.0, 0.0),
        }
    }
}

/// Pitch envelope settings for a preset.
#[derive(Clone, Debug)]
pub struct PresetPitchEg {
    pub rate1: f32,
    pub rate2: f32,
    pub rate3: f32,
    pub rate4: f32,
    pub level1: f32,
    pub level2: f32,
    pub level3: f32,
    pub level4: f32,
}

impl PresetPitchEg {
    /// Returns true when the EG would produce any audible offset (any level != 50).
    pub fn is_active(&self) -> bool {
        const NEUTRAL: f32 = 50.0;
        (self.level1 - NEUTRAL).abs() > 0.5
            || (self.level2 - NEUTRAL).abs() > 0.5
            || (self.level3 - NEUTRAL).abs() > 0.5
            || (self.level4 - NEUTRAL).abs() > 0.5
    }
}

impl Default for PresetPitchEg {
    fn default() -> Self {
        Self {
            rate1: 99.0,
            rate2: 99.0,
            rate3: 99.0,
            rate4: 99.0,
            level1: 50.0,
            level2: 50.0,
            level3: 50.0,
            level4: 50.0,
        }
    }
}

/// LFO settings for a preset.
#[derive(Clone, Debug)]
pub struct PresetLfo {
    pub waveform: LFOWaveform,
    pub rate: f32,
    pub delay: f32,
    pub pitch_mod_depth: f32,
    pub amp_mod_depth: f32,
    pub key_sync: bool,
}

impl Default for PresetLfo {
    fn default() -> Self {
        Self {
            waveform: LFOWaveform::Triangle,
            rate: 35.0,
            delay: 0.0,
            pitch_mod_depth: 0.0,
            amp_mod_depth: 0.0,
            key_sync: false,
        }
    }
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct Dx7Preset {
    pub name: String,
    pub collection: String,
    pub algorithm: u8,
    pub operators: [PresetOperator; 6],
    pub master_tune: Option<f32>,
    pub pitch_bend_range: Option<f32>,
    pub portamento_enable: Option<bool>,
    pub portamento_time: Option<f32>,
    /// Voice mode: None = leave synth as-is. Some = override.
    pub mono_mode: Option<bool>,
    /// Transpose in semitones from the DX7 reference (0 = C3 / no shift).
    pub transpose_semitones: i8,
    pub pitch_mod_sensitivity: u8,
    pub pitch_eg: Option<PresetPitchEg>,
    pub lfo: Option<PresetLfo>,
}

impl Dx7Preset {
    /// Build a preset from a live state snapshot. Used to export the current
    /// edit buffer (e.g. as a DX7 SysEx single-voice dump).
    pub fn from_snapshot(snapshot: &SynthSnapshot) -> Self {
        let operators: [PresetOperator; 6] = std::array::from_fn(|i| {
            let op = &snapshot.operators[i];
            PresetOperator {
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
                envelope: (
                    op.rate1, op.rate2, op.rate3, op.rate4, op.level1, op.level2, op.level3,
                    op.level4,
                ),
            }
        });

        let pitch_eg = PresetPitchEg {
            rate1: snapshot.pitch_eg.rate1,
            rate2: snapshot.pitch_eg.rate2,
            rate3: snapshot.pitch_eg.rate3,
            rate4: snapshot.pitch_eg.rate4,
            level1: snapshot.pitch_eg.level1,
            level2: snapshot.pitch_eg.level2,
            level3: snapshot.pitch_eg.level3,
            level4: snapshot.pitch_eg.level4,
        };

        let lfo = PresetLfo {
            waveform: snapshot.lfo_waveform,
            rate: snapshot.lfo_rate,
            delay: snapshot.lfo_delay,
            pitch_mod_depth: snapshot.lfo_pitch_depth,
            amp_mod_depth: snapshot.lfo_amp_depth,
            key_sync: snapshot.lfo_key_sync,
        };

        Self {
            name: snapshot.preset_name.clone(),
            collection: "current".to_string(),
            algorithm: snapshot.algorithm,
            operators,
            master_tune: Some(snapshot.master_tune),
            pitch_bend_range: Some(snapshot.pitch_bend_range),
            portamento_enable: Some(snapshot.portamento_enable),
            portamento_time: Some(snapshot.portamento_time),
            mono_mode: None,
            transpose_semitones: snapshot.transpose_semitones,
            pitch_mod_sensitivity: snapshot.pitch_mod_sensitivity,
            pitch_eg: Some(pitch_eg),
            lfo: Some(lfo),
        }
    }

    /// Apply this preset to the synth: algorithm, name, per-operator parameters,
    /// optional global parameters, pitch EG, and LFO. Voice mode and portamento
    /// stay as the synth had them unless explicitly set.
    pub fn apply_to_synth(&self, synth: &mut SynthEngine) {
        synth.set_algorithm(self.algorithm);
        synth.set_preset_name(self.name.clone());

        synth.set_transpose_semitones(self.transpose_semitones);
        synth.set_pitch_mod_sensitivity(self.pitch_mod_sensitivity);
        if let Some(range) = self.pitch_bend_range {
            synth.set_pitch_bend_range(range);
        }

        // Pitch EG
        if let Some(peg) = &self.pitch_eg {
            let active = peg.is_active();
            let p = synth.pitch_eg_mut();
            p.enabled = active;
            p.rate1 = peg.rate1;
            p.rate2 = peg.rate2;
            p.rate3 = peg.rate3;
            p.rate4 = peg.rate4;
            p.level1 = peg.level1;
            p.level2 = peg.level2;
            p.level3 = peg.level3;
            p.level4 = peg.level4;
        } else {
            synth.pitch_eg_mut().enabled = false;
        }

        // LFO
        if let Some(lfo) = &self.lfo {
            let dst = synth.lfo_mut();
            dst.set_waveform(lfo.waveform);
            dst.set_rate(lfo.rate);
            dst.set_delay(lfo.delay);
            dst.set_pitch_depth(lfo.pitch_mod_depth);
            dst.set_amp_depth(lfo.amp_mod_depth);
            dst.set_key_sync(lfo.key_sync);
        }

        for voice in synth.voices_mut() {
            for (i, op) in voice.operators.iter_mut().enumerate() {
                let p = &self.operators[i];
                op.frequency_ratio = p.frequency_ratio;
                op.output_level = p.output_level;
                op.detune = p.detune;
                op.feedback = p.feedback;
                op.velocity_sensitivity = p.velocity_sensitivity;
                op.key_scale_rate = p.key_scale_rate;
                op.key_scale_breakpoint = p.key_scale_breakpoint;
                op.key_scale_left_curve = p.key_scale_left_curve;
                op.key_scale_right_curve = p.key_scale_right_curve;
                op.key_scale_left_depth = p.key_scale_left_depth;
                op.key_scale_right_depth = p.key_scale_right_depth;
                op.am_sensitivity = p.am_sensitivity;
                op.oscillator_key_sync = p.oscillator_key_sync;
                op.fixed_frequency = p.fixed_frequency;
                op.fixed_freq_hz = p.fixed_freq_hz;
                let (r1, r2, r3, r4, l1, l2, l3, l4) = p.envelope;
                op.envelope.rate1 = r1;
                op.envelope.rate2 = r2;
                op.envelope.rate3 = r3;
                op.envelope.rate4 = r4;
                op.envelope.level1 = l1;
                op.envelope.level2 = l2;
                op.envelope.level3 = l3;
                op.envelope.level4 = l4;
                op.update_frequency();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_queue::create_command_queue;
    use crate::fm_synth::SynthEngine;
    use crate::state_snapshot::create_snapshot_channel;

    fn make_engine() -> SynthEngine {
        let (_tx, rx) = create_command_queue();
        let (snap_tx, _snap_rx) = create_snapshot_channel();
        SynthEngine::new(44_100.0, rx, snap_tx)
    }

    #[test]
    fn preset_operator_default_has_sane_values() {
        let op = PresetOperator::default();
        assert_eq!(op.frequency_ratio, 1.0);
        assert_eq!(op.output_level, 99.0);
        assert!(op.oscillator_key_sync);
        assert!(!op.fixed_frequency);
    }

    #[test]
    fn preset_lfo_default_uses_triangle() {
        let lfo = PresetLfo::default();
        assert_eq!(lfo.waveform, crate::lfo::LFOWaveform::Triangle);
    }

    #[test]
    fn preset_pitch_eg_default_is_inactive() {
        let peg = PresetPitchEg::default();
        assert!(!peg.is_active());
    }

    #[test]
    fn preset_pitch_eg_is_active_when_any_level_offset() {
        let peg = PresetPitchEg {
            level1: 70.0,
            ..PresetPitchEg::default()
        };
        assert!(peg.is_active());

        let peg = PresetPitchEg {
            level4: 30.0,
            ..PresetPitchEg::default()
        };
        assert!(peg.is_active());
    }

    #[test]
    fn preset_pitch_eg_treats_tiny_offsets_as_inactive() {
        let peg = PresetPitchEg {
            rate1: 99.0,
            rate2: 99.0,
            rate3: 99.0,
            rate4: 99.0,
            level1: 50.3, // within ±0.5 → inactive
            level2: 50.0,
            level3: 50.0,
            level4: 50.0,
        };
        assert!(!peg.is_active());
    }

    #[test]
    fn from_snapshot_round_trips_basic_fields() {
        let snap = crate::state_snapshot::SynthSnapshot {
            algorithm: 7,
            preset_name: "FROM SNAPSHOT".to_string(),
            transpose_semitones: 5,
            pitch_mod_sensitivity: 4,
            ..crate::state_snapshot::SynthSnapshot::default()
        };
        let preset = Dx7Preset::from_snapshot(&snap);
        assert_eq!(preset.name, "FROM SNAPSHOT");
        assert_eq!(preset.algorithm, 7);
        assert_eq!(preset.transpose_semitones, 5);
        assert_eq!(preset.pitch_mod_sensitivity, 4);
        assert_eq!(preset.collection, "current");
        assert!(preset.lfo.is_some());
        assert!(preset.pitch_eg.is_some());
    }

    #[test]
    fn apply_to_synth_sets_algorithm_and_name() {
        let mut engine = make_engine();
        let preset = Dx7Preset {
            name: "APPLIED".to_string(),
            collection: "test".to_string(),
            algorithm: 11,
            operators: std::array::from_fn(|_| PresetOperator::default()),
            master_tune: None,
            pitch_bend_range: Some(3.0),
            portamento_enable: None,
            portamento_time: None,
            mono_mode: None,
            transpose_semitones: -3,
            pitch_mod_sensitivity: 5,
            pitch_eg: None,
            lfo: None,
        };
        preset.apply_to_synth(&mut engine);
        assert_eq!(engine.preset_name, "APPLIED");
        assert_eq!(engine.get_algorithm(), 11);
    }

    #[test]
    fn apply_to_synth_handles_active_pitch_eg() {
        let mut engine = make_engine();
        let peg = PresetPitchEg {
            level1: 80.0, // active
            ..PresetPitchEg::default()
        };
        let preset = Dx7Preset {
            name: "PEG".to_string(),
            collection: "test".to_string(),
            algorithm: 1,
            operators: std::array::from_fn(|_| PresetOperator::default()),
            master_tune: None,
            pitch_bend_range: None,
            portamento_enable: None,
            portamento_time: None,
            mono_mode: None,
            transpose_semitones: 0,
            pitch_mod_sensitivity: 0,
            pitch_eg: Some(peg),
            lfo: None,
        };
        preset.apply_to_synth(&mut engine);
        assert!(engine.pitch_eg.enabled);
        assert_eq!(engine.pitch_eg.level1, 80.0);
    }

    #[test]
    fn apply_to_synth_disables_pitch_eg_when_none() {
        let mut engine = make_engine();
        engine.pitch_eg.enabled = true;
        let preset = Dx7Preset {
            name: "NONE".to_string(),
            collection: "test".to_string(),
            algorithm: 1,
            operators: std::array::from_fn(|_| PresetOperator::default()),
            master_tune: None,
            pitch_bend_range: None,
            portamento_enable: None,
            portamento_time: None,
            mono_mode: None,
            transpose_semitones: 0,
            pitch_mod_sensitivity: 0,
            pitch_eg: None,
            lfo: None,
        };
        preset.apply_to_synth(&mut engine);
        assert!(!engine.pitch_eg.enabled);
    }

    #[test]
    fn apply_to_synth_propagates_lfo_settings() {
        let mut engine = make_engine();
        let lfo = PresetLfo {
            waveform: crate::lfo::LFOWaveform::Square,
            rate: 70.0,
            delay: 20.0,
            pitch_mod_depth: 50.0,
            amp_mod_depth: 30.0,
            key_sync: true,
        };
        let preset = Dx7Preset {
            name: "LFO".to_string(),
            collection: "test".to_string(),
            algorithm: 1,
            operators: std::array::from_fn(|_| PresetOperator::default()),
            master_tune: None,
            pitch_bend_range: None,
            portamento_enable: None,
            portamento_time: None,
            mono_mode: None,
            transpose_semitones: 0,
            pitch_mod_sensitivity: 0,
            pitch_eg: None,
            lfo: Some(lfo),
        };
        preset.apply_to_synth(&mut engine);
        assert_eq!(engine.get_lfo_waveform(), crate::lfo::LFOWaveform::Square);
        assert_eq!(engine.get_lfo_rate(), 70.0);
    }

    #[test]
    fn apply_to_synth_writes_operators_into_voices() {
        let mut engine = make_engine();
        let mut ops: [PresetOperator; 6] = std::array::from_fn(|_| PresetOperator::default());
        ops[0].frequency_ratio = 3.0;
        ops[0].output_level = 80.0;
        ops[5].feedback = 4.0;
        let preset = Dx7Preset {
            name: "OPS".to_string(),
            collection: "test".to_string(),
            algorithm: 5,
            operators: ops,
            master_tune: None,
            pitch_bend_range: None,
            portamento_enable: None,
            portamento_time: None,
            mono_mode: None,
            transpose_semitones: 0,
            pitch_mod_sensitivity: 0,
            pitch_eg: None,
            lfo: None,
        };
        preset.apply_to_synth(&mut engine);
        let voice = &engine.voices()[0];
        assert_eq!(voice.operators[0].frequency_ratio, 3.0);
        assert_eq!(voice.operators[0].output_level, 80.0);
        assert_eq!(voice.operators[5].feedback, 4.0);
    }
}
