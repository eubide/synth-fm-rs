use super::{edit_slider, Dx7App};
use crate::command_queue::{LfoParam, PitchEgParam};
use eframe::egui;

impl Dx7App {
    pub(super) fn draw_lfo_panel(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.label("LFO CONTROLS");
                ui.separator();

                let mut lfo_rate = self.snapshot.lfo_rate;
                let mut lfo_delay = self.snapshot.lfo_delay;
                let mut lfo_pitch_depth = self.snapshot.lfo_pitch_depth;
                let mut lfo_amp_depth = self.snapshot.lfo_amp_depth;
                let lfo_waveform = self.snapshot.lfo_waveform;
                let mut lfo_key_sync = self.snapshot.lfo_key_sync;

                ui.columns(2, |columns| {
                    // Left column: Timing
                    columns[0].vertical(|ui| {
                        ui.label("TIMING");
                        ui.horizontal(|ui| {
                            ui.label("Rate:");
                            if ui
                                .add(edit_slider(&mut lfo_rate, 0.0..=99.0).integer())
                                .changed()
                            {
                                if let Ok(mut ctrl) = self.lock_controller() {
                                    ctrl.set_lfo_param(LfoParam::Rate, lfo_rate);
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Delay:");
                            if ui
                                .add(edit_slider(&mut lfo_delay, 0.0..=99.0).integer())
                                .changed()
                            {
                                if let Ok(mut ctrl) = self.lock_controller() {
                                    ctrl.set_lfo_param(LfoParam::Delay, lfo_delay);
                                }
                            }
                        });
                        ui.label(format!(
                            "Freq: {:.2} Hz | Delay: {:.2}s",
                            self.snapshot.lfo_frequency_hz, self.snapshot.lfo_delay_seconds
                        ));
                    });

                    // Right column: Modulation
                    columns[1].vertical(|ui| {
                        ui.label("MODULATION");
                        ui.horizontal(|ui| {
                            ui.label("Pitch:");
                            if ui
                                .add(edit_slider(&mut lfo_pitch_depth, 0.0..=99.0).integer())
                                .changed()
                            {
                                if let Ok(mut ctrl) = self.lock_controller() {
                                    ctrl.set_lfo_param(LfoParam::PitchDepth, lfo_pitch_depth);
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Amp:");
                            if ui
                                .add(edit_slider(&mut lfo_amp_depth, 0.0..=99.0).integer())
                                .changed()
                            {
                                if let Ok(mut ctrl) = self.lock_controller() {
                                    ctrl.set_lfo_param(LfoParam::AmpDepth, lfo_amp_depth);
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Wave:");
                            egui::ComboBox::from_id_salt("lfo_waveform")
                                .selected_text(lfo_waveform.name())
                                .show_ui(ui, |ui| {
                                    for (i, &waveform) in
                                        crate::lfo::LFOWaveform::all().iter().enumerate()
                                    {
                                        if ui
                                            .selectable_value(
                                                &mut lfo_waveform.clone(),
                                                waveform,
                                                waveform.name(),
                                            )
                                            .clicked()
                                        {
                                            if let Ok(mut ctrl) = self.lock_controller() {
                                                ctrl.set_lfo_param(
                                                    LfoParam::Waveform(i as u8),
                                                    0.0,
                                                );
                                            }
                                        }
                                    }
                                });
                        });
                        ui.horizontal(|ui| {
                            ui.label("Key Sync:");
                            if ui.checkbox(&mut lfo_key_sync, "").changed() {
                                if let Ok(mut ctrl) = self.lock_controller() {
                                    ctrl.set_lfo_param(
                                        LfoParam::KeySync,
                                        if lfo_key_sync { 1.0 } else { 0.0 },
                                    );
                                }
                            }
                        });
                    });
                });

                ui.separator();
                ui.label("MOD WHEEL ROUTING");
                let mut pms = self.snapshot.pitch_mod_sensitivity as f32;
                let mut eg_bias = self.snapshot.eg_bias_sensitivity as f32;
                let mut pitch_bias = self.snapshot.pitch_bias_sensitivity as f32;
                ui.columns(3, |columns| {
                    columns[0].horizontal(|ui| {
                        ui.label("PMS:");
                        if ui.add(edit_slider(&mut pms, 0.0..=7.0).integer()).changed() {
                            if let Ok(mut ctrl) = self.lock_controller() {
                                ctrl.set_pitch_mod_sensitivity(pms as u8);
                            }
                        }
                    });
                    columns[1].horizontal(|ui| {
                        ui.label("EG Bias:");
                        if ui
                            .add(edit_slider(&mut eg_bias, 0.0..=7.0).integer())
                            .changed()
                        {
                            if let Ok(mut ctrl) = self.lock_controller() {
                                ctrl.set_eg_bias_sensitivity(eg_bias as u8);
                            }
                        }
                    });
                    columns[2].horizontal(|ui| {
                        ui.label("P-Bias:");
                        if ui
                            .add(edit_slider(&mut pitch_bias, 0.0..=7.0).integer())
                            .changed()
                        {
                            if let Ok(mut ctrl) = self.lock_controller() {
                                ctrl.set_pitch_bias_sensitivity(pitch_bias as u8);
                            }
                        }
                    });
                });

                ui.separator();
                self.draw_pitch_eg_section(ui);

                ui.separator();
                let mod_pct = (self.snapshot.mod_wheel * 100.0) as i32;
                ui.label(format!(
                    "Mod Wheel: {}%{}",
                    mod_pct,
                    if mod_pct == 0 {
                        " (move to enable)"
                    } else {
                        ""
                    }
                ));
            });
        });
    }

    /// Pitch EG panel — 4 rates + 4 levels matching the amplitude EG layout.
    /// On the DX7, level 50 means "no pitch offset"; 0 ≈ −4 octaves and 99 ≈ +4 octaves.
    fn draw_pitch_eg_section(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("PITCH EG").strong());
            let mut peg_enabled = self.snapshot.pitch_eg.enabled;
            if ui.checkbox(&mut peg_enabled, "enabled").changed() {
                if let Ok(mut ctrl) = self.lock_controller() {
                    ctrl.set_pitch_eg_param(
                        PitchEgParam::Enabled,
                        if peg_enabled { 1.0 } else { 0.0 },
                    );
                }
            }
            ui.label(
                egui::RichText::new("(L=50 → no offset; 0 ≈ −4 oct, 99 ≈ +4 oct)")
                    .size(10.0)
                    .color(egui::Color32::from_rgb(120, 120, 120)),
            );
        });

