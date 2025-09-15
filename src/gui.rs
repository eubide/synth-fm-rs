use crate::algorithms;
use crate::audio_engine::AudioEngine;
use crate::fm_synth::FmSynthesizer;
use crate::midi_handler::MidiHandler;
use crate::presets::{get_dx7_presets, Dx7Preset};
use eframe::egui;
use std::sync::{Arc, Mutex};

pub struct Dx7App {
    synthesizer: Arc<Mutex<FmSynthesizer>>,
    _audio_engine: AudioEngine,
    _midi_handler: Option<MidiHandler>,
    selected_operator: usize,
    display_mode: DisplayMode,
    display_text: String,
    last_key_times: std::collections::HashMap<egui::Key, std::time::Instant>,
    current_octave: i32,
    presets: Vec<Dx7Preset>,
    selected_preset: usize,
}

#[derive(PartialEq)]
enum DisplayMode {
    Voice,
    Operator,
    Algorithm,
    LFO,
}

impl Dx7App {
    pub fn new(
        synthesizer: Arc<Mutex<FmSynthesizer>>,
        audio_engine: AudioEngine,
        midi_handler: Option<MidiHandler>,
    ) -> Self {
        let presets = get_dx7_presets();

        // Apply the first preset (E.PIANO 1)
        if !presets.is_empty() {
            if let Ok(mut synth) = synthesizer.lock() {
                presets[0].apply_to_synth(&mut synth);
            }
        }

        Self {
            synthesizer,
            _audio_engine: audio_engine,
            _midi_handler: midi_handler,
            selected_operator: 0,
            display_mode: DisplayMode::Voice,
            display_text: "YAMAHA DX7".to_string(),
            last_key_times: std::collections::HashMap::new(),
            current_octave: 4,
            presets,
            selected_preset: 0,
        }
    }

    fn lock_synth(
        &self,
    ) -> Result<
        std::sync::MutexGuard<FmSynthesizer>,
        std::sync::PoisonError<std::sync::MutexGuard<FmSynthesizer>>,
    > {
        self.synthesizer.lock()
    }

    fn draw_dx7_display(&mut self, ui: &mut egui::Ui) {
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

                // Mode-specific sub text
                let sub_text = match self.display_mode {
                    DisplayMode::Voice => {
                        if let Ok(synth) = self.lock_synth() {
                            format!(
                                "VOICE: {} | ALG: {:02}",
                                synth.preset_name,
                                synth.get_algorithm()
                            )
                        } else {
                            "VOICE: ERROR".to_string()
                        }
                    }
                    DisplayMode::Operator => {
                        format!("OP{} EDIT", self.selected_operator + 1)
                    }
                    DisplayMode::Algorithm => {
                        if let Ok(synth) = self.lock_synth() {
                            let current_alg = synth.get_algorithm();
                            let alg_name = algorithms::get_algorithm_name(current_alg).to_string();
                            format!("ALG {} - {}", current_alg, alg_name)
                        } else {
                            "ALGORITHM: ERROR".to_string()
                        }
                    }
                    DisplayMode::LFO => {
                        if let Ok(synth) = self.lock_synth() {
                            let waveform_name = synth.get_lfo_waveform().name();
                            format!(
                                "LFO: {} | Rate: {:.0} | Mod: {:.0}%",
                                waveform_name,
                                synth.get_lfo_rate(),
                                synth.get_mod_wheel() * 100.0
                            )
                        } else {
                            "LFO: ERROR".to_string()
                        }
                    }
                };

                ui.label(
                    egui::RichText::new(sub_text)
                        .font(display_font)
                        .color(display_color),
                );

                ui.add_space(5.0);
                ui.separator();

                // Always display current status information
                let synth = self.synthesizer.lock().unwrap();
                let mode_text = if synth.get_mono_mode() {
                    "MONO"
                } else {
                    "POLY"
                };
                let midi_text = if self._midi_handler.is_some() {
                    "MIDI OK"
                } else {
                    "NO MIDI"
                };

                let status_line = if synth.get_mono_mode() {
                    // Show portamento only in MONO mode
                    let porta_text = if synth.get_portamento_enable() {
                        "ON"
                    } else {
                        "OFF"
                    };
                    format!(
                        "VOICE: {} | ALG: {:02} | MODE: {} | PORTA: {} | {}",
                        synth.preset_name,
                        synth.get_algorithm(),
                        mode_text,
                        porta_text,
                        midi_text
                    )
                } else {
                    // In POLY mode, don't show portamento
                    format!(
                        "VOICE: {} | ALG: {:02} | MODE: {} | {}",
                        synth.preset_name,
                        synth.get_algorithm(),
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

    fn draw_global_controls(&mut self, ui: &mut egui::Ui) {
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
                                if let Ok(mut synth) = self.lock_synth() {
                                    ui.allocate_ui_with_layout(
                                        egui::vec2(60.0, 20.0),
                                        egui::Layout::left_to_right(egui::Align::Center),
                                        |ui| {
                                            let mut volume = synth.get_master_volume();
                                            if ui
                                                .add(
                                                    egui::Slider::new(&mut volume, 0.0..=1.0)
                                                        .show_value(false),
                                                )
                                                .changed()
                                            {
                                                synth.set_master_volume(volume);
                                            }
                                        },
                                    );
                                    ui.label(format!("{:.0}", synth.get_master_volume() * 100.0));
                                }
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
                                if let Ok(mut synth) = self.lock_synth() {
                                    ui.allocate_ui_with_layout(
                                        egui::vec2(60.0, 20.0),
                                        egui::Layout::left_to_right(egui::Align::Center),
                                        |ui| {
                                            let mut volume = synth.get_master_volume();
                                            if ui
                                                .add(
                                                    egui::Slider::new(&mut volume, 0.0..=1.0)
                                                        .show_value(false),
                                                )
                                                .changed()
                                            {
                                                synth.set_master_volume(volume);
                                            }
                                        },
                                    );
                                    ui.label(format!("{:.0}", synth.get_master_volume() * 100.0));
                                }
                            });
                        });

