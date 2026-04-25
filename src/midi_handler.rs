use crate::fm_synth::SynthController;
use midir::{MidiInput, MidiInputConnection};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

/// Sentinel for OMNI mode — accept any channel.
const MIDI_OMNI: u8 = 0xFF;

pub struct MidiHandler {
    _connection: Option<MidiInputConnection<()>>,
    /// 0..15 = specific MIDI channel (1..16 to the user); MIDI_OMNI = listen on all.
    /// Shared with the midir callback so the GUI can change it without locking.
    channel_filter: Arc<AtomicU8>,
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

        let channel_filter = Arc::new(AtomicU8::new(MIDI_OMNI));
        let filter_for_callback = channel_filter.clone();

        let connection = midi_in.connect(
            port,
            "DX7 MIDI",
            move |_timestamp, message, _| {
                Self::handle_midi_message(&controller, message, &filter_for_callback);
            },
            (),
        )?;

        Ok(Self {
            _connection: Some(connection),
            channel_filter,
        })
    }

    /// Configure which MIDI channel to listen on. `None` selects OMNI mode (default).
    /// `Some(0..15)` accepts only that 0-indexed channel (MIDI ch 1 = 0).
    pub fn set_channel(&self, channel: Option<u8>) {
        let value = match channel {
            None => MIDI_OMNI,
            Some(ch) => ch.min(15),
        };
        self.channel_filter.store(value, Ordering::Relaxed);
    }

    /// Returns the current channel filter. `None` means OMNI.
    #[allow(dead_code)] // public API; GUI surfaces it via the channel selector
    pub fn channel(&self) -> Option<u8> {
        let raw = self.channel_filter.load(Ordering::Relaxed);
        if raw == MIDI_OMNI {
            None
        } else {
            Some(raw)
        }
    }

    fn handle_midi_message(
        controller: &Arc<Mutex<SynthController>>,
        message: &[u8],
        channel_filter: &Arc<AtomicU8>,
    ) {
        if message.is_empty() {
            return;
        }

        let status_full = message[0];

        // SysEx and System Common messages (0xF0..0xFF) carry no channel.
        if status_full < 0xF0 {
            let configured = channel_filter.load(Ordering::Relaxed);
            let msg_channel = status_full & 0x0F;
            if configured != MIDI_OMNI && configured != msg_channel {
                return;
            }
        }

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
                        0 => "Bank Select MSB",
                        1 => "Mod Wheel",
                        2 => "Breath Controller",
                        4 => "Foot Controller",
                        11 => "Expression",
                        32 => "Bank Select LSB",
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
                            0 => ctrl.set_bank_msb(value),
                            1 => ctrl.mod_wheel(value as f32 / 127.0),
                            2 => ctrl.breath_controller(value as f32 / 127.0),
                            4 => ctrl.foot_controller(value as f32 / 127.0),
                            11 => ctrl.expression(value as f32 / 127.0),
                            32 => ctrl.set_bank_lsb(value),
                            64 => ctrl.sustain_pedal(value >= 64),
                            123 => ctrl.panic(),
                            _ => {}
                        }
                    } else {
                        log::error!("Failed to acquire controller lock for control change");
                    }
                }
            }

            // Channel Aftertouch (0xD0) — 1 data byte (pressure 0-127).
            // DX7S routes this to PITCH/AMP/EG_BIAS/PITCH_BIAS via per-controller sensitivities.
            0xD0 => {
                if message.len() >= 2 {
                    let pressure = message[1];
                    log::debug!("Aftertouch Ch{} Pressure:{}", channel, pressure);
                    if let Ok(mut ctrl) = controller.lock() {
                        ctrl.aftertouch(pressure as f32 / 127.0);
                    } else {
                        log::error!("Failed to acquire controller lock for aftertouch");
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

            // Program Change (0xC0) - preset selection. The engine combines this
            // with the current Bank Select MSB/LSB (CC0/CC32) before resolving the
            // absolute preset index.
            0xC0 => {
                let program = message[1];
                log::info!("Program Change Ch{} Program:{}", channel, program);
                if let Ok(mut ctrl) = controller.lock() {
                    ctrl.program_change(program);
                } else {
                    log::error!("Failed to acquire controller lock for program change");
                }
            }

            // SysEx (0xF0) — pass the complete message to the parser.
            // Channel byte logging is meaningless here (SysEx has no channel),
            // so we just dispatch the raw payload.
            0xF0 => {
                Self::handle_sysex(controller, message);
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

    fn handle_sysex(controller: &Arc<Mutex<SynthController>>, message: &[u8]) {
        use crate::sysex::{parse_message, SysexResult};
        match parse_message(message) {
            Ok(SysexResult::SingleVoice(preset)) => {
                log::info!("SysEx: single voice '{}' received", preset.name);
                if let Ok(mut ctrl) = controller.lock() {
                    ctrl.load_sysex_single_voice(*preset);
                }
            }
            Ok(SysexResult::Bulk(presets)) => {
                log::info!("SysEx: bulk dump with {} voices received", presets.len());
                if let Ok(mut ctrl) = controller.lock() {
                    ctrl.load_sysex_bulk(presets);
                }
            }
            Err(e) => {
                log::warn!("SysEx parse error ({} bytes): {}", message.len(), e);
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
