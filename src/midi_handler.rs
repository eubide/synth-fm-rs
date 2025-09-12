use crate::fm_synth::FmSynthesizer;
use midir::{MidiInput, MidiInputConnection};
use std::sync::{Arc, Mutex};

pub struct MidiHandler {
    _connection: Option<MidiInputConnection<()>>,
}

impl MidiHandler {
    pub fn new(synthesizer: Arc<Mutex<FmSynthesizer>>) -> Result<Self, Box<dyn std::error::Error>> {
        let midi_in = MidiInput::new("DX7 MIDI Input")?;

        let ports = midi_in.ports();
        if ports.is_empty() {
            return Err("No MIDI input devices found".into());
        }

        println!("Available MIDI inputs:");
        for (i, port) in ports.iter().enumerate() {
            println!("  {}: {}", i, midi_in.port_name(port)?);
        }

        let port = &ports[0];
        println!("Using MIDI input: {}", midi_in.port_name(port)?);

        let connection = midi_in.connect(
            port,
            "DX7 MIDI",
            move |_timestamp, message, _| {
                Self::handle_midi_message(&synthesizer, message);
            },
            (),
        )?;

        Ok(Self {
            _connection: Some(connection),
        })
    }

    fn handle_midi_message(synthesizer: &Arc<Mutex<FmSynthesizer>>, message: &[u8]) {
        if message.len() < 2 {
            return;
        }

        let status = message[0] & 0xF0;
        let _channel = message[0] & 0x0F;

        match status {
            0x90 => {
                if message.len() >= 3 {
                    let note = message[1];
                    let velocity = message[2];

                    let mut synth = synthesizer.lock().unwrap();
                    if velocity > 0 {
                        synth.note_on(note, velocity);
                    } else {
                        synth.note_off(note);
                    }
                }
            }

            0x80 => {
                if message.len() >= 3 {
                    let note = message[1];
                    let mut synth = synthesizer.lock().unwrap();
                    synth.note_off(note);
                }
            }

            0xB0 => {
                if message.len() >= 3 {
                    let controller = message[1];
                    let value = message[2];
                    let mut synth = synthesizer.lock().unwrap();
                    synth.control_change(controller, value);
                }
            }

            0xE0 => {
                if message.len() >= 3 {
                    let lsb = message[1] as i16;
                    let msb = message[2] as i16;
                    let value = (msb << 7) | lsb - 8192;
                    let mut synth = synthesizer.lock().unwrap();
                    synth.pitch_bend(value);
                }
            }

            _ => {}
        }
    }
}
