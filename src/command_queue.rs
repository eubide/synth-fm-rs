use crate::presets::Dx7Preset;
use rtrb::{Consumer, Producer, RingBuffer};

/// Size of the command ring buffer.
/// 1024 commands should be more than enough for any realistic GUI/MIDI interaction.
const COMMAND_BUFFER_SIZE: usize = 1024;

/// Parameters that can be set on an operator
#[allow(dead_code)] // some variants are surfaced via JSON loader / future GUI panels
#[derive(Debug, Clone, Copy)]
pub enum OperatorParam {
    Ratio,
    Level,
    Detune,
    Feedback,
    VelocitySensitivity,
    KeyScaleRate,
    KeyScaleBreakpoint,
    KeyScaleLeftDepth,
    KeyScaleRightDepth,
    KeyScaleLeftCurve, // payload: encoded KeyScaleCurve (0..3)
    KeyScaleRightCurve,
    AmSensitivity,    // 0-3
    OscillatorKeySync,
    FixedFrequency,    // bool: 0 = ratio, 1 = fixed
    FixedFreqHz,
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

/// Parameters that can be set on the pitch envelope.
#[allow(dead_code)] // exposed via JSON loader; full GUI panel pending
#[derive(Debug, Clone, Copy)]
pub enum PitchEgParam {
    Enabled,
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
#[allow(dead_code)] // some variants are issued only by JSON preset loading / MIDI / future panels
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
    /// 0 = Poly, 1 = Mono (full portamento), 2 = Mono Legato (portamento only when previous note still held).
    SetVoiceMode(u8),
    SetPitchBendRange(f32),
    SetPortamentoEnable(bool),
    SetPortamentoTime(f32),
    SetPortamentoGlissando(bool), // step (semitone) glide instead of continuous
    SetTranspose(i8),              // -24..+24 semitones around C3
    SetPitchModSensitivity(u8),    // 0-7 PMS for the LFO pitch depth
    SetEgBiasSensitivity(u8),      // 0-7 mod-wheel routing depth for EG Bias (amp-side)
    SetPitchBiasSensitivity(u8),   // 0-7 mod-wheel routing depth for Pitch Bias (semitone offset)
    // DX7S Aftertouch (channel pressure 0xD0) routing: 4 destinations (0-7 each)
    SetAftertouchPitchSens(u8),
    SetAftertouchAmpSens(u8),
    SetAftertouchEgBiasSens(u8),
    SetAftertouchPitchBiasSens(u8),
    // DX7 Breath Controller (CC2) routing: 4 destinations (0-7 each)
    SetBreathPitchSens(u8),
    SetBreathAmpSens(u8),
    SetBreathEgBiasSens(u8),
    SetBreathPitchBiasSens(u8),
    // DX7S Foot Controller (CC4) routing: VOLUME (0-15) + 3 destinations (0-7 each)
    SetFootVolumeSens(u8),
    SetFootPitchSens(u8),
    SetFootAmpSens(u8),
    SetFootEgBiasSens(u8),

    // Real-time controllers
    PitchBend(i16),
    ModWheel(f32),
    SustainPedal(bool),
    /// DX7S channel aftertouch (0..1, mapped from MIDI 0xD0).
    Aftertouch(f32),
    /// DX7 Breath Controller value (0..1, mapped from MIDI CC2).
    BreathController(f32),
    /// DX7S Foot Controller value (0..1, mapped from MIDI CC4).
    FootController(f32),
    /// MIDI Expression (CC11) — generic global attenuator (1.0 = full level).
    Expression(f32),
    /// MIDI Bank Select MSB (CC0). Combined with LSB and the next Program Change
    /// to address banks beyond the original 128 DX7 presets.
    SetBankSelectMsb(u8),
    /// MIDI Bank Select LSB (CC32).
    SetBankSelectLsb(u8),
    /// MIDI Program Change (0xC0). Combined with the current bank to compute the
    /// preset index = (msb << 14 | lsb << 7 | program).
    ProgramChange(u8),

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

    // Pitch EG parameters
    SetPitchEgParam {
        param: PitchEgParam,
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

    /// Apply a preset parsed from a DX7 SysEx single-voice dump as the live edit
    /// buffer. The bank stays untouched.
    LoadSysExSingleVoice(Box<Dx7Preset>),

    /// Replace the entire 32-voice bank with a SysEx bulk dump.
    LoadSysExBulk(Vec<Dx7Preset>),

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
