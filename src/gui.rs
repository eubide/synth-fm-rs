use crate::algorithms::Algorithm;
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
    // Cache for algorithm diagrams
    cached_algorithm: Option<u8>,
    cached_diagram: Option<crate::algorithms::AlgorithmGraph>,
}

#[derive(PartialEq)]
enum DisplayMode {
    Voice,
    Operator,
    Algorithm,
    Function,
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
            cached_algorithm: None,
            cached_diagram: None,
        }
    }

    fn lock_synth(&self) -> Result<std::sync::MutexGuard<FmSynthesizer>, std::sync::PoisonError<std::sync::MutexGuard<FmSynthesizer>>> {
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
                            format!("VOICE: {} | ALG: {:02}", synth.preset_name, synth.algorithm)
                        } else {
                            "VOICE: ERROR".to_string()
                        }
                    }
                    DisplayMode::Operator => {
                        format!("OP{} EDIT", self.selected_operator + 1)
                    }
                    DisplayMode::Algorithm => {
                        if let Ok(synth) = self.lock_synth() {
                            let algorithms = Algorithm::get_all_algorithms();
                            let alg_name = algorithms
                                .iter()
                                .find(|a| a.number == synth.algorithm)
                                .map(|a| a.name.clone())
                                .unwrap_or_else(|| "Unknown".to_string());
                            format!("ALG {} - {}", synth.algorithm, alg_name)
                        } else {
                            "ALGORITHM: ERROR".to_string()
                        }
                    }
                    DisplayMode::Function => "FUNCTION MODE".to_string(),
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
                let mode_text = if synth.mono_mode { "MONO" } else { "POLY" };
                let midi_text = if self._midi_handler.is_some() { "MIDI OK" } else { "NO MIDI" };
                
                let status_line = if synth.mono_mode {
                    // Show portamento only in MONO mode
                    let porta_text = if synth.portamento_enable { "ON" } else { "OFF" };
                    format!(
                        "VOICE: {} | ALG: {:02} | MODE: {} | PORTA: {} | {}",
                        synth.preset_name, synth.algorithm, mode_text, porta_text, midi_text
                    )
                } else {
                    // In POLY mode, don't show portamento
                    format!(
                        "VOICE: {} | ALG: {:02} | MODE: {} | {}",
                        synth.preset_name, synth.algorithm, mode_text, midi_text
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

    fn draw_membrane_buttons(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().button_padding = egui::vec2(8.0, 4.0);

            let voice_button = if self.display_mode == DisplayMode::Voice {
                egui::Button::new("VOICE").fill(egui::Color32::from_rgb(180, 200, 220))
            } else {
                egui::Button::new("VOICE")
            };

            if ui.add(voice_button).clicked() {
                self.display_mode = DisplayMode::Voice;
                self.display_text = "VOICE SELECT".to_string();
            }

            let algorithm_button = if self.display_mode == DisplayMode::Algorithm {
                egui::Button::new("ALGORITHM").fill(egui::Color32::from_rgb(180, 200, 220))
            } else {
                egui::Button::new("ALGORITHM")
            };

            if ui.add(algorithm_button).clicked() {
                self.display_mode = DisplayMode::Algorithm;
                self.display_text = "ALGORITHM SELECT".to_string();
            }

            let op_select_button = if self.display_mode == DisplayMode::Operator {
                egui::Button::new("OPERATOR").fill(egui::Color32::from_rgb(180, 200, 220))
            } else {
                egui::Button::new("OPERATOR")
            };

            if ui.add(op_select_button).clicked() {
                self.display_mode = DisplayMode::Operator;
                self.display_text = format!("OPERATOR {}", self.selected_operator + 1);
            }

            let function_button = if self.display_mode == DisplayMode::Function {
                egui::Button::new("FUNCTION").fill(egui::Color32::from_rgb(180, 200, 220))
            } else {
                egui::Button::new("FUNCTION")
            };

            if ui.add(function_button).clicked() {
                self.display_mode = DisplayMode::Function;
                self.display_text = "FUNCTION MODE".to_string();
            }

            // Only show operator buttons when in Operator mode
            if self.display_mode == DisplayMode::Operator {
                ui.separator();

                for i in 1..=6 {
                    let is_selected = self.selected_operator == i - 1;
                    let button = if is_selected {
                        egui::Button::new(&format!("{}", i))
                            .fill(egui::Color32::from_rgb(180, 200, 220))
                    } else {
                        egui::Button::new(&format!("{}", i))
                    };

                    if ui.add(button).clicked() {
                        self.selected_operator = i - 1;
                        self.display_text = format!("OPERATOR {}", i);
                    }
                }
            }
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

                ui.horizontal(|ui| {
                    ui.label("Frequency Ratio:");
                    if ui
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

                ui.horizontal(|ui| {
                    ui.label("Output Level:");
                    if ui
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

                ui.horizontal(|ui| {
                    ui.label("Detune:");
                    if ui
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
                    ui.horizontal(|ui| {
                        ui.label("Feedback:");
                        if ui
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

                ui.horizontal(|ui| {
                    ui.label("Velocity Sens:");
                    if ui
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

                ui.horizontal(|ui| {
                    ui.label("Key Scale Level:");
                    if ui
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

                ui.horizontal(|ui| {
                    ui.label("Key Scale Rate:");
                    if ui
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

                // Display presets in a grid
                egui::Grid::new("preset_grid")
                    .num_columns(4)
                    .spacing([10.0, 10.0])
                    .show(ui, |ui| {
                        for (i, preset) in self.presets.iter().enumerate() {
                            let is_selected = i == self.selected_preset;
                            let button = if is_selected {
                                egui::Button::new(preset.name)
                                    .fill(egui::Color32::from_rgb(180, 200, 220))
                                    .min_size(egui::vec2(120.0, 30.0))
                            } else {
                                egui::Button::new(preset.name).min_size(egui::vec2(120.0, 30.0))
                            };

                            if ui.add(button).clicked() {
                                self.selected_preset = i;
                                // Apply preset to synthesizer
                                let mut synth = self.synthesizer.lock().unwrap();
                                preset.apply_to_synth(&mut synth);
                                self.display_text = format!("LOADED: {}", preset.name);
                            }

                            if (i + 1) % 4 == 0 {
                                ui.end_row();
                            }
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

                ui.columns(2, |columns| {
                    columns[0].vertical(|ui| {
                        ui.label("RATES");

                        if ui
                            .add(
                                egui::Slider::new(&mut rate1, 0.0..=99.0)
                                    .text("R1")
                                    .integer(),
                            )
                            .changed()
                        {
                            self.synthesizer
                                .lock()
                                .unwrap()
                                .set_envelope_param(op_idx, "rate1", rate1);
                        }

                        if ui
                            .add(
                                egui::Slider::new(&mut rate2, 0.0..=99.0)
                                    .text("R2")
                                    .integer(),
                            )
                            .changed()
                        {
                            self.synthesizer
                                .lock()
                                .unwrap()
                                .set_envelope_param(op_idx, "rate2", rate2);
                        }

                        if ui
                            .add(
                                egui::Slider::new(&mut rate3, 0.0..=99.0)
                                    .text("R3")
                                    .integer(),
                            )
                            .changed()
                        {
                            self.synthesizer
                                .lock()
                                .unwrap()
                                .set_envelope_param(op_idx, "rate3", rate3);
                        }

                        if ui
                            .add(
                                egui::Slider::new(&mut rate4, 0.0..=99.0)
                                    .text("R4")
                                    .integer(),
                            )
                            .changed()
                        {
                            self.synthesizer
                                .lock()
                                .unwrap()
                                .set_envelope_param(op_idx, "rate4", rate4);
                        }
                    });

                    columns[1].vertical(|ui| {
                        ui.label("LEVELS");

                        if ui
                            .add(
                                egui::Slider::new(&mut level1, 0.0..=99.0)
                                    .text("L1")
                                    .integer(),
                            )
                            .changed()
                        {
                            self.synthesizer
                                .lock()
                                .unwrap()
                                .set_envelope_param(op_idx, "level1", level1);
                        }

                        if ui
                            .add(
                                egui::Slider::new(&mut level2, 0.0..=99.0)
                                    .text("L2")
                                    .integer(),
                            )
                            .changed()
                        {
                            self.synthesizer
                                .lock()
                                .unwrap()
                                .set_envelope_param(op_idx, "level2", level2);
                        }

                        if ui
                            .add(
                                egui::Slider::new(&mut level3, 0.0..=99.0)
                                    .text("L3")
                                    .integer(),
                            )
                            .changed()
                        {
                            self.synthesizer
                                .lock()
                                .unwrap()
                                .set_envelope_param(op_idx, "level3", level3);
                        }

                        if ui
                            .add(
                                egui::Slider::new(&mut level4, 0.0..=99.0)
                                    .text("L4")
                                    .integer(),
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
                let algorithms = Algorithm::get_all_algorithms();

                egui::ComboBox::from_label("")
                    .selected_text(format!(
                        "{:02} - {}",
                        synth.algorithm,
                        algorithms
                            .iter()
                            .find(|a| a.number == synth.algorithm)
                            .map(|a| a.name.as_str())
                            .unwrap_or("Unknown")
                    ))
                    .show_ui(ui, |ui| {
                        for alg in &algorithms {
                            if ui
                                .selectable_value(
                                    &mut synth.algorithm,
                                    alg.number,
                                    format!("{:02} - {}", alg.number, alg.name),
                                )
                                .clicked()
                            {
                                synth.set_algorithm(alg.number);
                            }
                        }
                    });

                ui.add_space(10.0);
                ui.label("Master Volume:");
                ui.add(egui::Slider::new(&mut synth.master_volume, 0.0..=1.0).show_value(true));
            });
        });
    }

    fn draw_function_panel(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.label("FUNCTION MODE");
                ui.separator();

                // Master Tune
                let mut synth = self.synthesizer.lock().unwrap();
                ui.horizontal(|ui| {
                    ui.label("Master Tune:");
                    let mut master_tune = synth.master_tune;
                    if ui
                        .add(
                            egui::Slider::new(&mut master_tune, -150.0..=150.0)
                                .text("cents")
                                .show_value(true),
                        )
                        .changed()
                    {
                        synth.set_master_tune(master_tune);
                    }
                });

                ui.add_space(10.0);

                // Reset Master Tune button
                ui.horizontal(|ui| {
                    if ui.button("Reset Tune").clicked() {
                        synth.set_master_tune(0.0);
                    }
                    ui.label("(Reset to A440)");
                });

                ui.add_space(10.0);
                ui.separator();

                // Poly/Mono Mode
                ui.horizontal(|ui| {
                    ui.label("Mode:");
                    let mut mono_mode = synth.mono_mode;

                    if ui.selectable_value(&mut mono_mode, false, "POLY").clicked() {
                        synth.set_mono_mode(false);
                    }

                    if ui.selectable_value(&mut mono_mode, true, "MONO").clicked() {
                        synth.set_mono_mode(true);
                    }
                });

                ui.add_space(10.0);
                ui.separator();

                // Pitch Bend Range
                ui.horizontal(|ui| {
                    ui.label("Pitch Bend Range:");
                    let mut pb_range = synth.pitch_bend_range;
                    if ui
                        .add(
                            egui::Slider::new(&mut pb_range, 0.0..=12.0)
                                .text("semitones")
                                .integer()
                                .show_value(true),
                        )
                        .changed()
                    {
                        synth.set_pitch_bend_range(pb_range);
                    }
                });

                ui.add_space(10.0);
                ui.separator();

                // Portamento (only in Mono mode)
                if synth.mono_mode {
                    ui.horizontal(|ui| {
                        ui.label("Portamento:");
                        let mut porta_enable = synth.portamento_enable;
                        if ui.checkbox(&mut porta_enable, "Enable").changed() {
                            synth.set_portamento_enable(porta_enable);
                        }
                    });

                    if synth.portamento_enable {
                        ui.horizontal(|ui| {
                            ui.label("Portamento Time:");
                            let mut porta_time = synth.portamento_time;
                            if ui
                                .add(
                                    egui::Slider::new(&mut porta_time, 0.0..=99.0)
                                        .integer()
                                        .show_value(true),
                                )
                                .changed()
                            {
                                synth.set_portamento_time(porta_time);
                            }
                        });
                    }
                } else {
                    ui.label("Portamento: Only available in MONO mode");
                }

                ui.add_space(10.0);
                ui.separator();

                // Voice Initialize
                ui.horizontal(|ui| {
                    if ui.button("VOICE INIT").clicked() {
                        synth.voice_initialize();
                    }
                    ui.label("Reset to basic DX7 init voice");
                });
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
            ui.heading("YAMAHA DX7 DIGITAL SYNTHESIZER");
            ui.separator();

            self.draw_dx7_display(ui);

            ui.add_space(10.0);

            self.draw_membrane_buttons(ui);

            ui.add_space(10.0);

            match self.display_mode {
                DisplayMode::Voice => {
                    self.draw_preset_selector(ui);
                }
                DisplayMode::Algorithm => {
                    // Algorithm controls + visualization
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            self.draw_algorithm_selector(ui);
                        });
                        ui.separator();
                        ui.vertical(|ui| {
                            self.draw_algorithm_diagram(ui);
                        });
                    });
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
                DisplayMode::Function => {
                    // Function mode - global controls
                    self.draw_function_panel(ui);
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
        let synth = self.synthesizer.lock().unwrap();
        let current_algorithm = synth.algorithm;
        drop(synth);
        
        if let Some(algorithm_def) = crate::algorithms::find_algorithm(current_algorithm) {
            let op_number = (op_idx + 1) as u8;
            
            // Check if this operator has feedback:
            // 1. Self-feedback (from == to, same operator)
            // 2. Cross-operator feedback where this operator receives feedback from a carrier
            algorithm_def.connections.iter().any(|conn| {
                // Case 1: Self-feedback (operator feeds back to itself)
                (conn.from == op_number && conn.to == op_number) ||
                // Case 2: Cross-operator feedback (this operator receives feedback from another)
                (conn.to == op_number && algorithm_def.carriers.contains(&conn.from))
            })
        } else {
            false
        }
    }

    fn draw_algorithm_diagram(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.label("ALGORITHM DIAGRAM");

                let synth = self.synthesizer.lock().unwrap();
                let current_algorithm = synth.algorithm;
                drop(synth);

                // Check if we need to regenerate the diagram
                let positioned_graph = if self.cached_algorithm != Some(current_algorithm) {
                    let graph = Algorithm::parse_algorithm_graph(current_algorithm);
                    let canvas_size = (400.0, 280.0);
                    let positioned = Algorithm::calculate_layout(graph, canvas_size);
                    self.cached_algorithm = Some(current_algorithm);
                    self.cached_diagram = Some(positioned.clone());
                    positioned
                } else if let Some(ref cached) = self.cached_diagram {
                    cached.clone()
                } else {
                    // Fallback
                    let graph = Algorithm::parse_algorithm_graph(current_algorithm);
                    let canvas_size = (400.0, 280.0);
                    Algorithm::calculate_layout(graph, canvas_size)
                };

                // Create drawing area (centered)
                let canvas_size = (400.0, 280.0);

                let (response, painter) = ui
                    .allocate_ui_with_layout(
                        egui::Vec2::new(ui.available_width(), canvas_size.1),
                        egui::Layout::top_down(egui::Align::Center),
                        |ui| {
                            ui.allocate_painter(
                                egui::Vec2::new(canvas_size.0, canvas_size.1),
                                egui::Sense::hover(),
                            )
                        },
                    )
                    .inner;

                let rect = response.rect;

                // Background
                painter.rect_filled(
                    rect,
                    egui::Rounding::same(5.0),
                    egui::Color32::from_gray(20),
                );
                painter.rect_stroke(
                    rect,
                    egui::Rounding::same(5.0),
                    egui::Stroke::new(1.0, egui::Color32::from_gray(100)),
                );

                // Draw carrier sum line first (at the bottom)
                let carriers: Vec<_> = positioned_graph
                    .operators
                    .iter()
                    .filter(|op| op.is_carrier)
                    .collect();
                if carriers.len() > 1 {
                    // Draw horizontal line connecting all carriers at the bottom
                    let carrier_y = carriers
                        .iter()
                        .map(|op| op.position.1)
                        .fold(f32::NEG_INFINITY, f32::max);
                    let sum_y = rect.top() + carrier_y + 35.0;

                    let min_x = carriers
                        .iter()
                        .map(|op| op.position.0)
                        .fold(f32::INFINITY, f32::min);
                    let max_x = carriers
                        .iter()
                        .map(|op| op.position.0)
                        .fold(f32::NEG_INFINITY, f32::max);

                    // Main sum line (thinner)
                    painter.line_segment(
                        [
                            egui::Pos2::new(rect.left() + min_x, sum_y),
                            egui::Pos2::new(rect.left() + max_x, sum_y),
                        ],
                        egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 200, 100)),
                    );

                    // Vertical lines from each carrier to sum line (thinner)
                    for carrier in &carriers {
                        let carrier_pos = egui::Pos2::new(
                            rect.left() + carrier.position.0,
                            rect.top() + carrier.position.1,
                        );
                        let sum_connection =
                            egui::Pos2::new(rect.left() + carrier.position.0, sum_y);

                        painter.line_segment(
                            [carrier_pos, sum_connection],
                            egui::Stroke::new(1.5, egui::Color32::from_rgb(255, 200, 100)),
                        );
                    }

                    // Smaller output label
                    painter.text(
                        egui::Pos2::new(rect.left() + (min_x + max_x) / 2.0, sum_y + 10.0),
                        egui::Align2::CENTER_CENTER,
                        "OUT",
                        egui::FontId::proportional(9.0),
                        egui::Color32::from_rgb(255, 200, 100),
                    );
                }

                // Draw connections (behind operators)
                for connection in &positioned_graph.connections {
                    if let (Some(from_op), Some(to_op)) = (
                        positioned_graph
                            .operators
                            .iter()
                            .find(|op| op.id == connection.from),
                        positioned_graph
                            .operators
                            .iter()
                            .find(|op| op.id == connection.to),
                    ) {
                        let from_pos = egui::Pos2::new(
                            rect.left() + from_op.position.0,
                            rect.top() + from_op.position.1,
                        );
                        let to_pos = egui::Pos2::new(
                            rect.left() + to_op.position.0,
                            rect.top() + to_op.position.1,
                        );

                        if connection.is_feedback {
                            let color = egui::Color32::from_rgb(255, 150, 150);

                            // Check if this is a self-loop (from == to)
                            if connection.from == connection.to {
                                // Draw external horizontal feedback line for self-loops
                                self.draw_self_loop_feedback(&painter, from_pos, color);
                            } else {
                                // Draw curved line for cross-operator feedback (legacy)
                                let op_radius = 12.5; // Half of op_size (25.0)
                                let (from_edge, to_edge) = self
                                    .calculate_edge_connection_points(from_pos, to_pos, op_radius);

                                let curve_offset = 20.0; // Smaller curve
                                let control1 = egui::Pos2::new(
                                    from_edge.x + curve_offset,
                                    from_edge.y - curve_offset,
                                );
                                let control2 = egui::Pos2::new(
                                    to_edge.x - curve_offset,
                                    to_edge.y - curve_offset,
                                );

                                // Approximate curve with fewer segments
                                let segments = 8;
                                for i in 0..segments {
                                    let t1 = i as f32 / segments as f32;
                                    let t2 = (i + 1) as f32 / segments as f32;

                                    let p1 =
                                        bezier_point(from_edge, control1, control2, to_edge, t1);
                                    let p2 =
                                        bezier_point(from_edge, control1, control2, to_edge, t2);

                                    painter.line_segment([p1, p2], egui::Stroke::new(1.5, color));
                                }

                                // Smaller arrow head for feedback
                                self.draw_arrow_head(
                                    &painter,
                                    bezier_point(from_edge, control1, control2, to_edge, 0.9),
                                    bezier_point(from_edge, control1, control2, to_edge, 1.0),
                                    color,
                                );
                            }
                        } else {
                            // Regular connection (thinner) - connect at edges
                            let color = egui::Color32::from_rgb(150, 150, 255);
                            let op_radius = 12.5; // Half of op_size (25.0)
                            let (from_edge, to_edge) =
                                self.calculate_edge_connection_points(from_pos, to_pos, op_radius);

                            painter
                                .line_segment([from_edge, to_edge], egui::Stroke::new(1.5, color));

                            // Smaller arrow head at the edge
                            self.draw_arrow_head(&painter, from_edge, to_edge, color);
                        }
                    }
                }

                // Draw operators
                for op in &positioned_graph.operators {
                    let pos =
                        egui::Pos2::new(rect.left() + op.position.0, rect.top() + op.position.1);

                    let op_size = 25.0;
                    let op_rect = egui::Rect::from_center_size(pos, egui::Vec2::splat(op_size));

                    // Color based on type
                    let (fill_color, text_color) = if op.is_carrier {
                        (egui::Color32::from_rgb(100, 150, 255), egui::Color32::WHITE)
                    // Blue for carriers
                    } else {
                        (egui::Color32::from_rgb(120, 120, 120), egui::Color32::WHITE)
                        // Gray for modulators
                    };

                    // Draw operator box
                    painter.rect_filled(op_rect, egui::Rounding::same(5.0), fill_color);
                    painter.rect_stroke(
                        op_rect,
                        egui::Rounding::same(5.0),
                        egui::Stroke::new(2.0, egui::Color32::WHITE),
                    );

                    // Draw operator number
                    let text = op.id.to_string();
                    painter.text(
                        pos,
                        egui::Align2::CENTER_CENTER,
                        text,
                        egui::FontId::proportional(12.0),
                        text_color,
                    );
                }

                // Legend (centered)
                ui.allocate_ui_with_layout(
                    egui::Vec2::new(ui.available_width(), 20.0),
                    egui::Layout::top_down(egui::Align::Center),
                    |ui| {
                        ui.horizontal(|ui| {
                            ui.colored_label(egui::Color32::from_rgb(100, 150, 255), "■");
                            ui.label("Carriers");
                            ui.add_space(15.0);
                            ui.colored_label(egui::Color32::from_rgb(120, 120, 120), "■");
                            ui.label("Modulators");
                            ui.add_space(15.0);
                            ui.colored_label(egui::Color32::from_rgb(255, 150, 150), "→");
                            ui.label("Feedback");
                            ui.add_space(15.0);
                            ui.colored_label(egui::Color32::from_rgb(255, 200, 100), "━");
                            ui.label("Output Sum");
                        });
                    },
                );
            });
        });
    }

    fn draw_arrow_head(
        &self,
        painter: &egui::Painter,
        from: egui::Pos2,
        to: egui::Pos2,
        color: egui::Color32,
    ) {
        let direction = (to - from).normalized();
        let perpendicular = egui::Vec2::new(-direction.y, direction.x);

        let arrow_size = 5.0; // Smaller arrow
        let arrow_tip = to - direction * 3.0; // Smaller offset

        let arrow_left = arrow_tip - direction * arrow_size + perpendicular * (arrow_size * 0.4);
        let arrow_right = arrow_tip - direction * arrow_size - perpendicular * (arrow_size * 0.4);

        painter.line_segment([arrow_left, arrow_tip], egui::Stroke::new(1.5, color));
        painter.line_segment([arrow_right, arrow_tip], egui::Stroke::new(1.5, color));
    }

    fn calculate_edge_connection_points(
        &self,
        from_center: egui::Pos2,
        to_center: egui::Pos2,
        op_radius: f32,
    ) -> (egui::Pos2, egui::Pos2) {
        let direction = (to_center - from_center).normalized();

        // Calculate edge points
        let from_edge = from_center + direction * op_radius;
        let to_edge = to_center - direction * op_radius;

        (from_edge, to_edge)
    }

    fn draw_self_loop_feedback(
        &self,
        painter: &egui::Painter,
        op_pos: egui::Pos2,
        color: egui::Color32,
    ) {
        let op_radius = 12.5; // Half of op_size (25.0)
        let feedback_line_length = 40.0; // Length of horizontal feedback line
        let external_offset = 25.0; // Distance from operator edge to feedback line

        // Position the horizontal line to the right of the operator, at external offset
        let line_start_x = op_pos.x + op_radius + external_offset;
        let line_end_x = line_start_x + feedback_line_length;
        let line_y = op_pos.y; // Same Y level as operator

        let line_start = egui::Pos2::new(line_start_x, line_y);
        let line_end = egui::Pos2::new(line_end_x, line_y);

        // Draw the main horizontal feedback line
        painter.line_segment([line_start, line_end], egui::Stroke::new(1.5, color));

        // Draw connector from operator to feedback line
        let connector_start = egui::Pos2::new(op_pos.x + op_radius, op_pos.y);
        painter.line_segment([connector_start, line_start], egui::Stroke::new(1.5, color));

        // Draw return connector from feedback line back to operator
        let return_connector_end = egui::Pos2::new(op_pos.x + op_radius, op_pos.y - 8.0); // Slightly above
        painter.line_segment(
            [line_end, return_connector_end],
            egui::Stroke::new(1.5, color),
        );

        // Draw arrow head at the return point
        self.draw_arrow_head(painter, line_end, return_connector_end, color);
    }
}

// Helper function for cubic Bezier curves
fn bezier_point(
    p0: egui::Pos2,
    p1: egui::Pos2,
    p2: egui::Pos2,
    p3: egui::Pos2,
    t: f32,
) -> egui::Pos2 {
    let u = 1.0 - t;
    let tt = t * t;
    let uu = u * u;
    let uuu = uu * u;
    let ttt = tt * t;

    let mut point = p0.to_vec2() * uuu;
    point += p1.to_vec2() * (3.0 * uu * t);
    point += p2.to_vec2() * (3.0 * u * tt);
    point += p3.to_vec2() * ttt;

    egui::Pos2::new(point.x, point.y)
}