                        ui.separator();

                        // Center-left section: Tuning controls
                        ui.vertical(|ui| {
                            ui.set_min_width(180.0);
                            if let Ok(mut synth) = self.lock_synth() {
                                // Master Tune
                                ui.horizontal(|ui| {
                                    ui.label("MASTER TUNE:");
                                    let mut master_tune = synth.get_master_tune();
                                    ui.allocate_ui_with_layout(
                                        egui::vec2(70.0, 20.0),
                                        egui::Layout::left_to_right(egui::Align::Center),
                                        |ui| {
                                            if ui
                                                .add(
                                                    egui::Slider::new(
                                                        &mut master_tune,
                                                        -150.0..=150.0,
                                                    )
                                                    .show_value(false),
                                                )
                                                .changed()
                                            {
                                                synth.set_master_tune(master_tune);
                                            }
                                        },
                                    );
                                    ui.label(format!("{:.0}c", master_tune));

                                    if ui.small_button("RST").clicked() {
                                        synth.set_master_tune(0.0);
                                    }
                                });

                                // Pitch Bend Range
                                ui.horizontal(|ui| {
                                    ui.label("PITCH BEND:");
                                    let mut pb_range = synth.get_pitch_bend_range();
                                    ui.allocate_ui_with_layout(
                                        egui::vec2(50.0, 20.0),
                                        egui::Layout::left_to_right(egui::Align::Center),
                                        |ui| {
                                            if ui
                                                .add(
                                                    egui::Slider::new(&mut pb_range, 0.0..=12.0)
                                                        .show_value(false),
                                                )
                                                .changed()
                                            {
                                                synth.set_pitch_bend_range(pb_range);
                                            }
                                        },
                                    );
                                    ui.label(format!("{:.0}", pb_range));
                                });
                            }
                        });

                        ui.separator();

                        // Center-right section: Mode controls
                        ui.vertical(|ui| {
                            ui.set_min_width(150.0);
                            if let Ok(mut synth) = self.lock_synth() {
                                ui.horizontal(|ui| {
                                    ui.label("MODE:");

                                    let was_mono = synth.get_mono_mode();

                                    if ui
                                        .selectable_value(&mut synth.get_mono_mode(), false, "POLY")
                                        .clicked()
                                    {
                                        if was_mono {
                                            synth.set_mono_mode(false);
                                        }
                                    }

                                    if ui
                                        .selectable_value(&mut synth.get_mono_mode(), true, "MONO")
                                        .clicked()
                                    {
                                        if !was_mono {
                                            synth.set_mono_mode(true);
                                        }
                                    }
                                });

                                // Portamento (only visible in MONO mode)
                                if synth.get_mono_mode() {
                                    ui.horizontal(|ui| {
                                        ui.label("PORTAMENTO:");
                                        let mut porta_on = synth.get_portamento_enable();
                                        if ui.checkbox(&mut porta_on, "").changed() {
                                            synth.set_portamento_enable(porta_on);
                                        }

                                        if synth.get_portamento_enable() {
                                            ui.label("TIME:");
                                            let mut porta_time = synth.get_portamento_time();
                                            ui.allocate_ui_with_layout(
                                                egui::vec2(50.0, 20.0),
                                                egui::Layout::left_to_right(egui::Align::Center),
                                                |ui| {
                                                    if ui
                                                        .add(
                                                            egui::Slider::new(
                                                                &mut porta_time,
                                                                0.0..=99.0,
                                                            )
                                                            .show_value(false),
                                                        )
                                                        .changed()
                                                    {
                                                        synth.set_portamento_time(porta_time);
                                                    }
                                                },
                                            );
                                            ui.label(format!("{:.0}", porta_time));
                                        }
                                    });
                                }
                            }
                        });

