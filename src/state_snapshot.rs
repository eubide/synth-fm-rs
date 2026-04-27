use crate::lfo::LFOWaveform;
use crate::lock_free::TripleBuffer;
use crate::operator::KeyScaleCurve;
use std::sync::Arc;

/// Snapshot of a single operator's state for GUI display.
#[allow(dead_code)] // some fields are populated for future panels not yet wired up
#[derive(Debug, Clone, Copy)]
pub struct OperatorSnapshot {
    pub enabled: bool,
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
    // Envelope parameters
    pub rate1: f32,
    pub rate2: f32,
    pub rate3: f32,
    pub rate4: f32,
    pub level1: f32,
    pub level2: f32,
    pub level3: f32,
    pub level4: f32,
    /// Live envelope output (0..=1), max across active voices.
    pub live_level: f32,
}

impl Default for OperatorSnapshot {
    fn default() -> Self {
        Self {
            enabled: true,
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
            rate1: 99.0,
            rate2: 50.0,
            rate3: 35.0,
            rate4: 50.0,
            level1: 99.0,
            level2: 75.0,
            level3: 50.0,
            level4: 0.0,
            live_level: 0.0,
        }
    }
}

/// Snapshot of chorus effect state
#[derive(Debug, Clone, Copy)]
pub struct ChorusSnapshot {
    pub enabled: bool,
    pub rate: f32,
    pub depth: f32,
    pub mix: f32,
    pub feedback: f32,
}

impl Default for ChorusSnapshot {
    fn default() -> Self {
        Self {
            enabled: false,
            rate: 1.5,
            depth: 3.0,
            mix: 0.5,
            feedback: 0.2,
        }
    }
}

/// Snapshot of delay effect state
#[derive(Debug, Clone, Copy)]
pub struct DelaySnapshot {
    pub enabled: bool,
    pub time_ms: f32,
    pub feedback: f32,
    pub mix: f32,
    pub ping_pong: bool,
}

impl Default for DelaySnapshot {
    fn default() -> Self {
        Self {
            enabled: false,
            time_ms: 300.0,
            feedback: 0.4,
            mix: 0.3,
            ping_pong: true,
        }
    }
}

/// Snapshot of autopan effect state
#[derive(Debug, Clone, Copy)]
pub struct AutoPanSnapshot {
    pub enabled: bool,
    pub rate_hz: f32,
    pub depth: f32,
}

impl Default for AutoPanSnapshot {
    fn default() -> Self {
        Self {
            enabled: false,
            rate_hz: 5.0,
            depth: 0.5,
        }
    }
}

/// Snapshot of reverb effect state
#[derive(Debug, Clone, Copy)]
pub struct ReverbSnapshot {
    pub enabled: bool,
    pub room_size: f32,
    pub damping: f32,
    pub mix: f32,
    pub width: f32,
}

impl Default for ReverbSnapshot {
    fn default() -> Self {
        Self {
            enabled: false,
            room_size: 0.7,
            damping: 0.5,
            mix: 0.25,
            width: 1.0,
        }
    }
}

/// DX7 voice mode: poly, mono with full portamento, or mono with legato
/// portamento (only when previous note still held).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum VoiceMode {
    #[default]
    Poly,
    Mono,
    MonoLegato,
}

/// Pitch envelope state mirrored to GUI for display.
#[derive(Debug, Clone, Copy)]
pub struct PitchEgSnapshot {
    pub enabled: bool,
    pub rate1: f32,
    pub rate2: f32,
    pub rate3: f32,
    pub rate4: f32,
    pub level1: f32,
    pub level2: f32,
    pub level3: f32,
    pub level4: f32,
}

