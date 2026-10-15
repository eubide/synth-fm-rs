use super::{edit_slider, Dx7App};
use crate::algorithms;
use crate::command_queue::{EnvelopeParam, OperatorParam};
use crate::operator::KeyScaleCurve;
use eframe::egui;

impl Dx7App {
    /// Operator selector strip: a row of 6 mini-panels distributed evenly
    /// across the full width. Each cell shows OP number, role (Carrier /
    /// Modulator / Feedback), live output level bar, and acts as a button
    /// to select that operator.
    pub(super) fn draw_operator_selector_strip(&mut self, ui: &mut egui::Ui) {
        let current_alg = self.snapshot.algorithm;
        let alg_info = algorithms::get_algorithm_info(current_alg);

        ui.group(|ui| {
            ui.label(egui::RichText::new("SELECT OPERATOR").size(10.0));
            ui.columns(6, |cols| {
                for (op_idx, cui) in cols.iter_mut().enumerate().take(6) {
                    let op_num = (op_idx + 1) as u8;
                    let is_carrier = alg_info.carriers.contains(&op_num);
                    let is_selected = self.selected_operator == op_idx;
                    let has_feedback = alg_info.feedback_op == op_num;

                    let enabled = self.snapshot.operators[op_idx].enabled;
                    let level = self.snapshot.operators[op_idx].output_level;

                    let base_color = if !enabled {
                        egui::Color32::from_rgb(80, 80, 80)
                    } else if is_carrier {
                        egui::Color32::from_rgb(70, 130, 180)
                    } else {
                        egui::Color32::from_rgb(100, 160, 100)
                    };

                    let frame = egui::Frame::NONE
                        .fill(if is_selected {
                            egui::Color32::from_rgb(240, 248, 255)
                        } else {
                            egui::Color32::from_rgb(250, 250, 250)
                        })
                        .stroke(egui::Stroke::new(
                            if is_selected { 2.5_f32 } else { 1.0 },
                            if is_selected {
                                egui::Color32::from_rgb(255, 180, 0)
                            } else {
                                base_color
                            },
                        ))
                        .corner_radius(4)
                        .inner_margin(6.0);

                    frame.show(cui, |ui| {
                        ui.vertical_centered(|ui| {
                            let role = if is_carrier { "C" } else { "M" };
                            let fb = if has_feedback { " F" } else { "" };
                            let label_text = format!("OP{} {}{}", op_num, role, fb);

                            if ui
                                .selectable_label(
                                    is_selected,
                                    egui::RichText::new(label_text).size(11.0).color(base_color),
                                )
                                .clicked()
                            {
                                self.selected_operator = op_idx;
                            }

                            // Level bar (horizontal). Width follows the cell,
                            // capped so very wide screens don't stretch it
                            // into a long strip.
                            let bar_width = ui.available_width().min(90.0);
                            let bar_height = 10.0;
                            let (bar_rect, _) = ui.allocate_exact_size(
                                egui::vec2(bar_width, bar_height),
                                egui::Sense::hover(),
                            );
                            ui.painter().rect_filled(
                                bar_rect,
                                2.0,
                                egui::Color32::from_rgb(40, 40, 40),
                            );
                            let fill_width = (level / 99.0) * bar_width;
                            let fill_rect = egui::Rect::from_min_size(
                                bar_rect.min,
                                egui::vec2(fill_width, bar_height),
                            );
                            ui.painter().rect_filled(
                                fill_rect,
                                2.0,
                                if enabled {
                                    base_color
                                } else {
                                    egui::Color32::from_rgb(60, 60, 60)
                                },
                            );

                            ui.label(egui::RichText::new(format!("{:.0}", level)).size(10.0));
                        });
                    });
                }
            });
        });
    }