                        ui.separator();

                        // Right section: Panic and Init buttons
                        ui.vertical(|ui| {
                            ui.set_min_width(100.0);
                            ui.horizontal(|ui| {
                                if ui.small_button("PANIC").clicked() {
                                    if let Ok(mut synth) = self.lock_synth() {
                                        synth.panic();
                                    }
                                }

                                if ui.small_button("INIT").clicked() {
                                    if let Ok(mut synth) = self.lock_synth() {
                                        synth.voice_initialize();
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
        if let Ok(mut synth) = self.lock_synth() {
            ui.horizontal(|ui| {
                ui.label("MODE:");

                let was_mono = synth.get_mono_mode();

                if ui
                    .selectable_value(&mut synth.get_mono_mode(), false, "POLY")
                    .clicked()
                {
                    if was_mono {
                        synth.set_mono_mode(false);
                    }
                }

                if ui
                    .selectable_value(&mut synth.get_mono_mode(), true, "MONO")
                    .clicked()
                {
                    if !was_mono {
                        synth.set_mono_mode(true);
                    }
                }
            });

            // Portamento (only visible in MONO mode)
            if synth.get_mono_mode() {
                ui.horizontal(|ui| {
                    ui.label("PORTA:");
                    let mut porta_on = synth.get_portamento_enable();
                    if ui.checkbox(&mut porta_on, "").changed() {
                        synth.set_portamento_enable(porta_on);
                    }

                    if synth.get_portamento_enable() {
                        ui.label("TIME:");
                        let mut porta_time = synth.get_portamento_time();
                        ui.allocate_ui_with_layout(
                            egui::vec2(40.0, 20.0),
                            egui::Layout::left_to_right(egui::Align::Center),
                            |ui| {
                                if ui
                                    .add(
                                        egui::Slider::new(&mut porta_time, 0.0..=99.0)
                                            .show_value(false),
                                    )
                                    .changed()
                                {
                                    synth.set_portamento_time(porta_time);
                                }
                            },
                        );
                        ui.label(format!("{:.0}", porta_time));
                    }
                });
            }
        }
    }

    fn draw_tune_and_utilities_compact(&mut self, ui: &mut egui::Ui) {
        if let Ok(mut synth) = self.lock_synth() {
            // First row: Master Tune
            ui.horizontal(|ui| {
                ui.label("TUNE:");
                let mut master_tune = synth.get_master_tune();
                ui.allocate_ui_with_layout(
                    egui::vec2(50.0, 20.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        if ui
                            .add(
                                egui::Slider::new(&mut master_tune, -150.0..=150.0)
                                    .show_value(false),
                            )
                            .changed()
                        {
                            synth.set_master_tune(master_tune);
                        }
                    },
                );
                ui.label(format!("{:.0}c", master_tune));

                if ui.small_button("RST").clicked() {
                    synth.set_master_tune(0.0);
                }
            });

            // Second row: Pitch Bend and utilities
            ui.horizontal(|ui| {
                ui.label("BEND:");
                let mut pb_range = synth.get_pitch_bend_range();
                ui.allocate_ui_with_layout(
                    egui::vec2(40.0, 20.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        if ui
                            .add(egui::Slider::new(&mut pb_range, 0.0..=12.0).show_value(false))
                            .changed()
                        {
                            synth.set_pitch_bend_range(pb_range);
                        }
                    },
                );
                ui.label(format!("{:.0}", pb_range));

                ui.separator();

                if ui.small_button("PANIC").clicked() {
                    synth.panic();
                }

                if ui.small_button("INIT").clicked() {
                    synth.voice_initialize();
                }
            });
        }
    }

    fn draw_membrane_buttons(&mut self, ui: &mut egui::Ui) {
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

                let algorithm_button = if self.display_mode == DisplayMode::Algorithm {
                    egui::Button::new("ALGORITHM")
                        .fill(egui::Color32::from_rgb(180, 200, 220))
                        .min_size(button_size)
                } else {
                    egui::Button::new("ALGORITHM").min_size(button_size)
                };

                if ui.add(algorithm_button).clicked() {
                    self.display_mode = DisplayMode::Algorithm;
                    self.display_text = "ALGORITHM SELECT".to_string();
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

                // Only show operator buttons when in Operator mode
                if self.display_mode == DisplayMode::Operator {
                    ui.separator();
                    ui.label("OP:");

                    for i in 1..=6 {
                        let is_selected = self.selected_operator == i - 1;
                        let op_button_size = egui::vec2(25.0, 25.0);
                        let button = if is_selected {
                            egui::Button::new(&format!("{}", i))
                                .fill(egui::Color32::from_rgb(180, 200, 220))
                                .min_size(op_button_size)
                        } else {
                            egui::Button::new(&format!("{}", i)).min_size(op_button_size)
                        };

                        if ui.add(button).clicked() {
                            self.selected_operator = i - 1;
                            self.display_text = format!("OPERATOR {}", i);
                        }
                    }
                }
            });
        });
    }

    fn draw_operator_panel(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.label(format!("OPERATOR {}", self.selected_operator + 1));

                let op_idx = self.selected_operator;

                let (
                    mut freq_ratio,
                    mut output_level,
                    mut detune,
                    mut feedback,
                    mut vel_sens,
                    mut key_scale_lvl,
                    mut key_scale_rt,
                ) = {
                    let synth = self.synthesizer.lock().unwrap();
                    if let Some(voice) = synth.voices.first() {
                        let op = &voice.operators[op_idx];
                        (
                            op.frequency_ratio,
                            op.output_level,
                            op.detune,
                            op.feedback,
                            op.velocity_sensitivity,
                            op.key_scale_level,
                            op.key_scale_rate,
                        )
                    } else {
                        (1.0, 99.0, 0.0, 0.0, 0.0, 0.0, 0.0)
                    }
                };

                ui.columns(2, |columns| {
                    columns[0].label("Frequency Ratio:");
                    if columns[1]
                        .add(
                            egui::Slider::new(&mut freq_ratio, 0.5..=15.0)
                                .step_by(0.5)
                                .show_value(true),
                        )
                        .changed()
                    {
                        self.synthesizer
                            .lock()
                            .unwrap()
                            .set_operator_param(op_idx, "ratio", freq_ratio);
                    }
                });

                ui.columns(2, |columns| {
                    columns[0].label("Output Level:");
                    if columns[1]
                        .add(
                            egui::Slider::new(&mut output_level, 0.0..=99.0)
                                .integer()
                                .show_value(true),
                        )
                        .changed()
                    {
                        self.synthesizer.lock().unwrap().set_operator_param(
                            op_idx,
                            "level",
                            output_level,
                        );
                    }
                });

                ui.columns(2, |columns| {
                    columns[0].label("Detune:");
                    if columns[1]
                        .add(
                            egui::Slider::new(&mut detune, -7.0..=7.0)
                                .integer()
                                .show_value(true),
                        )
                        .changed()
                    {
                        self.synthesizer
                            .lock()
                            .unwrap()
                            .set_operator_param(op_idx, "detune", detune);
                    }
                });

                if self.operator_has_feedback(op_idx) {
                    ui.columns(2, |columns| {
                        columns[0].label("Feedback:");
                        if columns[1]
                            .add(
                                egui::Slider::new(&mut feedback, 0.0..=7.0)
                                    .integer()
                                    .show_value(true),
                            )
                            .changed()
                        {
                            self.synthesizer
                                .lock()
                                .unwrap()
                                .set_operator_param(op_idx, "feedback", feedback);
                        }
                    });
                }

                ui.separator();
                ui.label("Sensitivity:");

                ui.columns(2, |columns| {
                    columns[0].label("Velocity Sens:");
                    if columns[1]
                        .add(
                            egui::Slider::new(&mut vel_sens, 0.0..=7.0)
                                .integer()
                                .show_value(true),
                        )
                        .changed()
                    {
                        self.synthesizer
                            .lock()
                            .unwrap()
                            .set_operator_param(op_idx, "vel_sens", vel_sens);
                    }
                });

                ui.columns(2, |columns| {
                    columns[0].label("Key Scale Level:");
                    if columns[1]
                        .add(
                            egui::Slider::new(&mut key_scale_lvl, 0.0..=99.0)
                                .integer()
                                .show_value(true),
                        )
                        .changed()
                    {
                        self.synthesizer.lock().unwrap().set_operator_param(
                            op_idx,
                            "key_scale_level",
                            key_scale_lvl,
                        );
                    }
                });

                ui.columns(2, |columns| {
                    columns[0].label("Key Scale Rate:");
                    if columns[1]
                        .add(
                            egui::Slider::new(&mut key_scale_rt, 0.0..=7.0)
                                .integer()
                                .show_value(true),
                        )
                        .changed()
                    {
                        self.synthesizer.lock().unwrap().set_operator_param(
                            op_idx,
                            "key_scale_rate",
                            key_scale_rt,
                        );
                    }
                });
            });
        });
    }

    fn draw_preset_selector(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.label("SELECT VOICE:");
                ui.separator();

                // Calculate responsive grid columns based on available width
                let available_width = ui.available_width();
                let button_width = 120.0;
                let button_spacing = 10.0;
                let min_columns = 2;
                let max_columns = 6;

                // Calculate how many columns fit, with padding for margins
                let padding = 40.0; // Account for group padding and margins
                let usable_width = available_width - padding;
                let columns_that_fit =
                    ((usable_width + button_spacing) / (button_width + button_spacing)) as usize;
                let optimal_columns = columns_that_fit.clamp(min_columns, max_columns);

                // Display presets in a responsive grid
                egui::Grid::new("preset_grid")
                    .num_columns(optimal_columns)
                    .spacing([button_spacing, 10.0])
                    .min_col_width(button_width)
                    .max_col_width(button_width)
                    .show(ui, |ui| {
                        for (i, preset) in self.presets.iter().enumerate() {
                            let is_selected = i == self.selected_preset;
                            let button = if is_selected {
                                egui::Button::new(preset.name)
                                    .fill(egui::Color32::from_rgb(180, 200, 220))
                                    .min_size(egui::vec2(button_width, 30.0))
                            } else {
                                egui::Button::new(preset.name)
                                    .min_size(egui::vec2(button_width, 30.0))
                            };

                            if ui.add(button).clicked() {
                                self.selected_preset = i;
                                let preset_name = preset.name.to_string();
                                // Apply preset to synthesizer
                                if let Ok(mut synth) = self.lock_synth() {
                                    preset.apply_to_synth(&mut synth);
                                };
                                self.display_text = format!("LOADED: {}", preset_name);
                            }

                            // End row when we reach the optimal column count
                            if (i + 1) % optimal_columns == 0 {
                                ui.end_row();
                            }
                        }

                        // Handle the last row if it's incomplete
                        let total_presets = self.presets.len();
                        let last_row_items = total_presets % optimal_columns;
                        if last_row_items != 0 {
                            // Add empty cells to complete the last row for better alignment
                            for _ in last_row_items..optimal_columns {
                                ui.add_space(button_width);
                            }
                            ui.end_row();
                        }
                    });

                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Current Voice:");
                    ui.label(egui::RichText::new(self.presets[self.selected_preset].name).strong());
                });
            });
        });
    }

    fn draw_envelope_panel(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.label(format!("ENVELOPE - OP{}", self.selected_operator + 1));

                let op_idx = self.selected_operator;

                let (
                    mut rate1,
                    mut rate2,
                    mut rate3,
                    mut rate4,
                    mut level1,
                    mut level2,
                    mut level3,
                    mut level4,
                ) = {
                    let synth = self.synthesizer.lock().unwrap();
                    if let Some(voice) = synth.voices.first() {
                        let env = &voice.operators[op_idx].envelope;
                        (
                            env.rate1, env.rate2, env.rate3, env.rate4, env.level1, env.level2,
                            env.level3, env.level4,
                        )
                    } else {
                        (99.0, 50.0, 35.0, 50.0, 99.0, 75.0, 50.0, 0.0)
                    }
                };

                ui.vertical(|ui| {
                    ui.label("RATES");
                    ui.columns(2, |columns| {
                        columns[0].label("Attack Rate:");
                        if columns[1]
                            .add(
                                egui::Slider::new(&mut rate1, 0.0..=99.0)
                                    .integer()
                                    .show_value(true),
                            )
                            .changed()
                        {
                            self.synthesizer
                                .lock()
                                .unwrap()
                                .set_envelope_param(op_idx, "rate1", rate1);
                        }
                    });
                    ui.columns(2, |columns| {
                        columns[0].label("Decay Rate:");
                        if columns[1]
                            .add(
                                egui::Slider::new(&mut rate2, 0.0..=99.0)
                                    .integer()
                                    .show_value(true),
                            )
                            .changed()
                        {
                            self.synthesizer
                                .lock()
                                .unwrap()
                                .set_envelope_param(op_idx, "rate2", rate2);
                        }
                    });
                    ui.columns(2, |columns| {
                        columns[0].label("Sustain Rate:");
                        if columns[1]
                            .add(
                                egui::Slider::new(&mut rate3, 0.0..=99.0)
                                    .integer()
                                    .show_value(true),
                            )
                            .changed()
                        {
                            self.synthesizer
                                .lock()
                                .unwrap()
                                .set_envelope_param(op_idx, "rate3", rate3);
                        }
                    });
                    ui.columns(2, |columns| {
                        columns[0].label("Release Rate:");
                        if columns[1]
                            .add(
                                egui::Slider::new(&mut rate4, 0.0..=99.0)
                                    .integer()
                                    .show_value(true),
                            )
                            .changed()
                        {
                            self.synthesizer
                                .lock()
                                .unwrap()
                                .set_envelope_param(op_idx, "rate4", rate4);
                        }
                    });
                });

                ui.vertical(|ui| {
                    ui.label("LEVELS");
                    ui.columns(2, |columns| {
                        columns[0].label("Attack Level:");
                        if columns[1]
                            .add(
                                egui::Slider::new(&mut level1, 0.0..=99.0)
                                    .integer()
                                    .show_value(true),
                            )
                            .changed()
                        {
                            self.synthesizer
                                .lock()
                                .unwrap()
                                .set_envelope_param(op_idx, "level1", level1);
                        }
                    });
                    ui.columns(2, |columns| {
                        columns[0].label("Decay Level:");
                        if columns[1]
                            .add(
                                egui::Slider::new(&mut level2, 0.0..=99.0)
                                    .integer()
                                    .show_value(true),
                            )
                            .changed()
                        {
                            self.synthesizer
                                .lock()
                                .unwrap()
                                .set_envelope_param(op_idx, "level2", level2);
                        }
                    });
                    ui.columns(2, |columns| {
                        columns[0].label("Sustain Level:");
                        if columns[1]
                            .add(
                                egui::Slider::new(&mut level3, 0.0..=99.0)
                                    .integer()
                                    .show_value(true),
                            )
                            .changed()
                        {
                            self.synthesizer
                                .lock()
                                .unwrap()
                                .set_envelope_param(op_idx, "level3", level3);
                        }
                    });
                    ui.columns(2, |columns| {
                        columns[0].label("Release Level:");
                        if columns[1]
                            .add(
                                egui::Slider::new(&mut level4, 0.0..=99.0)
                                    .integer()
                                    .show_value(true),
                            )
                            .changed()
                        {
                            self.synthesizer
                                .lock()
                                .unwrap()
                                .set_envelope_param(op_idx, "level4", level4);
                        }
                    });
                });
            });
        });
    }

    fn draw_algorithm_selector(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.label("ALGORITHM");

                let mut synth = self.synthesizer.lock().unwrap();
                let current_alg = synth.get_algorithm();
                let current_name = algorithms::get_algorithm_name(current_alg).to_string();

                egui::ComboBox::from_label("")
                    .selected_text(format!("{:02} - {}", current_alg, current_name))
                    .show_ui(ui, |ui| {
                        for i in 1..=35 {
                            let alg_name = algorithms::get_algorithm_name(i).to_string();

                            if ui
                                .selectable_value(
                                    &mut current_alg.clone(),
                                    i,
                                    format!("{:02} - {}", i, alg_name),
                                )
                                .clicked()
                            {
                                synth.set_algorithm(i);
                            }
                        }
                    });

                ui.add_space(10.0);
            });
        });
    }

    fn handle_keyboard_input(&mut self, ctx: &egui::Context) {
        use egui::Key;

        let key_map = [
            (Key::Z, 0),     // C
            (Key::S, 1),     // C#
            (Key::X, 2),     // D
            (Key::D, 3),     // D#
            (Key::C, 4),     // E
            (Key::V, 5),     // F
            (Key::G, 6),     // F#
            (Key::B, 7),     // G
            (Key::H, 8),     // G#
            (Key::N, 9),     // A
            (Key::J, 10),    // A#
            (Key::M, 11),    // B
            (Key::Q, 12),    // C (octave up)
            (Key::Num2, 13), // C#
            (Key::W, 14),    // D
            (Key::Num3, 15), // D#
            (Key::E, 16),    // E
            (Key::R, 17),    // F
            (Key::Num5, 18), // F#
            (Key::T, 19),    // G
            (Key::Num6, 20), // G#
            (Key::Y, 21),    // A
            (Key::Num7, 22), // A#
            (Key::U, 23),    // B
        ];

        let now = std::time::Instant::now();

        for (key, semitone) in &key_map {
            if ctx.input(|i| i.key_pressed(*key)) {
                let note = (self.current_octave * 12 + 12 + semitone) as u8;
                self.synthesizer.lock().unwrap().note_on(note, 100);
                self.last_key_times.insert(*key, now);
            } else if ctx.input(|i| i.key_released(*key)) {
                if let Some(&_press_time) = self.last_key_times.get(key) {
                    let note = (self.current_octave * 12 + 12 + semitone) as u8;
                    self.synthesizer.lock().unwrap().note_off(note);
                    self.last_key_times.remove(key);
                }
            }
        }

        if ctx.input(|i| i.key_pressed(Key::ArrowUp)) {
            self.current_octave = (self.current_octave + 1).min(7);
        }
        if ctx.input(|i| i.key_pressed(Key::ArrowDown)) {
            self.current_octave = (self.current_octave - 1).max(0);
        }

        if ctx.input(|i| i.key_pressed(Key::Space)) {
            self.synthesizer.lock().unwrap().panic();
        }
    }
}