impl Default for PitchEgSnapshot {
    fn default() -> Self {
        // DX7 pitch EG defaults: all rates 99 (instant), all levels 50 (= no offset).
        Self {
            enabled: false,
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

/// Read-only snapshot of synthesizer state for GUI display.
/// Updated by audio thread, read by GUI thread without blocking.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SynthSnapshot {
    // Voice info
    pub preset_name: String,
    pub algorithm: u8,
    pub active_voices: u8,

    // Global parameters
    pub master_volume: f32,
    pub master_tune: f32,
    pub voice_mode: VoiceMode,
    pub portamento_enable: bool,
    pub portamento_time: f32,
    pub portamento_glissando: bool, // portamento step ON/OFF
    pub pitch_bend_range: f32,
    pub transpose_semitones: i8, // -24..+24 semitones, 0 means C3 (DX7 reference)
    pub pitch_mod_sensitivity: u8, // 0-7 PMS (LFO pitch depth scaler)
    pub eg_bias_sensitivity: u8, // 0-7 EG Bias routing from Mod Wheel
    pub pitch_bias_sensitivity: u8, // 0-7 Pitch Bias routing from Mod Wheel

    // Real-time controllers
    pub pitch_bend: f32,
    pub mod_wheel: f32,
    pub sustain_pedal: bool,
    pub aftertouch: f32,
    pub breath: f32,
    pub foot: f32,
    pub expression: f32,

    // Aftertouch routing sensitivities (0-7 each)
    pub aftertouch_pitch_sens: u8,
    pub aftertouch_amp_sens: u8,
    pub aftertouch_eg_bias_sens: u8,
    pub aftertouch_pitch_bias_sens: u8,

    // Breath Controller routing sensitivities (0-7 each)
    pub breath_pitch_sens: u8,
    pub breath_amp_sens: u8,
    pub breath_eg_bias_sens: u8,
    pub breath_pitch_bias_sens: u8,

    // Foot Controller: VOLUME 0-15, others 0-7
    pub foot_volume_sens: u8,
    pub foot_pitch_sens: u8,
    pub foot_amp_sens: u8,
    pub foot_eg_bias_sens: u8,

    // LFO state
    pub lfo_rate: f32,
    pub lfo_delay: f32,
    pub lfo_pitch_depth: f32,
    pub lfo_amp_depth: f32,
    pub lfo_waveform: LFOWaveform,
    pub lfo_key_sync: bool,
    pub lfo_frequency_hz: f32,
    pub lfo_delay_seconds: f32,

    // Pitch EG state
    pub pitch_eg: PitchEgSnapshot,

    // Effects state (detailed for effects panel)
    pub chorus: ChorusSnapshot,
    pub auto_pan: AutoPanSnapshot,
    pub delay: DelaySnapshot,
    pub reverb: ReverbSnapshot,

    // Operator states (detailed for editor)
    pub operators: [OperatorSnapshot; 6],
}

impl Default for SynthSnapshot {
    fn default() -> Self {
        Self {
            preset_name: "Init Voice".to_string(),
            algorithm: 1,
            active_voices: 0,

            master_volume: 0.7,
            master_tune: 0.0,
            voice_mode: VoiceMode::Poly,
            portamento_enable: false,
            portamento_time: 50.0,
            portamento_glissando: false,
            pitch_bend_range: 2.0,
            transpose_semitones: 0,
            pitch_mod_sensitivity: 0,
            eg_bias_sensitivity: 0,
            pitch_bias_sensitivity: 0,

            pitch_bend: 0.0,
            mod_wheel: 0.0,
            sustain_pedal: false,
            aftertouch: 0.0,
            breath: 0.0,
            foot: 0.0,
            expression: 1.0,

            aftertouch_pitch_sens: 0,
            aftertouch_amp_sens: 0,
            aftertouch_eg_bias_sens: 0,
            aftertouch_pitch_bias_sens: 0,

            breath_pitch_sens: 0,
            breath_amp_sens: 0,
            breath_eg_bias_sens: 0,
            breath_pitch_bias_sens: 0,

            foot_volume_sens: 0,
            foot_pitch_sens: 0,
            foot_amp_sens: 0,
            foot_eg_bias_sens: 0,

            lfo_rate: 35.0,
            lfo_delay: 0.0,
            lfo_pitch_depth: 0.0,
            lfo_amp_depth: 0.0,
            lfo_waveform: LFOWaveform::Triangle,
            lfo_key_sync: false,
            lfo_frequency_hz: 0.0,
            lfo_delay_seconds: 0.0,

            pitch_eg: PitchEgSnapshot::default(),

            chorus: ChorusSnapshot::default(),
            auto_pan: AutoPanSnapshot::default(),
            delay: DelaySnapshot::default(),
            reverb: ReverbSnapshot::default(),

            operators: [OperatorSnapshot::default(); 6],
        }
    }
}

/// Sender side of snapshot channel (audio thread)
pub struct SnapshotSender {
    buffer: Arc<TripleBuffer<SynthSnapshot>>,
}

impl SnapshotSender {
    /// Update the snapshot (audio thread)
    pub fn send(&self, snapshot: SynthSnapshot) {
        self.buffer.write(snapshot);
    }
}

/// Receiver side of snapshot channel (GUI thread)
#[allow(dead_code)]
pub struct SnapshotReceiver {
    buffer: Arc<TripleBuffer<SynthSnapshot>>,
}

impl SnapshotReceiver {
    /// Get the latest snapshot (GUI thread)
    /// This swaps in the latest data from the audio thread
    pub fn get(&self) -> &SynthSnapshot {
        self.buffer.read()
    }

    /// Peek at current snapshot without swapping
    #[allow(dead_code)]
    pub fn peek(&self) -> &SynthSnapshot {
        self.buffer.peek()
    }
}

/// Create a new snapshot channel pair (sender, receiver)
pub fn create_snapshot_channel() -> (SnapshotSender, SnapshotReceiver) {
    let buffer = Arc::new(TripleBuffer::new(SynthSnapshot::default()));

    (
        SnapshotSender {
            buffer: buffer.clone(),
        },
        SnapshotReceiver { buffer },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_default() {
        let snapshot = SynthSnapshot::default();
        assert_eq!(snapshot.algorithm, 1);
        assert_eq!(snapshot.preset_name, "Init Voice");
        assert_eq!(snapshot.voice_mode, VoiceMode::Poly);
    }

    #[test]
    fn test_snapshot_channel() {
        let (sender, receiver) = create_snapshot_channel();

        // Initial state
        let snapshot = receiver.get();
        assert_eq!(snapshot.algorithm, 1);

        // Update state
        let new_snapshot = SynthSnapshot {
            algorithm: 5,
            preset_name: "E.PIANO 1".to_string(),
            active_voices: 3,
            ..SynthSnapshot::default()
        };
        sender.send(new_snapshot);

        // Read updated state
        let snapshot = receiver.get();
        assert_eq!(snapshot.algorithm, 5);
        assert_eq!(snapshot.preset_name, "E.PIANO 1");
        assert_eq!(snapshot.active_voices, 3);
    }

    #[test]
    fn test_snapshot_concurrent() {
        use std::sync::Arc;
        use std::thread;

        let (sender, receiver) = create_snapshot_channel();
        let sender = Arc::new(sender);
        let receiver = Arc::new(receiver);

        let s = sender.clone();
        let writer = thread::spawn(move || {
            for i in 0..1000 {
                let snapshot = SynthSnapshot {
                    active_voices: (i % 16) as u8,
                    ..SynthSnapshot::default()
                };
                s.send(snapshot);
            }
        });

        let r = receiver.clone();
        let reader = thread::spawn(move || {
            for _ in 0..1000 {
                let snapshot = r.get();
                // Verify no corruption (active_voices should always be 0-15)
                assert!(snapshot.active_voices < 16);
            }
        });

        writer.join().unwrap();
        reader.join().unwrap();
    }
}
