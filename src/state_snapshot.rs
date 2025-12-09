use crate::lfo::LFOWaveform;
use crate::lock_free::TripleBuffer;
use std::sync::Arc;

/// Snapshot of a single operator's state for GUI display.
#[derive(Debug, Clone, Copy)]
pub struct OperatorSnapshot {
    pub enabled: bool,
    pub frequency_ratio: f32,
    pub output_level: f32,
    pub detune: f32,
    pub feedback: f32,
    pub velocity_sensitivity: f32,
    pub key_scale_level: f32,
    pub key_scale_rate: f32,
    // Envelope parameters
    pub rate1: f32,
    pub rate2: f32,
    pub rate3: f32,
    pub rate4: f32,
    pub level1: f32,
    pub level2: f32,
    pub level3: f32,
    pub level4: f32,
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
            key_scale_level: 0.0,
            key_scale_rate: 0.0,
            rate1: 99.0,
            rate2: 50.0,
            rate3: 35.0,
            rate4: 50.0,
            level1: 99.0,
            level2: 75.0,
            level3: 50.0,
            level4: 0.0,
        }
    }
}

/// Read-only snapshot of synthesizer state for GUI display.
/// Updated by audio thread, read by GUI thread without blocking.
#[derive(Debug, Clone)]
pub struct SynthSnapshot {
    // Voice info
    pub preset_name: String,
    pub algorithm: u8,
    pub active_voices: u8,

    // Global parameters
    pub master_volume: f32,
    pub master_tune: f32,
    pub mono_mode: bool,
    pub portamento_enable: bool,
    pub portamento_time: f32,
    pub pitch_bend_range: f32,

    // Real-time controllers
    pub pitch_bend: f32,
    pub mod_wheel: f32,
    pub sustain_pedal: bool,

    // LFO state
    pub lfo_rate: f32,
    pub lfo_delay: f32,
    pub lfo_pitch_depth: f32,
    pub lfo_amp_depth: f32,
    pub lfo_waveform: LFOWaveform,
    pub lfo_key_sync: bool,
    pub lfo_frequency_hz: f32,
    pub lfo_delay_seconds: f32,

    // Effects state
    pub chorus_enabled: bool,
    pub delay_enabled: bool,
    pub reverb_enabled: bool,

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
            mono_mode: false,
            portamento_enable: false,
            portamento_time: 50.0,
            pitch_bend_range: 2.0,

            pitch_bend: 0.0,
            mod_wheel: 0.0,
            sustain_pedal: false,

            lfo_rate: 35.0,
            lfo_delay: 0.0,
            lfo_pitch_depth: 0.0,
            lfo_amp_depth: 0.0,
            lfo_waveform: LFOWaveform::Triangle,
            lfo_key_sync: false,
            lfo_frequency_hz: 0.0,
            lfo_delay_seconds: 0.0,

            chorus_enabled: false,
            delay_enabled: false,
            reverb_enabled: false,

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
        assert!(!snapshot.mono_mode);
    }

    #[test]
    fn test_snapshot_channel() {
        let (sender, receiver) = create_snapshot_channel();

        // Initial state
        let snapshot = receiver.get();
        assert_eq!(snapshot.algorithm, 1);

        // Update state
        let mut new_snapshot = SynthSnapshot::default();
        new_snapshot.algorithm = 5;
        new_snapshot.preset_name = "E.PIANO 1".to_string();
        new_snapshot.active_voices = 3;
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
                let mut snapshot = SynthSnapshot::default();
                snapshot.active_voices = (i % 16) as u8;
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
