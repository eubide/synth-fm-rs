use super::{edit_slider, Dx7App};
use crate::command_queue::{EffectParam, EffectType};
use eframe::egui;

impl Dx7App {
    pub(super) fn draw_effects_panel(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.label("EFFECTS");
                ui.separator();

                ui.columns(4, |columns| {
                    self.draw_chorus_effect(&mut columns[0]);
                    self.draw_auto_pan_effect(&mut columns[1]);
                    self.draw_delay_effect(&mut columns[2]);
                    self.draw_reverb_effect(&mut columns[3]);
                });

                ui.separator();
                ui.label("Signal: Input -> Chorus -> AutoPan -> Delay -> Reverb -> Output");
            });
        });
    }

    fn draw_chorus_effect(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("CHORUS").strong());

                let chorus = &self.snapshot.chorus;
                let mut enabled = chorus.enabled;
                let mut rate = chorus.rate;
                let mut depth = chorus.depth;
                let mut mix = chorus.mix;
                let mut feedback = chorus.feedback;

                ui.horizontal(|ui| {
                    ui.label("Enable:");
                    if ui.checkbox(&mut enabled, "").changed() {
                        if let Ok(mut ctrl) = self.lock_controller() {
                            ctrl.set_effect_param(
                                EffectType::Chorus,
                                EffectParam::Enabled,
                                if enabled { 1.0 } else { 0.0 },
                            );
                        }
                    }
                });

                ui.add_enabled_ui(enabled, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Rate:");
                        if ui
                            .add(
                                edit_slider(&mut rate, 0.1..=5.0)
                                    .suffix(" Hz")
                                    .show_value(true),
                            )
                            .changed()
                        {
                            if let Ok(mut ctrl) = self.lock_controller() {
                                ctrl.set_effect_param(
                                    EffectType::Chorus,
                                    EffectParam::ChorusRate,
                                    rate,
                                );
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Depth:");
                        if ui
                            .add(
                                edit_slider(&mut depth, 0.0..=10.0)
                                    .suffix(" ms")
                                    .show_value(true),
                            )
                            .changed()
                        {
                            if let Ok(mut ctrl) = self.lock_controller() {
                                ctrl.set_effect_param(
                                    EffectType::Chorus,
                                    EffectParam::ChorusDepth,
                                    depth,
                                );
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Mix:");
                        if ui
                            .add(edit_slider(&mut mix, 0.0..=1.0).show_value(true))
                            .changed()
                        {
                            if let Ok(mut ctrl) = self.lock_controller() {
                                ctrl.set_effect_param(EffectType::Chorus, EffectParam::Mix, mix);
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Feedback:");
                        if ui
                            .add(edit_slider(&mut feedback, 0.0..=0.7).show_value(true))
                            .changed()
                        {
                            if let Ok(mut ctrl) = self.lock_controller() {
                                ctrl.set_effect_param(
                                    EffectType::Chorus,
                                    EffectParam::ChorusFeedback,
                                    feedback,
                                );
                            }
                        }
                    });
                });
            });
        });
    }

    fn draw_auto_pan_effect(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("AUTOPAN").strong());

                let ap = &self.snapshot.auto_pan;
                let mut enabled = ap.enabled;
                let mut rate_hz = ap.rate_hz;
                let mut depth = ap.depth;

                ui.horizontal(|ui| {
                    ui.label("Enable:");
                    if ui.checkbox(&mut enabled, "").changed() {
                        if let Ok(mut ctrl) = self.lock_controller() {
                            ctrl.set_effect_param(
                                EffectType::AutoPan,
                                EffectParam::Enabled,
                                if enabled { 1.0 } else { 0.0 },
                            );
                        }
                    }
                });

                ui.add_enabled_ui(enabled, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Rate:");
                        if ui
                            .add(
                                edit_slider(&mut rate_hz, 0.1..=10.0)
                                    .suffix(" Hz")
                                    .show_value(true),
                            )
                            .changed()
                        {
                            if let Ok(mut ctrl) = self.lock_controller() {
                                ctrl.set_effect_param(
                                    EffectType::AutoPan,
                                    EffectParam::AutoPanRate,
                                    rate_hz,
                                );
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Depth:");
                        if ui
                            .add(edit_slider(&mut depth, 0.0..=1.0).show_value(true))
                            .changed()
                        {
                            if let Ok(mut ctrl) = self.lock_controller() {
                                ctrl.set_effect_param(
                                    EffectType::AutoPan,
                                    EffectParam::AutoPanDepth,
                                    depth,
                                );
                            }
                        }
                    });
                });
            });
        });
    }

    fn draw_delay_effect(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("DELAY").strong());

                let delay = &self.snapshot.delay;
                let mut enabled = delay.enabled;
                let mut time_ms = delay.time_ms;
                let mut feedback = delay.feedback;
                let mut mix = delay.mix;
                let mut ping_pong = delay.ping_pong;

                ui.horizontal(|ui| {
                    ui.label("Enable:");
                    if ui.checkbox(&mut enabled, "").changed() {
                        if let Ok(mut ctrl) = self.lock_controller() {
                            ctrl.set_effect_param(
                                EffectType::Delay,
                                EffectParam::Enabled,
                                if enabled { 1.0 } else { 0.0 },
                            );
                        }
                    }
                });

                ui.add_enabled_ui(enabled, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Time:");
                        if ui
                            .add(
                                edit_slider(&mut time_ms, 0.0..=1000.0)
                                    .suffix(" ms")
                                    .show_value(true),
                            )
                            .changed()
                        {
                            if let Ok(mut ctrl) = self.lock_controller() {
                                ctrl.set_effect_param(
                                    EffectType::Delay,
                                    EffectParam::DelayTime,
                                    time_ms,
                                );
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Feedback:");
                        if ui
                            .add(edit_slider(&mut feedback, 0.0..=0.9).show_value(true))
                            .changed()
                        {
                            if let Ok(mut ctrl) = self.lock_controller() {
                                ctrl.set_effect_param(
                                    EffectType::Delay,
                                    EffectParam::DelayFeedback,
                                    feedback,
                                );
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Mix:");
                        if ui
                            .add(edit_slider(&mut mix, 0.0..=1.0).show_value(true))
                            .changed()
                        {
                            if let Ok(mut ctrl) = self.lock_controller() {
                                ctrl.set_effect_param(EffectType::Delay, EffectParam::Mix, mix);
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Ping-Pong:");
                        if ui.checkbox(&mut ping_pong, "").changed() {
                            if let Ok(mut ctrl) = self.lock_controller() {
                                ctrl.set_effect_param(
                                    EffectType::Delay,
                                    EffectParam::DelayPingPong,
                                    if ping_pong { 1.0 } else { 0.0 },
                                );
                            }
                        }
                    });
                });
            });
        });
    }

    fn draw_reverb_effect(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("REVERB").strong());

                let reverb = &self.snapshot.reverb;
                let mut enabled = reverb.enabled;
                let mut room_size = reverb.room_size;
                let mut damping = reverb.damping;
                let mut mix = reverb.mix;
                let mut width = reverb.width;

                ui.horizontal(|ui| {
                    ui.label("Enable:");
                    if ui.checkbox(&mut enabled, "").changed() {
                        if let Ok(mut ctrl) = self.lock_controller() {
                            ctrl.set_effect_param(
                                EffectType::Reverb,
                                EffectParam::Enabled,
                                if enabled { 1.0 } else { 0.0 },
                            );
                        }
                    }
                });

                ui.add_enabled_ui(enabled, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Room Size:");
                        if ui
                            .add(edit_slider(&mut room_size, 0.0..=1.0).show_value(true))
                            .changed()
                        {
                            if let Ok(mut ctrl) = self.lock_controller() {
                                ctrl.set_effect_param(
                                    EffectType::Reverb,
                                    EffectParam::ReverbRoomSize,
                                    room_size,
                                );
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Damping:");
                        if ui
                            .add(edit_slider(&mut damping, 0.0..=1.0).show_value(true))
                            .changed()
                        {
                            if let Ok(mut ctrl) = self.lock_controller() {
                                ctrl.set_effect_param(
                                    EffectType::Reverb,
                                    EffectParam::ReverbDamping,
                                    damping,
                                );
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Mix:");
                        if ui
                            .add(edit_slider(&mut mix, 0.0..=1.0).show_value(true))
                            .changed()
                        {
                            if let Ok(mut ctrl) = self.lock_controller() {
                                ctrl.set_effect_param(EffectType::Reverb, EffectParam::Mix, mix);
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Width:");
                        if ui
                            .add(edit_slider(&mut width, 0.0..=1.0).show_value(true))
                            .changed()
                        {
                            if let Ok(mut ctrl) = self.lock_controller() {
                                ctrl.set_effect_param(
                                    EffectType::Reverb,
                                    EffectParam::ReverbWidth,
                                    width,
                                );
                            }
                        }
                    });
                });
            });
        });
    }
}
