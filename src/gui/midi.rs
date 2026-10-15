use super::{edit_slider, Dx7App};
use crate::fm_synth::SynthController;
use crate::presets::Dx7Preset;
use eframe::egui;

impl Dx7App {
    pub(super) fn draw_midi_panel(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new("MIDI / CONTROLLERS")
                        .size(14.0)
                        .strong(),
                );
                ui.separator();

                self.draw_device_section(ui);
                ui.add_space(6.0);
                self.draw_midi_channel_section(ui);
                ui.add_space(6.0);
                ui.separator();

                self.draw_aftertouch_routing(ui);
                ui.add_space(4.0);
                self.draw_breath_routing(ui);
                ui.add_space(4.0);
                self.draw_foot_routing(ui);

                ui.add_space(6.0);
                ui.separator();
                self.draw_sysex_section(ui);
            });
        });
    }

    fn draw_midi_channel_section(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("INPUT CHANNEL").strong());
            let label = match self.midi_channel_ui {
                None => "OMNI".to_string(),
                Some(c) => format!("Ch {}", c + 1),
            };
            egui::ComboBox::from_id_salt("midi_channel_combo")
                .selected_text(label)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(self.midi_channel_ui.is_none(), "OMNI (all channels)")
                        .clicked()
                    {
                        self.midi_channel_ui = None;
                        if let Some(handler) = self.midi_handler.as_ref() {
                            handler.set_channel(None);
                        }
                    }
                    for ch in 0u8..16 {
                        let selected = self.midi_channel_ui == Some(ch);
                        if ui
                            .selectable_label(selected, format!("Ch {}", ch + 1))
                            .clicked()
                        {
                            self.midi_channel_ui = Some(ch);
                            if let Some(handler) = self.midi_handler.as_ref() {
                                handler.set_channel(Some(ch));
                            }
                        }
                    }
                });
        });
    }

    fn draw_device_section(&mut self, ui: &mut egui::Ui) {
        let mut chosen_port = None;
        let mut chosen_output = None;
        egui::Grid::new("device_grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("INPUT PORT").strong());
                let connected = self
                    .midi_handler
                    .as_ref()
                    .and_then(|m| m.connected_port())
                    .unwrap_or("(none)")
                    .to_string();
                egui::ComboBox::from_id_salt("midi_port_combo")
                    .selected_text(&connected)
                    .width(280.0)
                    .show_ui(ui, |ui| {
                        if self.midi_ports.is_empty() {
                            ui.label("no MIDI inputs found");
                        }
                        for port in &self.midi_ports {
                            if ui.selectable_label(*port == connected, port).clicked() {
                                chosen_port = Some(port.clone());
                            }
                        }
                    });
                ui.end_row();

                ui.label(egui::RichText::new("AUDIO OUTPUT").strong());
                let current = self
                    .audio_engine
                    .as_ref()
                    .map_or("(none)", |a| a.device_name())
                    .to_string();
                egui::ComboBox::from_id_salt("audio_output_combo")
                    .selected_text(&current)
                    .width(280.0)
                    .show_ui(ui, |ui| {
                        for device in &self.audio_devices {
                            if ui.selectable_label(*device == current, device).clicked() {
                                chosen_output = Some(device.clone());
                            }
                        }
                    });
                ui.end_row();
            });

        if let (Some(port), Some(midi)) = (chosen_port, self.midi_handler.as_mut()) {
            midi.select_port(&port);
            self.device_status = if midi.connected_port() == Some(port.as_str()) {
                format!("MIDI input: {}", port)
            } else {
                format!("Could not open {}; will retry while it is listed", port)
            };
        }
        if let (Some(device), Some(audio)) = (chosen_output, self.audio_engine.as_mut()) {
            self.device_status = match audio.switch_to(&device) {
                Ok(()) => format!("Audio output: {}", device),
                Err(e) => format!("Audio output unchanged: {}", e),
            };
        }
        if !self.device_status.is_empty() {
            ui.label(
                egui::RichText::new(&self.device_status)
                    .size(11.0)
                    .color(egui::Color32::from_rgb(120, 120, 120)),
            );
        }
    }

    fn draw_aftertouch_routing(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("AFTERTOUCH (0xD0)")
                    .strong()
                    .color(egui::Color32::from_rgb(50, 90, 160)),
            );
            ui.label(format!("input: {:.0}%", self.snapshot.aftertouch * 100.0));
        });
        ui.horizontal(|ui| {
            self.routing_slider(
                ui,
                "PITCH",
                self.snapshot.aftertouch_pitch_sens,
                7,
                |ctrl, v| ctrl.set_aftertouch_pitch_sens(v),
            );
            self.routing_slider(
                ui,
                "AMP",
                self.snapshot.aftertouch_amp_sens,
                7,
                |ctrl, v| ctrl.set_aftertouch_amp_sens(v),
            );
            self.routing_slider(
                ui,
                "EG-BIAS",
                self.snapshot.aftertouch_eg_bias_sens,
                7,
                |ctrl, v| ctrl.set_aftertouch_eg_bias_sens(v),
            );
            self.routing_slider(
                ui,
                "P-BIAS",
                self.snapshot.aftertouch_pitch_bias_sens,
                7,
                |ctrl, v| ctrl.set_aftertouch_pitch_bias_sens(v),
            );
        });
    }

    fn draw_breath_routing(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("BREATH CTRL (CC2)")
                    .strong()
                    .color(egui::Color32::from_rgb(50, 90, 160)),
            );
            ui.label(format!("input: {:.0}%", self.snapshot.breath * 100.0));
        });
        ui.horizontal(|ui| {
            self.routing_slider(
                ui,
                "PITCH",
                self.snapshot.breath_pitch_sens,
                7,
                |ctrl, v| ctrl.set_breath_pitch_sens(v),
            );
            self.routing_slider(ui, "AMP", self.snapshot.breath_amp_sens, 7, |ctrl, v| {
                ctrl.set_breath_amp_sens(v)
            });
            self.routing_slider(
                ui,
                "EG-BIAS",
                self.snapshot.breath_eg_bias_sens,
                7,
                |ctrl, v| ctrl.set_breath_eg_bias_sens(v),
            );
            self.routing_slider(
                ui,
                "P-BIAS",
                self.snapshot.breath_pitch_bias_sens,
                7,
                |ctrl, v| ctrl.set_breath_pitch_bias_sens(v),
            );
        });
    }

    fn draw_foot_routing(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("FOOT CTRL (CC4)")
                    .strong()
                    .color(egui::Color32::from_rgb(50, 90, 160)),
            );
            ui.label(format!("input: {:.0}%", self.snapshot.foot * 100.0));
        });
        ui.horizontal(|ui| {
            // VOLUME has 0-15 range on the DX7S, the rest are 0-7.
            self.routing_slider(
                ui,
                "VOLUME",
                self.snapshot.foot_volume_sens,
                15,
                |ctrl, v| ctrl.set_foot_volume_sens(v),
            );
            self.routing_slider(ui, "PITCH", self.snapshot.foot_pitch_sens, 7, |ctrl, v| {
                ctrl.set_foot_pitch_sens(v)
            });
            self.routing_slider(ui, "AMP", self.snapshot.foot_amp_sens, 7, |ctrl, v| {
                ctrl.set_foot_amp_sens(v)
            });
            self.routing_slider(
                ui,
                "EG-BIAS",
                self.snapshot.foot_eg_bias_sens,
                7,
                |ctrl, v| ctrl.set_foot_eg_bias_sens(v),
            );
        });
    }

    /// Render a labelled 0..max integer slider for a routing destination.
    /// `apply` is called with the new value when the user changes it.
    fn routing_slider<F>(&self, ui: &mut egui::Ui, label: &str, value: u8, max: u8, mut apply: F)
    where
        F: FnMut(&mut SynthController, u8),
    {
        ui.vertical(|ui| {
            ui.label(label);
            let mut v = value as i32;
            if ui
                .add(edit_slider(&mut v, 0..=max as i32).show_value(true))
                .changed()
            {
                if let Ok(mut ctrl) = self.lock_controller() {
                    apply(&mut ctrl, v.clamp(0, max as i32) as u8);
                }
            }
        });
    }

    fn draw_sysex_section(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("SYSEX (DX7 voice exchange)").strong());
        ui.horizontal(|ui| {
            if ui.button("Load .syx…").clicked() {
                if let Some(path) = self.sysex_dialog().pick_file() {
                    self.sysex_path = path.to_string_lossy().into_owned();
                    self.load_sysex_from_path();
                }
            }
            if ui.button("Save voice as .syx…").clicked() {
                let file_name = format!(
                    "{}.syx",
                    crate::preset_loader::file_stem_for(&self.voice_name)
                );
                if let Some(path) = self.sysex_dialog().set_file_name(file_name).save_file() {
                    self.sysex_path = path.to_string_lossy().into_owned();
                    self.save_sysex_to_path();
                }
            }
        });
        if !self.sysex_status.is_empty() {
            ui.label(
                egui::RichText::new(&self.sysex_status)
                    .size(11.0)
                    .color(egui::Color32::from_rgb(120, 120, 120)),
            );
        }
    }

    fn sysex_dialog(&self) -> rfd::FileDialog {
        let dialog = rfd::FileDialog::new().add_filter("DX7 SysEx", &["syx", "SYX"]);
        match std::path::Path::new(&self.sysex_path).parent() {
            Some(dir) if dir.is_dir() => dialog.set_directory(dir),
            _ => dialog,
        }
    }

    pub(super) fn load_sysex_from_path(&mut self) {
        let path = self.sysex_path.trim().to_string();
        let collection = std::path::Path::new(&path)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "SysEx".to_string());
        match std::fs::read(&path) {
            Ok(bytes) => match crate::sysex::parse_message(&bytes) {
                Ok(crate::sysex::SysexResult::SingleVoice(mut preset)) => {
                    let name = preset.name.clone();
                    preset.collection = collection.clone();
                    self.replace_collection(&collection, vec![(*preset).clone()]);
                    if let Ok(mut ctrl) = self.lock_controller() {
                        ctrl.load_voice(*preset);
                    }
                    self.sysex_status = format!("Loaded single voice '{}' from {}", name, path);
                }
                Ok(crate::sysex::SysexResult::Bulk(mut presets)) => {
                    let count = presets.len();
                    for p in &mut presets {
                        p.collection = collection.clone();
                    }
                    self.replace_collection(&collection, presets.clone());
                    if let Ok(mut ctrl) = self.lock_controller() {
                        ctrl.load_sysex_bulk(presets);
                    }
                    self.sysex_status =
                        format!("Loaded bulk dump ({} voices) from {}", count, path);
                }
                Err(e) => {
                    self.sysex_status = format!("Parse error: {}", e);
                }
            },
            Err(e) => {
                self.sysex_status = format!("Read error ({}): {}", path, e);
            }
        }
    }

    pub(super) fn save_sysex_to_path(&mut self) {
        let path = self.sysex_path.trim().to_string();
        let mut preset = Dx7Preset::from_snapshot(&self.snapshot);
        preset.name = self.voice_name.trim().to_string();
        let channel = self.midi_channel_ui.unwrap_or(0);
        let bytes = crate::sysex::encode_single_voice(&preset, channel);
        match std::fs::write(&path, &bytes) {
            Ok(_) => {
                self.sysex_status = format!(
                    "Saved '{}' ({} bytes) to {}",
                    preset.name,
                    bytes.len(),
                    path
                );
            }
            Err(e) => {
                self.sysex_status = format!("Write error ({}): {}", path, e);
            }
        }
    }
}
