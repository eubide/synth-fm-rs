use crate::fm_synth::SynthController;
use midir::{MidiInput, MidiInputConnection};
use std::sync::{Arc, Mutex};

pub struct MidiHandler {
    _connection: Option<MidiInputConnection<()>>,
}

impl MidiHandler {
    pub fn new(
        controller: Arc<Mutex<SynthController>>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let midi_in = MidiInput::new("DX7 MIDI Input")?;

        let ports = midi_in.ports();
        if ports.is_empty() {
            return Err("No MIDI input devices found".into());
        }

        log::info!("Available MIDI inputs:");
        for (i, port) in ports.iter().enumerate() {
            log::info!("  {}: {}", i, midi_in.port_name(port)?);
        }

        let port = &ports[0];
        log::info!("Using MIDI input: {}", midi_in.port_name(port)?);

        let connection = midi_in.connect(
            port,
            "DX7 MIDI",
            move |_timestamp, message, _| {
                Self::handle_midi_message(&controller, message);
            },
            (),
        )?;

        Ok(Self {
            _connection: Some(connection),
        })
    }

    fn handle_midi_message(controller: &Arc<Mutex<SynthController>>, message: &[u8]) {
        if message.len() < 2 {
            return;
        }

        let status = message[0] & 0xF0;
        let channel = (message[0] & 0x0F) + 1;

        match status {
            0x90 => {
                if message.len() >= 3 {
                    let note = message[1];
                    let velocity = message[2];

                    if velocity > 0 {
                        log::debug!(
                            "Note ON Ch{} Note:{} ({}) Vel:{}",
                            channel,
                            note,
                            Self::note_name(note),
                            velocity
                        );
                        if let Ok(mut ctrl) = controller.lock() {
                            ctrl.note_on(note, velocity);
                        } else {
                            log::error!("Failed to acquire controller lock for note on");
                        }
                    } else {
                        log::debug!(
                            "Note OFF Ch{} Note:{} ({}) (via vel=0)",
                            channel,
                            note,
                            Self::note_name(note)
                        );
                        if let Ok(mut ctrl) = controller.lock() {
                            ctrl.note_off(note);
                        } else {
                            log::error!("Failed to acquire controller lock for note off");
                        }
                    }
                }
            }

            0x80 => {
                if message.len() >= 3 {
                    let note = message[1];
                    log::debug!(
                        "Note OFF Ch{} Note:{} ({})",
                        channel,
                        note,
                        Self::note_name(note)
                    );
                    if let Ok(mut ctrl) = controller.lock() {
                        ctrl.note_off(note);
                    } else {
                        log::error!("Failed to acquire controller lock for note off");
                    }
                }
            }

            0xB0 => {
                if message.len() >= 3 {
                    let controller_num = message[1];
                    let value = message[2];

                    let cc_name = match controller_num {
                        1 => "Mod Wheel",
                        64 => "Sustain Pedal",
                        123 => "All Notes Off",
                        _ => "Unknown CC",
                    };

                    log::debug!(
                        "Control Change Ch{} CC{} ({}) Value:{}",
                        channel,
                        controller_num,
                        cc_name,
                        value
                    );
                    if let Ok(mut ctrl) = controller.lock() {
                        match controller_num {
                            1 => ctrl.mod_wheel(value as f32 / 127.0),
                            64 => ctrl.sustain_pedal(value >= 64),
                            123 => ctrl.panic(),
                            _ => {}
                        }
                    } else {
                        log::error!("Failed to acquire controller lock for control change");
                    }
                }
            }

            0xE0 => {
                if message.len() >= 3 {
                    let lsb = message[1] as i16;
                    let msb = message[2] as i16;
                    let value = ((msb << 7) | lsb) - 8192;
                    log::debug!("Pitch Bend Ch{} Value:{}", channel, value);
                    if let Ok(mut ctrl) = controller.lock() {
                        ctrl.pitch_bend(value);
                    } else {
                        log::error!("Failed to acquire controller lock for pitch bend");
                    }
                }
            }

            _ => {
                log::debug!(
                    "Unknown MIDI message: Status:0x{:02X} Ch{}",
                    status,
                    channel
                );
            }
        }
    }

    fn note_name(note: u8) -> String {
        let notes = [
            "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
        ];
        let octave = (note / 12) as i32 - 1;
        let note_index = note % 12;
        format!("{}{}", notes[note_index as usize], octave)
    }
}

impl Drop for MidiHandler {
    fn drop(&mut self) {
        if self._connection.is_some() {
            log::info!("MIDI connection closed");
        }
    }
}