impl eframe::App for Dx7App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_keyboard_input(ctx);

        // Set light theme with soft colors
        ctx.set_visuals(egui::Visuals::light());

        egui::CentralPanel::default().show(ctx, |ui| {
            // Header
            ui.vertical_centered(|ui| {
                ui.heading("YAMAHA DX7 DIGITAL SYNTHESIZER");
            });
            ui.separator();

            // DX7 LCD Display
            self.draw_dx7_display(ui);

            ui.add_space(8.0);

            // Global Controls Panel (always visible)
            self.draw_global_controls(ui);

            ui.add_space(8.0);

            // Mode Selection Buttons
            self.draw_membrane_buttons(ui);

            ui.add_space(8.0);

            match self.display_mode {
                DisplayMode::Voice => {
                    self.draw_preset_selector(ui);
                }
                DisplayMode::Algorithm => {
                    self.draw_algorithm_selector(ui);
                }
                DisplayMode::Operator => {
                    // Show operator controls and envelope in two columns
                    ui.columns(2, |columns| {
                        // Left column: Operator parameters
                        columns[0].vertical(|ui| {
                            self.draw_operator_panel(ui);
                        });

                        // Right column: Envelope parameters
                        columns[1].vertical(|ui| {
                            self.draw_envelope_panel(ui);
                        });
                    });
                }
                DisplayMode::LFO => {
                    self.draw_lfo_panel(ui);
                }
            }

            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Keyboard: Z-M (lower octave), Q-U (upper octave)");
                ui.label(format!("| Octave: {}", self.current_octave));
                ui.label("| Space: Panic");
                ui.label("| Up/Down: Change octave");
            });
        });

        // Only repaint when needed (user interaction or animation)
        if ctx.input(|i| !i.events.is_empty()) {
            ctx.request_repaint_after(std::time::Duration::from_millis(16)); // ~60 FPS
        }
    }
}

