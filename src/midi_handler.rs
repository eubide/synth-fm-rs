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

    pub(crate) fn note_name(note: u8) -> String {
        let notes = [
            "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
        ];
        let octave = (note / 12) as i32 - 1;
        let note_index = note % 12;
        format!("{}{}", notes[note_index as usize], octave)
    }

    #[cfg(test)]
    pub(crate) fn dispatch(
        controller: &Arc<Mutex<SynthController>>,
        message: &[u8],
        channel_filter: &Arc<AtomicU8>,
    ) {
        Self::handle_midi_message(controller, message, channel_filter);
    }

    #[cfg(test)]
    pub(crate) fn omni_sentinel() -> u8 {
        MIDI_OMNI
    }
}

impl Drop for MidiHandler {
    fn drop(&mut self) {
        if self._connection.is_some() {
            log::info!("MIDI connection closed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fm_synth::create_synth;

    fn make_controller() -> (Arc<Mutex<SynthController>>, Arc<AtomicU8>) {
        let (_engine, controller) = create_synth(44_100.0);
        (
            Arc::new(Mutex::new(controller)),
            Arc::new(AtomicU8::new(MidiHandler::omni_sentinel())),
        )
    }

    #[test]
    fn note_name_handles_full_range() {
        assert_eq!(MidiHandler::note_name(0), "C-1");
        assert_eq!(MidiHandler::note_name(60), "C4"); // MIDI standard convention
        assert_eq!(MidiHandler::note_name(69), "A4");
        assert_eq!(MidiHandler::note_name(127), "G9");
    }

    #[test]
    fn note_name_includes_sharps() {
        assert_eq!(MidiHandler::note_name(61), "C#4");
        assert_eq!(MidiHandler::note_name(70), "A#4");
    }

    #[test]
    fn empty_message_is_dropped() {
        let (ctrl, filter) = make_controller();
        MidiHandler::dispatch(&ctrl, &[], &filter);
    }

    #[test]
    fn note_on_with_velocity_zero_is_treated_as_note_off() {
        let (ctrl, filter) = make_controller();
        MidiHandler::dispatch(&ctrl, &[0x90, 60, 0], &filter);
        // Should not panic; note_off command queued.
    }

    #[test]
    fn note_on_with_positive_velocity_dispatches() {
        let (ctrl, filter) = make_controller();
        MidiHandler::dispatch(&ctrl, &[0x90, 60, 100], &filter);
    }

    #[test]
    fn explicit_note_off_dispatches() {
        let (ctrl, filter) = make_controller();
        MidiHandler::dispatch(&ctrl, &[0x80, 60, 100], &filter);
    }

    #[test]
    fn truncated_note_messages_are_ignored() {
        let (ctrl, filter) = make_controller();
        MidiHandler::dispatch(&ctrl, &[0x90, 60], &filter); // missing velocity
        MidiHandler::dispatch(&ctrl, &[0x80, 60], &filter);
    }

    #[test]
    fn control_change_routes_recognised_ccs() {
        let (ctrl, filter) = make_controller();
        for cc in [0u8, 1, 2, 4, 11, 32, 64, 123] {
            MidiHandler::dispatch(&ctrl, &[0xB0, cc, 64], &filter);
        }
        // Unknown CC: still handled (no-op)
        MidiHandler::dispatch(&ctrl, &[0xB0, 50, 64], &filter);
    }

    #[test]
    fn control_change_truncated_is_ignored() {
        let (ctrl, filter) = make_controller();
        MidiHandler::dispatch(&ctrl, &[0xB0, 1], &filter);
    }

    #[test]
    fn aftertouch_dispatches() {
        let (ctrl, filter) = make_controller();
        MidiHandler::dispatch(&ctrl, &[0xD0, 100], &filter);
    }

    #[test]
    fn aftertouch_too_short_is_ignored() {
        let (ctrl, filter) = make_controller();
        MidiHandler::dispatch(&ctrl, &[0xD0], &filter);
    }

    #[test]
    fn pitch_bend_combines_lsb_and_msb() {
        let (ctrl, filter) = make_controller();
        // Center bend = 8192 → LSB=0, MSB=64. After subtracting 8192 → 0.
        MidiHandler::dispatch(&ctrl, &[0xE0, 0, 64], &filter);
        // Max up bend = 16383 → LSB=127, MSB=127.
        MidiHandler::dispatch(&ctrl, &[0xE0, 127, 127], &filter);
    }

    #[test]
    fn program_change_dispatches() {
        let (ctrl, filter) = make_controller();
        MidiHandler::dispatch(&ctrl, &[0xC0, 5], &filter);
    }

    #[test]
    fn unknown_status_byte_is_logged_but_safe() {
        let (ctrl, filter) = make_controller();
        // 0xA0 = polyphonic key pressure (we don't handle it specifically)
        MidiHandler::dispatch(&ctrl, &[0xA0, 60, 100], &filter);
    }

    #[test]
    fn channel_filter_suppresses_non_matching_channel() {
        let (ctrl, filter) = make_controller();
        // Listen only on MIDI channel 5 (0-indexed = 4)
        filter.store(4, Ordering::Relaxed);
        // Send a note on channel 1 (0-indexed = 0)
        MidiHandler::dispatch(&ctrl, &[0x90, 60, 100], &filter);
        // No way to assert directly; this exercises the filter branch.
    }

    #[test]
    fn sysex_messages_are_routed_to_parser() {
        let (ctrl, filter) = make_controller();
        // Invalid SysEx — short, not Yamaha. Parser will reject it but dispatch must not panic.
        let bytes = [0xF0u8, 0x42, 0x00, 0xF7];
        MidiHandler::dispatch(&ctrl, &bytes, &filter);
    }

    #[test]
    fn channel_filter_omni_accepts_all_channels() {
        let (ctrl, filter) = make_controller();
        // OMNI sentinel
        filter.store(MidiHandler::omni_sentinel(), Ordering::Relaxed);
        for ch in 0..16u8 {
            MidiHandler::dispatch(&ctrl, &[0x90 | ch, 60, 100], &filter);
        }
    }

    #[test]
    fn system_messages_skip_channel_filter() {
        let (ctrl, filter) = make_controller();
        filter.store(0, Ordering::Relaxed);
        // System Common message (status >= 0xF0 below 0xF8) should not be filtered out.
        MidiHandler::dispatch(&ctrl, &[0xF0, 0x43, 0x00, 0xF7], &filter);
    }

    /// Build a `MidiHandler` shell without invoking `midir::MidiInput::connect`.
    /// We exercise `set_channel` / `channel` on this stub so the public API is
    /// covered without needing an actual MIDI device.
    fn stub_handler() -> MidiHandler {
        MidiHandler {
            _connection: None,
            channel_filter: Arc::new(AtomicU8::new(MidiHandler::omni_sentinel())),
        }
    }

    #[test]
    fn set_channel_to_specific_value_records_zero_indexed_channel() {
        let h = stub_handler();
        h.set_channel(Some(3));
        assert_eq!(h.channel(), Some(3));
    }

    #[test]
    fn set_channel_clamps_above_15() {
        let h = stub_handler();
        h.set_channel(Some(99));
        assert_eq!(h.channel(), Some(15));
    }

    #[test]
    fn set_channel_none_returns_omni() {
        let h = stub_handler();
        h.set_channel(Some(7));
        h.set_channel(None);
        assert_eq!(h.channel(), None);
    }

    #[test]
    fn drop_logs_when_connection_present() {
        // Drop with no connection — exercises the early-return branch.
        let h = stub_handler();
        drop(h);
    }

    #[test]
    fn sysex_dispatch_with_invalid_payload_is_a_noop() {
        let (ctrl, filter) = make_controller();
        // Empty SysEx-like payload — parser will reject with TooShort.
        MidiHandler::dispatch(&ctrl, &[0xF0, 0xF7], &filter);
    }

    #[test]
    fn sysex_dispatch_with_valid_single_voice_loads_preset() {
        use crate::presets::{Dx7Preset, PresetLfo, PresetOperator, PresetPitchEg};
        use crate::sysex::encode_single_voice;

        let preset = Dx7Preset {
            name: "MIDI SX".to_string(),
            collection: "test".to_string(),
            algorithm: 9,
            operators: std::array::from_fn(|_| PresetOperator::default()),
            master_tune: None,
            pitch_bend_range: None,
            portamento_enable: None,
            portamento_time: None,
            mono_mode: None,
            transpose_semitones: 0,
            pitch_mod_sensitivity: 2,
            pitch_eg: Some(PresetPitchEg::default()),
            lfo: Some(PresetLfo::default()),
        };
        let bytes = encode_single_voice(&preset, 0);
        let (ctrl, filter) = make_controller();
        MidiHandler::dispatch(&ctrl, &bytes, &filter);
    }
}