    /// Full operator panel with all parameters and envelope
    pub(super) fn draw_operator_full_panel(&mut self, ui: &mut egui::Ui) {
        let op_idx = self.selected_operator;
        let current_alg = self.snapshot.algorithm;
        let alg_info = algorithms::get_algorithm_info(current_alg);
        let op_num = (op_idx + 1) as u8;
        let is_carrier = alg_info.carriers.contains(&op_num);
        let has_feedback = alg_info.feedback_op == op_num;

        // Read all operator parameters from snapshot (lock-free)
        let op_snap = &self.snapshot.operators[op_idx];
        let mut enabled = op_snap.enabled;
        // DX7 COARSE: 0 is the 0.5x ratio, 1..=31 the integer multipliers.
        let mut coarse: u8 = match op_snap.frequency_ratio {
            r if r < 0.75 => 0,
            r => r.round().clamp(1.0, 31.0) as u8,
        };
        let mut output_level = op_snap.output_level;
        let mut detune = op_snap.detune;
        let mut feedback = op_snap.feedback;
        let mut vel_sens = op_snap.velocity_sensitivity;
        let mut l_depth = op_snap.key_scale_left_depth;
        let mut r_depth = op_snap.key_scale_right_depth;
        let mut breakpoint_note = op_snap.key_scale_breakpoint as f32;
        let mut l_curve = op_snap.key_scale_left_curve;
        let mut r_curve = op_snap.key_scale_right_curve;
        let mut key_scale_rt = op_snap.key_scale_rate;
        let mut am_sens = op_snap.am_sensitivity as f32;
        let mut osc_sync = op_snap.oscillator_key_sync;
        let mut fixed_freq = op_snap.fixed_frequency;
        let mut fixed_hz = op_snap.fixed_freq_hz;
        let mut rate1 = op_snap.rate1;
        let mut rate2 = op_snap.rate2;
        let mut rate3 = op_snap.rate3;
        let mut rate4 = op_snap.rate4;
        let mut level1 = op_snap.level1;
        let mut level2 = op_snap.level2;
        let mut level3 = op_snap.level3;
        let mut level4 = op_snap.level4;

        ui.group(|ui| {
            // Header
            let role = if is_carrier { "CARRIER" } else { "MODULATOR" };
            let fb_text = if has_feedback { " [FB]" } else { "" };
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("OPERATOR {} — {}{}", op_num, role, fb_text))
                        .strong(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.checkbox(&mut enabled, "ON").changed() {
                        if let Ok(mut ctrl) = self.lock_controller() {
                            ctrl.set_operator_param(
                                op_idx as u8,
                                OperatorParam::Enabled,
                                if enabled { 1.0 } else { 0.0 },
                            );
                        }
                    }
                });
            });
            ui.separator();

            ui.add_enabled_ui(enabled, |ui| {
                ui.columns(3, |cols| {
                    cols[0].vertical(|ui| {
                        ui.label(egui::RichText::new("PARAMETERS").size(10.0).strong());
                        egui::Grid::new(("op_params_grid", op_idx))
                            .num_columns(2)
                            .spacing([8.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Ratio:");
                                let coarse_ratio = |c: f64| if c < 0.5 { 0.5 } else { c };
                                if ui
                                    .add(
                                        edit_slider(&mut coarse, 0..=31).custom_formatter(
                                            |n, _| format!("{:.2}", coarse_ratio(n)),
                                        ),
                                    )
                                    .changed()
                                {
                                    if let Ok(mut ctrl) = self.lock_controller() {
                                        ctrl.set_operator_param(
                                            op_idx as u8,
                                            OperatorParam::Ratio,
                                            coarse_ratio(coarse as f64) as f32,
                                        );
                                    }
                                }
                                ui.end_row();

                                ui.label("Level:");
                                if ui
                                    .add(edit_slider(&mut output_level, 0.0..=99.0).integer())
                                    .changed()
                                {
                                    if let Ok(mut ctrl) = self.lock_controller() {
                                        ctrl.set_operator_param(
                                            op_idx as u8,
                                            OperatorParam::Level,
                                            output_level,
                                        );
                                    }
                                }
                                ui.end_row();

                                ui.label("Detune:");
                                if ui
                                    .add(edit_slider(&mut detune, -7.0..=7.0).integer())
                                    .changed()
                                {
                                    if let Ok(mut ctrl) = self.lock_controller() {
                                        ctrl.set_operator_param(
                                            op_idx as u8,
                                            OperatorParam::Detune,
                                            detune,
                                        );
                                    }
                                }
                                ui.end_row();

                                ui.label("Vel Sens:");
                                if ui
                                    .add(edit_slider(&mut vel_sens, 0.0..=7.0).integer())
                                    .changed()
                                {
                                    if let Ok(mut ctrl) = self.lock_controller() {
                                        ctrl.set_operator_param(
                                            op_idx as u8,
                                            OperatorParam::VelocitySensitivity,
                                            vel_sens,
                                        );
                                    }
                                }
                                ui.end_row();

                                if has_feedback {
                                    ui.label("Feedback:");
                                    if ui
                                        .add(edit_slider(&mut feedback, 0.0..=7.0).integer())
                                        .changed()
                                    {
                                        if let Ok(mut ctrl) = self.lock_controller() {
                                            ctrl.set_operator_param(
                                                op_idx as u8,
                                                OperatorParam::Feedback,
                                                feedback,
                                            );
                                        }
                                    }
                                    ui.end_row();
                                }

                                ui.label("AM Sens:");
                                if ui
                                    .add(edit_slider(&mut am_sens, 0.0..=3.0).integer())
                                    .changed()
                                {
                                    if let Ok(mut ctrl) = self.lock_controller() {
                                        ctrl.set_operator_param(
                                            op_idx as u8,
                                            OperatorParam::AmSensitivity,
                                            am_sens,
                                        );
                                    }
                                }
                                ui.end_row();

                                ui.label("Key Sync:");
                                if ui.checkbox(&mut osc_sync, "ON").changed() {
                                    if let Ok(mut ctrl) = self.lock_controller() {
                                        ctrl.set_operator_param(
                                            op_idx as u8,
                                            OperatorParam::OscillatorKeySync,
                                            if osc_sync { 1.0 } else { 0.0 },
                                        );
                                    }
                                }
                                ui.end_row();

                                ui.label("Fixed:");
                                if ui.checkbox(&mut fixed_freq, "Hz").changed() {
                                    if let Ok(mut ctrl) = self.lock_controller() {
                                        ctrl.set_operator_param(
                                            op_idx as u8,
                                            OperatorParam::FixedFrequency,
                                            if fixed_freq { 1.0 } else { 0.0 },
                                        );
                                    }
                                }
                                ui.end_row();

                                if fixed_freq {
                                    ui.label("Fixed Hz:");
                                    if ui
                                        .add(
                                            edit_slider(
                                                &mut fixed_hz,
                                                1.0..=crate::dx7_frequency::fixed_frequency_hz(
                                                    3, 99,
                                                ),
                                            )
                                            .logarithmic(true)
                                            .suffix(" Hz"),
                                        )
                                        .changed()
                                    {
                                        if let Ok(mut ctrl) = self.lock_controller() {
                                            ctrl.set_operator_param(
                                                op_idx as u8,
                                                OperatorParam::FixedFreqHz,
                                                fixed_hz,
                                            );
                                        }
                                    }
                                    ui.end_row();
                                }
                            });
                    });

                    cols[1].vertical(|ui| {
                        ui.label(egui::RichText::new("KEY SCALING").size(10.0).strong());
                        egui::Grid::new(("op_keyscale_grid", op_idx))
                            .num_columns(2)
                            .spacing([8.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Breakpoint:");
                                if ui
                                    .add(
                                        edit_slider(&mut breakpoint_note, 0.0..=127.0)
                                            .integer()
                                            .custom_formatter(|n, _| midi_note_name(n as u8)),
                                    )
                                    .changed()
                                {
                                    if let Ok(mut ctrl) = self.lock_controller() {
                                        ctrl.set_operator_param(
                                            op_idx as u8,
                                            OperatorParam::KeyScaleBreakpoint,
                                            breakpoint_note.round(),
                                        );
                                    }
                                }
                                ui.end_row();

                                ui.label("Rate Scl:");
                                if ui
                                    .add(edit_slider(&mut key_scale_rt, 0.0..=7.0).integer())
                                    .changed()
                                {
                                    if let Ok(mut ctrl) = self.lock_controller() {
                                        ctrl.set_operator_param(
                                            op_idx as u8,
                                            OperatorParam::KeyScaleRate,
                                            key_scale_rt,
                                        );
                                    }
                                }
                                ui.end_row();

                                ui.label("L Depth:");
                                if ui
                                    .add(edit_slider(&mut l_depth, 0.0..=99.0).integer())
                                    .changed()
                                {
                                    if let Ok(mut ctrl) = self.lock_controller() {
                                        ctrl.set_operator_param(
                                            op_idx as u8,
                                            OperatorParam::KeyScaleLeftDepth,
                                            l_depth,
                                        );
                                    }
                                }
                                ui.end_row();

                                ui.label("R Depth:");
                                if ui
                                    .add(edit_slider(&mut r_depth, 0.0..=99.0).integer())
                                    .changed()
                                {
                                    if let Ok(mut ctrl) = self.lock_controller() {
                                        ctrl.set_operator_param(
                                            op_idx as u8,
                                            OperatorParam::KeyScaleRightDepth,
                                            r_depth,
                                        );
                                    }
                                }
                                ui.end_row();

                                ui.label("L Curve:");
                                let prev_l_curve = l_curve;
                                egui::ComboBox::from_id_salt(("op_lcurve", op_idx))
                                    .selected_text(key_scale_curve_label(l_curve))
                                    .width(70.0)
                                    .show_ui(ui, |ui| {
                                        for c in [
                                            KeyScaleCurve::NegLin,
                                            KeyScaleCurve::NegExp,
                                            KeyScaleCurve::PosExp,
                                            KeyScaleCurve::PosLin,
                                        ] {
                                            ui.selectable_value(
                                                &mut l_curve,
                                                c,
                                                key_scale_curve_label(c),
                                            );
                                        }
                                    });
                                if l_curve != prev_l_curve {
                                    if let Ok(mut ctrl) = self.lock_controller() {
                                        ctrl.set_operator_param(
                                            op_idx as u8,
                                            OperatorParam::KeyScaleLeftCurve,
                                            l_curve.to_dx7_code() as f32,
                                        );
                                    }
                                }
                                ui.end_row();

                                ui.label("R Curve:");
                                let prev_r_curve = r_curve;
                                egui::ComboBox::from_id_salt(("op_rcurve", op_idx))
                                    .selected_text(key_scale_curve_label(r_curve))
                                    .width(70.0)
                                    .show_ui(ui, |ui| {
                                        for c in [
                                            KeyScaleCurve::NegLin,
                                            KeyScaleCurve::NegExp,
                                            KeyScaleCurve::PosExp,
                                            KeyScaleCurve::PosLin,
                                        ] {
                                            ui.selectable_value(
                                                &mut r_curve,
                                                c,
                                                key_scale_curve_label(c),
                                            );
                                        }
                                    });
                                if r_curve != prev_r_curve {
                                    if let Ok(mut ctrl) = self.lock_controller() {
                                        ctrl.set_operator_param(
                                            op_idx as u8,
                                            OperatorParam::KeyScaleRightCurve,
                                            r_curve.to_dx7_code() as f32,
                                        );
                                    }
                                }
                                ui.end_row();
                            });
                    });

                    cols[2].vertical(|ui| {
                        ui.label(egui::RichText::new("ENVELOPE").size(10.0).strong());
                        egui::Grid::new(("op_env_grid", op_idx))
                            .num_columns(2)
                            .spacing([8.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("R1:");
                                if ui
                                    .add(edit_slider(&mut rate1, 0.0..=99.0).integer())
                                    .changed()
                                {
                                    if let Ok(mut ctrl) = self.lock_controller() {
                                        ctrl.set_envelope_param(
                                            op_idx as u8,
                                            EnvelopeParam::Rate1,
                                            rate1,
                                        );
                                    }
                                }
                                ui.end_row();

                                ui.label("L1:");
                                if ui
                                    .add(edit_slider(&mut level1, 0.0..=99.0).integer())
                                    .changed()
                                {
                                    if let Ok(mut ctrl) = self.lock_controller() {
                                        ctrl.set_envelope_param(
                                            op_idx as u8,
                                            EnvelopeParam::Level1,
                                            level1,
                                        );
                                    }
                                }
                                ui.end_row();

                                ui.label("R2:");
                                if ui
                                    .add(edit_slider(&mut rate2, 0.0..=99.0).integer())
                                    .changed()
                                {
                                    if let Ok(mut ctrl) = self.lock_controller() {
                                        ctrl.set_envelope_param(
                                            op_idx as u8,
                                            EnvelopeParam::Rate2,
                                            rate2,
                                        );
                                    }
                                }
                                ui.end_row();

                                ui.label("L2:");
                                if ui
                                    .add(edit_slider(&mut level2, 0.0..=99.0).integer())
                                    .changed()
                                {
                                    if let Ok(mut ctrl) = self.lock_controller() {
                                        ctrl.set_envelope_param(
                                            op_idx as u8,
                                            EnvelopeParam::Level2,
                                            level2,
                                        );
                                    }
                                }
                                ui.end_row();

                                ui.label("R3:");
                                if ui
                                    .add(edit_slider(&mut rate3, 0.0..=99.0).integer())
                                    .changed()
                                {
                                    if let Ok(mut ctrl) = self.lock_controller() {
                                        ctrl.set_envelope_param(
                                            op_idx as u8,
                                            EnvelopeParam::Rate3,
                                            rate3,
                                        );
                                    }
                                }
                                ui.end_row();

                                ui.label("L3:");
                                if ui
                                    .add(edit_slider(&mut level3, 0.0..=99.0).integer())
                                    .changed()
                                {
                                    if let Ok(mut ctrl) = self.lock_controller() {
                                        ctrl.set_envelope_param(
                                            op_idx as u8,
                                            EnvelopeParam::Level3,
                                            level3,
                                        );
                                    }
                                }
                                ui.end_row();

                                ui.label("R4:");
                                if ui
                                    .add(edit_slider(&mut rate4, 0.0..=99.0).integer())
                                    .changed()
                                {
                                    if let Ok(mut ctrl) = self.lock_controller() {
                                        ctrl.set_envelope_param(
                                            op_idx as u8,
                                            EnvelopeParam::Rate4,
                                            rate4,
                                        );
                                    }
                                }
                                ui.end_row();

                                ui.label("L4:");
                                if ui
                                    .add(edit_slider(&mut level4, 0.0..=99.0).integer())
                                    .changed()
                                {
                                    if let Ok(mut ctrl) = self.lock_controller() {
                                        ctrl.set_envelope_param(
                                            op_idx as u8,
                                            EnvelopeParam::Level4,
                                            level4,
                                        );
                                    }
                                }
                                ui.end_row();
                            });
                    });
                });
            });
        });
    }
}

/// Format a MIDI note number using the DX7/codebase convention
/// (A-1 = MIDI 21, C3 = MIDI 60). Used by the Key Scaling Breakpoint slider.
fn midi_note_name(midi: u8) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let octave = (midi as i32) / 12 - 2;
    format!("{}{}", NAMES[(midi as usize) % 12], octave)
}

/// Compact label for the Key Scaling curve dropdowns.
/// Mirrors Dexed: -Lin / -Exp / +Exp / +Lin.
fn key_scale_curve_label(curve: KeyScaleCurve) -> &'static str {
    match curve {
        KeyScaleCurve::NegLin => "-Lin",
        KeyScaleCurve::NegExp => "-Exp",
        KeyScaleCurve::PosExp => "+Exp",
        KeyScaleCurve::PosLin => "+Lin",
    }
}