impl Dx7App {
    fn operator_has_feedback(&self, op_idx: usize) -> bool {
        // Feedback detection will be implemented later with algorithm analysis
        // For now, assume only operator 6 (index 5) has feedback in most algorithms
        op_idx == 5
    }


    fn draw_lfo_panel(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.label("LFO CONTROLS");
                ui.separator();

                // Get current LFO values
                let (
                    mut lfo_rate,
                    mut lfo_delay,
                    mut lfo_pitch_depth,
                    mut lfo_amp_depth,
                    lfo_waveform,
                    mut lfo_key_sync,
                ) = {
                    if let Ok(synth) = self.synthesizer.lock() {
                        (
                            synth.get_lfo_rate(),
                            synth.get_lfo_delay(),
                            synth.get_lfo_pitch_depth(),
                            synth.get_lfo_amp_depth(),
                            synth.get_lfo_waveform(),
                            synth.get_lfo_key_sync(),
                        )
                    } else {
                        (0.0, 0.0, 0.0, 0.0, crate::lfo::LFOWaveform::Triangle, false)
                    }
                };

                ui.columns(2, |columns| {
                    // Left column: Rate and Delay
                    columns[0].vertical(|ui| {
                        ui.label("TIMING");

                        ui.columns(2, |cols| {
                            cols[0].label("Rate:");
                            if cols[1]
                                .add(
                                    egui::Slider::new(&mut lfo_rate, 0.0..=99.0)
                                        .integer()
                                        .show_value(true),
                                )
                                .changed()
                            {
                                if let Ok(mut synth) = self.synthesizer.lock() {
                                    synth.set_lfo_rate(lfo_rate);
                                }
                            }
                        });

                        ui.columns(2, |cols| {
                            cols[0].label("Delay:");
                            if cols[1]
                                .add(
                                    egui::Slider::new(&mut lfo_delay, 0.0..=99.0)
                                        .integer()
                                        .show_value(true),
                                )
                                .changed()
                            {
                                if let Ok(mut synth) = self.synthesizer.lock() {
                                    synth.set_lfo_delay(lfo_delay);
                                }
                            }
                        });

                        // Display current frequency in Hz
                        if let Ok(synth) = self.synthesizer.lock() {
                            let freq_hz = synth.get_lfo_frequency_hz();
                            let delay_sec = synth.get_lfo_delay_seconds();
                            ui.separator();
                            ui.label(format!("Frequency: {:.2} Hz", freq_hz));
                            ui.label(format!("Delay: {:.2} sec", delay_sec));
                        }
                    });

                    // Right column: Depths and Waveform
                    columns[1].vertical(|ui| {
                        ui.label("MODULATION");

                        ui.columns(2, |cols| {
                            cols[0].label("Pitch Depth:");
                            if cols[1]
                                .add(
                                    egui::Slider::new(&mut lfo_pitch_depth, 0.0..=99.0)
                                        .integer()
                                        .show_value(true),
                                )
                                .changed()
                            {
                                if let Ok(mut synth) = self.synthesizer.lock() {
                                    synth.set_lfo_pitch_depth(lfo_pitch_depth);
                                }
                            }
                        });

                        ui.columns(2, |cols| {
                            cols[0].label("Amp Depth:");
                            if cols[1]
                                .add(
                                    egui::Slider::new(&mut lfo_amp_depth, 0.0..=99.0)
                                        .integer()
                                        .show_value(true),
                                )
                                .changed()
                            {
                                if let Ok(mut synth) = self.synthesizer.lock() {
                                    synth.set_lfo_amp_depth(lfo_amp_depth);
                                }
                            }
                        });

                        ui.separator();

                        // Waveform selector
                        ui.horizontal(|ui| {
                            ui.label("Waveform:");
                            egui::ComboBox::from_id_source("lfo_waveform")
                                .selected_text(lfo_waveform.name())
                                .show_ui(ui, |ui| {
                                    for &waveform in crate::lfo::LFOWaveform::all() {
                                        let response = ui.selectable_value(
                                            &mut lfo_waveform.clone(),
                                            waveform,
                                            waveform.name(),
                                        );
                                        if response.clicked() {
                                            if let Ok(mut synth) = self.synthesizer.lock() {
                                                synth.set_lfo_waveform(waveform);
                                            }
                                        }
                                    }
                                });
                        });

                        // Key sync checkbox
                        ui.horizontal(|ui| {
                            ui.label("Key Sync:");
                            if ui.checkbox(&mut lfo_key_sync, "").changed() {
                                if let Ok(mut synth) = self.synthesizer.lock() {
                                    synth.set_lfo_key_sync(lfo_key_sync);
                                }
                            }
                        });
                    });
                });

                ui.separator();

                // Mod Wheel status display
                if let Ok(synth) = self.synthesizer.lock() {
                    let mod_wheel_percent = (synth.get_mod_wheel() * 100.0) as i32;
                    ui.horizontal(|ui| {
                        ui.label("Mod Wheel:");
                        ui.label(format!("{}%", mod_wheel_percent));
                        if mod_wheel_percent == 0 {
                            ui.label("(LFO effect disabled - move mod wheel)");
                        } else {
                            ui.label("(LFO active)");
                        }
                    });
                }
            });
        });
    }
}
