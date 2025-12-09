use rtrb::{Consumer, Producer, RingBuffer};

/// Size of the command ring buffer.
/// 1024 commands should be more than enough for any realistic GUI/MIDI interaction.
const COMMAND_BUFFER_SIZE: usize = 1024;

/// Parameters that can be set on an operator
#[derive(Debug, Clone, Copy)]
pub enum OperatorParam {
    Ratio,
    Level,
    Detune,
    Feedback,
    VelocitySensitivity,
    KeyScaleLevel,
    KeyScaleRate,
    Enabled,
}

/// Parameters that can be set on an envelope
#[derive(Debug, Clone, Copy)]
pub enum EnvelopeParam {
    Rate1,
    Rate2,
    Rate3,
    Rate4,
    Level1,
    Level2,
    Level3,
    Level4,
}

/// Parameters that can be set on the LFO
#[derive(Debug, Clone, Copy)]
pub enum LfoParam {
    Rate,
    Delay,
    PitchDepth,
    AmpDepth,
    Waveform(u8), // 0-5 for different waveforms
    KeySync,
}

/// Effect types for effect parameter commands
#[derive(Debug, Clone, Copy)]
pub enum EffectType {
    Chorus,
    Delay,
    Reverb,
}

/// Parameters that can be set on effects
#[derive(Debug, Clone, Copy)]
pub enum EffectParam {
    // Common
    Enabled,
    Mix,

    // Chorus
    ChorusRate,
    ChorusDepth,
    ChorusFeedback,

    // Delay
    DelayTime,
    DelayFeedback,
    DelayPingPong,

    // Reverb
    ReverbRoomSize,
    ReverbDamping,
    ReverbWidth,
}

/// Commands sent from GUI/MIDI thread to audio thread
#[derive(Debug, Clone)]
pub enum SynthCommand {
    // Note events
    NoteOn {
        note: u8,
        velocity: u8,
    },
    NoteOff {
        note: u8,
    },

    // Global parameters
    SetAlgorithm(u8),
    SetMasterVolume(f32),
    SetMasterTune(f32),
    SetMonoMode(bool),
    SetPitchBendRange(f32),
    SetPortamentoEnable(bool),
    SetPortamentoTime(f32),

    // Real-time controllers
    PitchBend(i16),
    ModWheel(f32),
    SustainPedal(bool),

    // Operator parameters
    SetOperatorParam {
        operator: u8,
        param: OperatorParam,
        value: f32,
    },

    // Envelope parameters
    SetEnvelopeParam {
        operator: u8,
        param: EnvelopeParam,
        value: f32,
    },

    // LFO parameters
    SetLfoParam {
        param: LfoParam,
        value: f32,
    },

    // Effect parameters
    SetEffectParam {
        effect: EffectType,
        param: EffectParam,
        value: f32,
    },

    // Preset loading (for MIDI program change)
    LoadPreset(usize),

    // Voice initialization
    VoiceInitialize,

    // Panic - stop all sound
    Panic,
}

/// Sender side of the command queue (GUI/MIDI thread)
pub struct CommandSender {
    producer: Producer<SynthCommand>,
}

impl CommandSender {
    /// Send a command to the audio thread.
    /// Returns true if the command was sent, false if the buffer is full.
    pub fn send(&mut self, command: SynthCommand) -> bool {
        self.producer.push(command).is_ok()
    }

    /// Check how many slots are available in the buffer
    #[allow(dead_code)]
    pub fn available(&self) -> usize {
        self.producer.slots()
    }

    /// Check if the buffer is full
    #[allow(dead_code)]
    pub fn is_full(&self) -> bool {
        self.producer.is_full()
    }
}

/// Receiver side of the command queue (audio thread)
pub struct CommandReceiver {
    consumer: Consumer<SynthCommand>,
}

impl CommandReceiver {
    /// Try to receive a command from the GUI/MIDI thread.
    /// Returns None if no command is available.
    pub fn try_recv(&mut self) -> Option<SynthCommand> {
        self.consumer.pop().ok()
    }

    /// Process all pending commands with a callback.
    /// This is the recommended way to process commands in the audio callback.
    #[allow(dead_code)]
    pub fn process_all<F>(&mut self, mut callback: F)
    where
        F: FnMut(SynthCommand),
    {
        while let Some(cmd) = self.try_recv() {
            callback(cmd);
        }
    }

    /// Check how many commands are waiting
    #[allow(dead_code)]
    pub fn pending(&self) -> usize {
        self.consumer.slots()
    }

    /// Check if there are any pending commands
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.consumer.is_empty()
    }
}

/// Create a new command queue pair (sender, receiver)
pub fn create_command_queue() -> (CommandSender, CommandReceiver) {
    let (producer, consumer) = RingBuffer::new(COMMAND_BUFFER_SIZE);

    (CommandSender { producer }, CommandReceiver { consumer })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_receive() {
        let (mut sender, mut receiver) = create_command_queue();

        assert!(sender.send(SynthCommand::NoteOn {
            note: 60,
            velocity: 100
        }));
        assert!(sender.send(SynthCommand::NoteOff { note: 60 }));

        let cmd1 = receiver.try_recv().unwrap();
        match cmd1 {
            SynthCommand::NoteOn { note, velocity } => {
                assert_eq!(note, 60);
                assert_eq!(velocity, 100);
            }
            _ => panic!("Expected NoteOn"),
        }

        let cmd2 = receiver.try_recv().unwrap();
        match cmd2 {
            SynthCommand::NoteOff { note } => {
                assert_eq!(note, 60);
            }
            _ => panic!("Expected NoteOff"),
        }

        assert!(receiver.try_recv().is_none());
    }

    #[test]
    fn test_process_all() {
        let (mut sender, mut receiver) = create_command_queue();

        sender.send(SynthCommand::SetAlgorithm(5));
        sender.send(SynthCommand::SetMasterVolume(0.8));
        sender.send(SynthCommand::Panic);

        let mut count = 0;
        receiver.process_all(|_cmd| {
            count += 1;
        });

        assert_eq!(count, 3);
        assert!(receiver.is_empty());
    }

    #[test]
    fn test_buffer_capacity() {
        let (mut sender, mut receiver) = create_command_queue();

        // Fill the buffer
        for i in 0..COMMAND_BUFFER_SIZE {
            assert!(
                sender.send(SynthCommand::NoteOn {
                    note: (i % 128) as u8,
                    velocity: 100
                }),
                "Failed to send command {}",
                i
            );
        }

        // Buffer should be full now
        assert!(sender.is_full());
        assert!(!sender.send(SynthCommand::Panic));

        // Drain the buffer
        let mut count = 0;
        receiver.process_all(|_| count += 1);
        assert_eq!(count, COMMAND_BUFFER_SIZE);
    }

    #[test]
    fn test_operator_params() {
        let (mut sender, mut receiver) = create_command_queue();

        sender.send(SynthCommand::SetOperatorParam {
            operator: 0,
            param: OperatorParam::Ratio,
            value: 2.0,
        });

        let cmd = receiver.try_recv().unwrap();
        match cmd {
            SynthCommand::SetOperatorParam {
                operator,
                param,
                value,
            } => {
                assert_eq!(operator, 0);
                assert!(matches!(param, OperatorParam::Ratio));
                assert!((value - 2.0).abs() < 0.001);
            }
            _ => panic!("Expected SetOperatorParam"),
        }
    }
}