        let peg = self.snapshot.pitch_eg;
        egui::Grid::new("pitch_eg_grid")
            .num_columns(4)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                self.pitch_eg_slider(ui, "R1:", peg.rate1, PitchEgParam::Rate1);
                self.pitch_eg_slider(ui, "R2:", peg.rate2, PitchEgParam::Rate2);
                ui.end_row();
                self.pitch_eg_slider(ui, "L1:", peg.level1, PitchEgParam::Level1);
                self.pitch_eg_slider(ui, "L2:", peg.level2, PitchEgParam::Level2);
                ui.end_row();
                self.pitch_eg_slider(ui, "R3:", peg.rate3, PitchEgParam::Rate3);
                self.pitch_eg_slider(ui, "R4:", peg.rate4, PitchEgParam::Rate4);
                ui.end_row();
                self.pitch_eg_slider(ui, "L3:", peg.level3, PitchEgParam::Level3);
                self.pitch_eg_slider(ui, "L4:", peg.level4, PitchEgParam::Level4);
                ui.end_row();
            });
    }

    /// One labelled 0..99 slider for a Pitch EG parameter. Mirrors the look of
    /// the operator amplitude EG and the existing `routing_slider` helper.
    fn pitch_eg_slider(&self, ui: &mut egui::Ui, label: &str, value: f32, param: PitchEgParam) {
        ui.label(label);
        let mut v = value;
        if ui.add(edit_slider(&mut v, 0.0..=99.0).integer()).changed() {
            if let Ok(mut ctrl) = self.lock_controller() {
                ctrl.set_pitch_eg_param(param, v);
            }
        }
    }
}
