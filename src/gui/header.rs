use super::{edit_slider, DisplayMode, Dx7App};
use eframe::egui;

impl Dx7App {
    pub(super) fn draw_dx7_display(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            // Light background like classic LCD
            ui.style_mut().visuals.widgets.noninteractive.bg_fill =
                egui::Color32::from_rgb(230, 240, 235);
            ui.style_mut()
                .visuals
                .widgets
                .noninteractive
                .fg_stroke
                .color = egui::Color32::from_rgb(30, 30, 30);

            ui.set_min_height(80.0);
            ui.vertical_centered(|ui| {
                ui.add_space(5.0);

                let display_font = egui::FontId::new(16.0, egui::FontFamily::Monospace);
                let small_font = egui::FontId::new(12.0, egui::FontFamily::Monospace);
                let display_color = egui::Color32::from_rgb(30, 30, 30);

                // Main display text (current mode)
                ui.label(
                    egui::RichText::new(&self.display_text)
                        .font(display_font.clone())
                        .color(display_color),
                );

                // Mode-specific sub text (using cached snapshot)
                let sub_text = match self.display_mode {
                    DisplayMode::Voice => {
                        format!(
                            "VOICE: {} | ALG: {:02}",
                            self.snapshot.preset_name, self.snapshot.algorithm
                        )
                    }
                    DisplayMode::Operator => {
                        format!("OP{} EDIT", self.selected_operator + 1)
                    }
                    DisplayMode::LFO => {
                        let waveform_name = self.snapshot.lfo_waveform.name();
                        format!(
                            "LFO: {} | Rate: {:.0} | Mod: {:.0}%",
                            waveform_name,
                            self.snapshot.lfo_rate,
                            self.snapshot.mod_wheel * 100.0
                        )
                    }
                    DisplayMode::Effects => {
                        let chorus = if self.snapshot.chorus.enabled {
                            "CHO"
                        } else {
                            "-"
                        };
                        let delay = if self.snapshot.delay.enabled {
                            "DLY"
                        } else {
                            "-"
                        };
                        let reverb = if self.snapshot.reverb.enabled {
                            "REV"
                        } else {
                            "-"
                        };
                        format!("EFFECTS: {} {} {}", chorus, delay, reverb)
                    }
                    DisplayMode::Midi => {
                        let ch_text = match self.midi_channel_ui {
                            None => "OMNI".to_string(),
                            Some(c) => format!("CH {}", c + 1),
                        };
                        format!(
                            "MIDI: {} | AT:{:.0}% BR:{:.0}% FT:{:.0}%",
                            ch_text,
                            self.snapshot.aftertouch * 100.0,
                            self.snapshot.breath * 100.0,
                            self.snapshot.foot * 100.0
                        )
                    }
                };

                ui.label(
                    egui::RichText::new(sub_text)
                        .font(display_font)
                        .color(display_color),
                );

                ui.add_space(5.0);
                ui.separator();

                // Always display current status information (from snapshot)
                let mode_text = match self.snapshot.voice_mode {
                    crate::state_snapshot::VoiceMode::Poly => "POLY",
                    crate::state_snapshot::VoiceMode::Mono => "MONO",
                    crate::state_snapshot::VoiceMode::MonoLegato => "M-LEG",
                };
                let midi_connected = self
                    .midi_handler
                    .as_ref()
                    .is_some_and(|m| m.connected_port().is_some());
                let midi_text = if midi_connected { "MIDI OK" } else { "NO MIDI" };

                let edited_mark = if self.snapshot.voice_edited { "*" } else { "" };
                let is_mono = self.snapshot.voice_mode != crate::state_snapshot::VoiceMode::Poly;
                let status_line = if is_mono {
                    // Show portamento only in MONO modes
                    let porta_text = if self.snapshot.portamento_enable {
                        "ON"
                    } else {
                        "OFF"
                    };
                    format!(
                        "VOICE: {}{} | ALG: {:02} | MODE: {} | PORTA: {} | {}",
                        self.snapshot.preset_name,
                        edited_mark,
                        self.snapshot.algorithm,
                        mode_text,
                        porta_text,
                        midi_text
                    )
                } else {
                    // In POLY mode, don't show portamento
                    format!(
                        "VOICE: {}{} | ALG: {:02} | MODE: {} | {}",
                        self.snapshot.preset_name,
                        edited_mark,
                        self.snapshot.algorithm,
                        mode_text,
                        midi_text
                    )
                };

                ui.label(
                    egui::RichText::new(status_line)
                        .font(small_font)
                        .color(display_color),
                );
            });
        });
    }

    pub(super) fn draw_global_controls(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            // Light gray background for global panel
            ui.style_mut().visuals.widgets.noninteractive.bg_fill =
                egui::Color32::from_rgb(245, 245, 245);

            let available_width = ui.available_width();
            let is_narrow = available_width < 800.0;

            ui.set_min_height(if is_narrow { 100.0 } else { 60.0 });

            if is_narrow {
                // Vertical layout for narrow windows
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("GLOBAL CONTROLS").size(10.0).strong());

                    // First row: Volume and Mode
                    ui.horizontal(|ui| {
                        // Volume section
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label("MASTER VOL:");
                                let mut volume = self.snapshot.master_volume;
                                let slider_response =
                                    ui.add(edit_slider(&mut volume, 0.0..=1.0).show_value(false));
                                if slider_response.changed() {
                                    if let Ok(mut ctrl) = self.lock_controller() {
                                        ctrl.set_master_volume(volume);
                                    }
                                }
                                ui.label(format!("{:.0}", self.snapshot.master_volume * 100.0));
                            });
                        });

                        ui.separator();

                        // Mode section
                        ui.vertical(|ui| {
                            self.draw_mode_controls_compact(ui);
                        });
                    });

                    // Second row: Tune and utilities
                    ui.horizontal(|ui| {
                        self.draw_tune_and_utilities_compact(ui);
                    });
                });
            } else {
                // Horizontal layout for wide windows
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("GLOBAL CONTROLS").size(10.0).strong());

                    // First row: Volume, Tuning, Mode, Panic/Init
                    ui.horizontal(|ui| {
                        // Left section: Volume
                        ui.vertical(|ui| {
                            ui.set_min_width(120.0);
                            ui.horizontal(|ui| {
                                ui.label("MASTER VOL:");
                                let mut volume = self.snapshot.master_volume;
                                if ui
                                    .add(edit_slider(&mut volume, 0.0..=1.0).show_value(false))
                                    .changed()
                                {
                                    if let Ok(mut ctrl) = self.lock_controller() {
                                        ctrl.set_master_volume(volume);
                                    }
                                }
                                ui.label(format!("{:.0}", self.snapshot.master_volume * 100.0));
                            });
                        });

                        ui.separator();

                        // Center-left section: Tuning controls
                        ui.vertical(|ui| {
                            ui.set_min_width(180.0);
                            // Master Tune
                            ui.horizontal(|ui| {
                                ui.label("MASTER TUNE:");
                                let mut master_tune = self.snapshot.master_tune;
                                if ui
                                    .add(
                                        edit_slider(&mut master_tune, -150.0..=150.0)
                                            .show_value(false),
                                    )
                                    .changed()
                                {
                                    if let Ok(mut ctrl) = self.lock_controller() {
                                        ctrl.set_master_tune(master_tune);
                                    }
                                }
                                ui.label(format!("{:.0}c", self.snapshot.master_tune));
                                if ui.small_button("RST").clicked() {
                                    if let Ok(mut ctrl) = self.lock_controller() {
                                        ctrl.set_master_tune(0.0);
                                    }
                                }
                            });

                            // Pitch Bend Range
                            ui.horizontal(|ui| {
                                ui.label("PITCH BEND:");
                                let mut pb_range = self.snapshot.pitch_bend_range;
                                if ui
                                    .add(edit_slider(&mut pb_range, 0.0..=12.0).show_value(false))
                                    .changed()
                                {
                                    if let Ok(mut ctrl) = self.lock_controller() {
                                        ctrl.set_pitch_bend_range(pb_range);
                                    }
                                }
                                ui.label(format!("{:.0}", self.snapshot.pitch_bend_range));
                            });
                        });

                        ui.separator();

                        // Center-right section: Mode controls
                        ui.vertical(|ui| {
                            ui.set_min_width(180.0);
                            let voice_mode = self.snapshot.voice_mode;
                            let is_mono = voice_mode != crate::state_snapshot::VoiceMode::Poly;
                            let porta_enable = self.snapshot.portamento_enable;
                            let porta_time = self.snapshot.portamento_time;

                            ui.horizontal(|ui| {
                                ui.label("MODE:");
                                let mut mode = voice_mode;
                                use crate::state_snapshot::VoiceMode;
                                if ui
                                    .selectable_value(&mut mode, VoiceMode::Poly, "POLY")
                                    .clicked()
                                    && voice_mode != VoiceMode::Poly
                                {
                                    if let Ok(mut ctrl) = self.lock_controller() {
                                        ctrl.set_voice_mode(VoiceMode::Poly);
                                    }
                                }
                                if ui
                                    .selectable_value(&mut mode, VoiceMode::Mono, "MONO")
                                    .clicked()
                                    && voice_mode != VoiceMode::Mono
                                {
                                    if let Ok(mut ctrl) = self.lock_controller() {
                                        ctrl.set_voice_mode(VoiceMode::Mono);
                                    }
                                }
                                if ui
                                    .selectable_value(&mut mode, VoiceMode::MonoLegato, "M-LEG")
                                    .clicked()
                                    && voice_mode != VoiceMode::MonoLegato
                                {
                                    if let Ok(mut ctrl) = self.lock_controller() {
                                        ctrl.set_voice_mode(VoiceMode::MonoLegato);
                                    }
                                }
                            });

                            // Portamento (only visible in MONO modes)
                            if is_mono {
                                ui.horizontal(|ui| {
                                    ui.label("PORTAMENTO:");
                                    let mut porta_on = porta_enable;
                                    if ui.checkbox(&mut porta_on, "").changed() {
                                        if let Ok(mut ctrl) = self.lock_controller() {
                                            ctrl.set_portamento_enable(porta_on);
                                        }
                                    }

                                    if porta_enable {
                                        ui.label("TIME:");
                                        let mut pt = porta_time;
                                        if ui
                                            .add(edit_slider(&mut pt, 0.0..=99.0).show_value(false))
                                            .changed()
                                        {
                                            if let Ok(mut ctrl) = self.lock_controller() {
                                                ctrl.set_portamento_time(pt);
                                            }
                                        }
                                        ui.label(format!("{:.0}", porta_time));
                                    }
                                });

                                ui.horizontal(|ui| {
                                    ui.label("GLIS:");
                                    let mut gliss = self.snapshot.portamento_glissando;
                                    if ui.checkbox(&mut gliss, "").changed() {
                                        if let Ok(mut ctrl) = self.lock_controller() {
                                            ctrl.set_portamento_glissando(gliss);
                                        }
                                    }
                                });
                            }
                        });

                        ui.separator();

                        // Right section: Panic and Init buttons
                        ui.vertical(|ui| {
                            ui.set_min_width(100.0);
                            ui.horizontal(|ui| {
                                if ui.small_button("PANIC").clicked() {
                                    if let Ok(mut ctrl) = self.lock_controller() {
                                        ctrl.panic();
                                    }
                                }

                                if ui.small_button("INIT").clicked() {
                                    self.compare_edit = None;
                                    if let Ok(mut ctrl) = self.lock_controller() {
                                        ctrl.voice_initialize();
                                    }
                                }
                            });
                        });
                    });
                });
            };
        });
    }

    fn draw_mode_controls_compact(&mut self, ui: &mut egui::Ui) {
        use crate::state_snapshot::VoiceMode;
        let voice_mode = self.snapshot.voice_mode;
        let is_mono = voice_mode != VoiceMode::Poly;
        ui.horizontal(|ui| {
            ui.label("MODE:");
            let mut mode = voice_mode;
            if ui
                .selectable_value(&mut mode, VoiceMode::Poly, "POLY")
                .clicked()
                && voice_mode != VoiceMode::Poly
            {
                if let Ok(mut ctrl) = self.lock_controller() {
                    ctrl.set_voice_mode(VoiceMode::Poly);
                }
            }
            if ui
                .selectable_value(&mut mode, VoiceMode::Mono, "MONO")
                .clicked()
                && voice_mode != VoiceMode::Mono
            {
                if let Ok(mut ctrl) = self.lock_controller() {
                    ctrl.set_voice_mode(VoiceMode::Mono);
                }
            }
            if ui
                .selectable_value(&mut mode, VoiceMode::MonoLegato, "M-LEG")
                .clicked()
                && voice_mode != VoiceMode::MonoLegato
            {
                if let Ok(mut ctrl) = self.lock_controller() {
                    ctrl.set_voice_mode(VoiceMode::MonoLegato);
                }
            }
        });

        // Portamento (only visible in MONO modes)
        if is_mono {
            let porta_enable = self.snapshot.portamento_enable;
            let porta_time = self.snapshot.portamento_time;
            ui.horizontal(|ui| {
                ui.label("PORTA:");
                let mut porta_on = porta_enable;
                if ui.checkbox(&mut porta_on, "").changed() {
                    if let Ok(mut ctrl) = self.lock_controller() {
                        ctrl.set_portamento_enable(porta_on);
                    }
                }

                if porta_enable {
                    ui.label("TIME:");
                    let mut pt = porta_time;
                    if ui
                        .add(edit_slider(&mut pt, 0.0..=99.0).show_value(false))
                        .changed()
                    {
                        if let Ok(mut ctrl) = self.lock_controller() {
                            ctrl.set_portamento_time(pt);
                        }
                    }
                    ui.label(format!("{:.0}", porta_time));
                }
            });
        }
    }

    fn draw_tune_and_utilities_compact(&mut self, ui: &mut egui::Ui) {
        let master_tune = self.snapshot.master_tune;
        let pb_range = self.snapshot.pitch_bend_range;

        // First row: Master Tune
        ui.horizontal(|ui| {
            ui.label("TUNE:");
            let mut tune = master_tune;
            if ui
                .add(edit_slider(&mut tune, -150.0..=150.0).show_value(false))
                .changed()
            {
                if let Ok(mut ctrl) = self.lock_controller() {
                    ctrl.set_master_tune(tune);
                }
            }
            ui.label(format!("{:.0}c", master_tune));

            if ui.small_button("RST").clicked() {
                if let Ok(mut ctrl) = self.lock_controller() {
                    ctrl.set_master_tune(0.0);
                }
            }
        });

        // Second row: Pitch Bend and utilities
        ui.horizontal(|ui| {
            ui.label("BEND:");
            let mut pb = pb_range;
            if ui
                .add(edit_slider(&mut pb, 0.0..=12.0).show_value(false))
                .changed()
            {
                if let Ok(mut ctrl) = self.lock_controller() {
                    ctrl.set_pitch_bend_range(pb);
                }
            }
            ui.label(format!("{:.0}", pb_range));

            ui.separator();

            if ui.small_button("PANIC").clicked() {
                if let Ok(mut ctrl) = self.lock_controller() {
                    ctrl.panic();
                }
            }

            if ui.small_button("INIT").clicked() {
                self.compare_edit = None;
                if let Ok(mut ctrl) = self.lock_controller() {
                    ctrl.voice_initialize();
                }
            }
        });
    }

    pub(super) fn draw_membrane_buttons(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().button_padding = egui::vec2(12.0, 6.0);

                // Make buttons more DX7-like with consistent sizing
                let button_size = egui::vec2(85.0, 25.0);

                let voice_button = if self.display_mode == DisplayMode::Voice {
                    egui::Button::new("VOICE")
                        .fill(egui::Color32::from_rgb(180, 200, 220))
                        .min_size(button_size)
                } else {
                    egui::Button::new("VOICE").min_size(button_size)
                };

                if ui.add(voice_button).clicked() {
                    self.display_mode = DisplayMode::Voice;
                    self.display_text = "VOICE SELECT".to_string();
                }

                let op_select_button = if self.display_mode == DisplayMode::Operator {
                    egui::Button::new("OPERATOR")
                        .fill(egui::Color32::from_rgb(180, 200, 220))
                        .min_size(button_size)
                } else {
                    egui::Button::new("OPERATOR").min_size(button_size)
                };

                if ui.add(op_select_button).clicked() {
                    self.display_mode = DisplayMode::Operator;
                    self.display_text = format!("OPERATOR {}", self.selected_operator + 1);
                }

                let lfo_button = if self.display_mode == DisplayMode::LFO {
                    egui::Button::new("LFO")
                        .fill(egui::Color32::from_rgb(180, 200, 220))
                        .min_size(button_size)
                } else {
                    egui::Button::new("LFO").min_size(button_size)
                };

                if ui.add(lfo_button).clicked() {
                    self.display_mode = DisplayMode::LFO;
                    self.display_text = "LFO CONTROLS".to_string();
                }

                let effects_button = if self.display_mode == DisplayMode::Effects {
                    egui::Button::new("EFFECTS")
                        .fill(egui::Color32::from_rgb(180, 200, 220))
                        .min_size(button_size)
                } else {
                    egui::Button::new("EFFECTS").min_size(button_size)
                };

                if ui.add(effects_button).clicked() {
                    self.display_mode = DisplayMode::Effects;
                    self.display_text = "EFFECTS".to_string();
                }

                let midi_button = if self.display_mode == DisplayMode::Midi {
                    egui::Button::new("MIDI")
                        .fill(egui::Color32::from_rgb(180, 200, 220))
                        .min_size(button_size)
                } else {
                    egui::Button::new("MIDI").min_size(button_size)
                };

                if ui.add(midi_button).clicked() {
                    self.display_mode = DisplayMode::Midi;
                    self.display_text = "MIDI / CONTROLLERS".to_string();
                    self.last_device_poll = None;
                }

                let comparing = self.compare_edit.is_some();
                let compare_button = if comparing {
                    egui::Button::new("COMPARE")
                        .fill(egui::Color32::from_rgb(230, 180, 90))
                        .min_size(button_size)
                } else {
                    egui::Button::new("COMPARE").min_size(button_size)
                };
                let can_compare =
                    comparing || (self.snapshot.voice_edited && self.compare_original().is_some());
                if ui
                    .add_enabled(can_compare, compare_button)
                    .on_hover_text("Hear the stored voice; press again to return to your edit")
                    .on_disabled_hover_text("Available after editing a voice from the browser")
                    .clicked()
                {
                    self.toggle_compare();
                }
            });
        });
    }
}
