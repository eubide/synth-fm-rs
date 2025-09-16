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

        // Log MIDI message to console
        print!("MIDI: [");
        for (i, byte) in message.iter().enumerate() {
            if i > 0 {
                print!(", ");
            }
            print!("{:02X}", byte);
        }
        print!("] -> ");

        let status = message[0] & 0xF0;
        let channel = message[0] & 0x0F;

        match status {
            0x90 => {
                if message.len() >= 3 {
                    let note = message[1];
                    let velocity = message[2];

                    if velocity > 0 {
                        println!("Note ON Ch{} Note:{} Vel:{}", channel + 1, note, velocity);
                    } else {
                        println!("Note OFF Ch{} Note:{} (via vel=0)", channel + 1, note);
                    }

                    if let Ok(mut synth) = synthesizer.lock() {
                        if velocity > 0 {
                            synth.note_on(note, velocity);
                        } else {
                            synth.note_off(note);
                        }
                    } else {
                        eprintln!("Failed to acquire synth lock for note on/off");
                    }
                }
            }

            0x80 => {
                if message.len() >= 3 {
                    let note = message[1];
                    println!("Note OFF Ch{} Note:{}", channel + 1, note);
                    if let Ok(mut synth) = synthesizer.lock() {
                        synth.note_off(note);
                    } else {
                        eprintln!("Failed to acquire synth lock for note off");
                    }
                }
            }

            0xB0 => {
                if message.len() >= 3 {
                    let controller = message[1];
                    let value = message[2];

                    let cc_name = match controller {
                        1 => "Mod Wheel",
                        64 => "Sustain Pedal",
                        123 => "All Notes Off",
                        _ => "Unknown CC",
                    };

                    println!(
                        "Control Change Ch{} CC{} ({}) Value:{}",
                        channel + 1,
                        controller,
                        cc_name,
                        value
                    );
                    if let Ok(mut synth) = synthesizer.lock() {
                        synth.control_change(controller, value);
                    } else {
                        eprintln!("Failed to acquire synth lock for control change");
                    }
                }
            }

            0xE0 => {
                if message.len() >= 3 {
                    let lsb = message[1] as i16;
                    let msb = message[2] as i16;
                    let value = ((msb << 7) | lsb) - 8192;
                    println!(
                        "Pitch Bend Ch{} Value:{} (14-bit: LSB:{} MSB:{})",
                        channel + 1,
                        value,
                        lsb,
                        msb
                    );
                    if let Ok(mut synth) = synthesizer.lock() {
                        synth.pitch_bend(value);
                    } else {
                        eprintln!("Failed to acquire synth lock for pitch bend");
                    }
                }
            }

            _ => {
                println!(
                    "Unknown MIDI message: Status:0x{:02X} Ch{}",
                    status,
                    channel + 1
                );
            }
        }
    }
}
