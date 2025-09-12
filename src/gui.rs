use eframe::egui;
use std::sync::{Arc, Mutex};
use crate::fm_synth::FmSynthesizer;
use crate::audio_engine::AudioEngine;
use crate::midi_handler::MidiHandler;
use crate::algorithms::Algorithm;
use crate::presets::{get_dx7_presets, Dx7Preset};

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
    Function,
}

impl Dx7App {
    pub fn new(synthesizer: Arc<Mutex<FmSynthesizer>>, audio_engine: AudioEngine, midi_handler: Option<MidiHandler>) -> Self {
        let presets = get_dx7_presets();
        
        // Apply the first preset (E.PIANO 1)
        if !presets.is_empty() {
            let mut synth = synthesizer.lock().unwrap();
            presets[0].apply_to_synth(&mut synth);
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
    
    fn draw_dx7_display(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            // Light background like classic LCD
            ui.style_mut().visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(230, 240, 235);
            ui.style_mut().visuals.widgets.noninteractive.fg_stroke.color = egui::Color32::from_rgb(30, 30, 30);
            
            ui.set_min_height(60.0);
            ui.vertical_centered(|ui| {
                ui.add_space(5.0);
                
                let display_font = egui::FontId::new(16.0, egui::FontFamily::Monospace);
                let display_color = egui::Color32::from_rgb(30, 30, 30);
                
                ui.label(egui::RichText::new(&self.display_text)
                    .font(display_font.clone())
                    .color(display_color));
                
                let sub_text = match self.display_mode {
                    DisplayMode::Voice => {
                        let synth = self.synthesizer.lock().unwrap();
                        format!("VOICE: {} | ALG: {:02}", synth.preset_name, synth.algorithm)
                    }
                    DisplayMode::Operator => {
                        format!("OP{} EDIT", self.selected_operator + 1)
                    }
                    DisplayMode::Algorithm => {
                        let synth = self.synthesizer.lock().unwrap();
                        let algorithms = Algorithm::get_all_algorithms();
                        let alg_name = algorithms.iter()
                            .find(|a| a.number == synth.algorithm)
                            .map(|a| a.name.clone())
                            .unwrap_or_else(|| "Unknown".to_string());
                        format!("ALG {} - {}", synth.algorithm, alg_name)
                    }
                    DisplayMode::Function => {
                        "FUNCTION MODE".to_string()
                    }
                };
                
                ui.label(egui::RichText::new(sub_text)
                    .font(display_font)
                    .color(display_color));
            });
        });
    }
    
    fn draw_membrane_buttons(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().button_padding = egui::vec2(8.0, 4.0);
            
            let voice_button = if self.display_mode == DisplayMode::Voice {
                egui::Button::new("VOICE")
                    .fill(egui::Color32::from_rgb(180, 200, 220))
            } else {
                egui::Button::new("VOICE")
            };
            
            if ui.add(voice_button).clicked() {
                self.display_mode = DisplayMode::Voice;
                self.display_text = "VOICE SELECT".to_string();
            }
            
            let algorithm_button = if self.display_mode == DisplayMode::Algorithm {
                egui::Button::new("ALGORITHM")
                    .fill(egui::Color32::from_rgb(180, 200, 220))
            } else {
                egui::Button::new("ALGORITHM")
            };
            
            if ui.add(algorithm_button).clicked() {
                self.display_mode = DisplayMode::Algorithm;
                self.display_text = "ALGORITHM SELECT".to_string();
            }
            
            let op_select_button = if self.display_mode == DisplayMode::Operator {
                egui::Button::new("OPERATOR")
                    .fill(egui::Color32::from_rgb(180, 200, 220))
            } else {
                egui::Button::new("OPERATOR")
            };
            
            if ui.add(op_select_button).clicked() {
                self.display_mode = DisplayMode::Operator;
                self.display_text = format!("OPERATOR {}", self.selected_operator + 1);
            }
            
            let function_button = if self.display_mode == DisplayMode::Function {
                egui::Button::new("FUNCTION")
                    .fill(egui::Color32::from_rgb(180, 200, 220))
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
                
                let (mut freq_ratio, mut output_level, mut detune, mut feedback, 
                     mut vel_sens, mut key_scale_lvl, mut key_scale_rt) = {
                    let synth = self.synthesizer.lock().unwrap();
                    if let Some(voice) = synth.voices.first() {
                        let op = &voice.operators[op_idx];
                        (op.frequency_ratio, op.output_level, op.detune, op.feedback,
                         op.velocity_sensitivity, op.key_scale_level, op.key_scale_rate)
                    } else {
                        (1.0, 99.0, 0.0, 0.0, 0.0, 0.0, 0.0)
                    }
                };
                
                ui.horizontal(|ui| {
                    ui.label("Frequency Ratio:");
                    if ui.add(egui::Slider::new(&mut freq_ratio, 0.5..=15.0)
                        .step_by(0.5)
                        .show_value(true)).changed() {
                        self.synthesizer.lock().unwrap().set_operator_param(op_idx, "ratio", freq_ratio);
                    }
                });
                
                ui.horizontal(|ui| {
                    ui.label("Output Level:");
                    if ui.add(egui::Slider::new(&mut output_level, 0.0..=99.0)
                        .integer()
                        .show_value(true)).changed() {
                        self.synthesizer.lock().unwrap().set_operator_param(op_idx, "level", output_level);
                    }
                });
                
                ui.horizontal(|ui| {
                    ui.label("Detune:");
                    if ui.add(egui::Slider::new(&mut detune, -7.0..=7.0)
                        .integer()
                        .show_value(true)).changed() {
                        self.synthesizer.lock().unwrap().set_operator_param(op_idx, "detune", detune);
                    }
                });
                
                if op_idx == 5 {
                    ui.horizontal(|ui| {
                        ui.label("Feedback:");
                        if ui.add(egui::Slider::new(&mut feedback, 0.0..=7.0)
                            .integer()
                            .show_value(true)).changed() {
                            self.synthesizer.lock().unwrap().set_operator_param(op_idx, "feedback", feedback);
                        }
                    });
                }
                
                ui.separator();
                ui.label("Sensitivity:");
                
                ui.horizontal(|ui| {
                    ui.label("Velocity Sens:");
                    if ui.add(egui::Slider::new(&mut vel_sens, 0.0..=7.0)
                        .integer()
                        .show_value(true)).changed() {
                        self.synthesizer.lock().unwrap().set_operator_param(op_idx, "vel_sens", vel_sens);
                    }
                });
                
                ui.horizontal(|ui| {
                    ui.label("Key Scale Level:");
                    if ui.add(egui::Slider::new(&mut key_scale_lvl, 0.0..=99.0)
                        .integer()
                        .show_value(true)).changed() {
                        self.synthesizer.lock().unwrap().set_operator_param(op_idx, "key_scale_level", key_scale_lvl);
                    }
                });
                
                ui.horizontal(|ui| {
                    ui.label("Key Scale Rate:");
                    if ui.add(egui::Slider::new(&mut key_scale_rt, 0.0..=7.0)
                        .integer()
                        .show_value(true)).changed() {
                        self.synthesizer.lock().unwrap().set_operator_param(op_idx, "key_scale_rate", key_scale_rt);
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
                                egui::Button::new(preset.name)
                                    .min_size(egui::vec2(120.0, 30.0))
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
                    ui.label(egui::RichText::new(self.presets[self.selected_preset].name)
                        .strong());
                });
            });
        });
    }
    
    fn draw_envelope_panel(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.label(format!("ENVELOPE - OP{}", self.selected_operator + 1));
                
                let op_idx = self.selected_operator;
                
                let (mut rate1, mut rate2, mut rate3, mut rate4,
                     mut level1, mut level2, mut level3, mut level4) = {
                    let synth = self.synthesizer.lock().unwrap();
                    if let Some(voice) = synth.voices.first() {
                        let env = &voice.operators[op_idx].envelope;
                        (env.rate1, env.rate2, env.rate3, env.rate4,
                         env.level1, env.level2, env.level3, env.level4)
                    } else {
                        (99.0, 50.0, 35.0, 50.0, 99.0, 75.0, 50.0, 0.0)
                    }
                };
                
                ui.columns(2, |columns| {
                    columns[0].vertical(|ui| {
                        ui.label("RATES");
                        
                        if ui.add(egui::Slider::new(&mut rate1, 0.0..=99.0)
                            .text("R1")
                            .integer()).changed() {
                            self.synthesizer.lock().unwrap().set_envelope_param(op_idx, "rate1", rate1);
                        }
                        
                        if ui.add(egui::Slider::new(&mut rate2, 0.0..=99.0)
                            .text("R2")
                            .integer()).changed() {
                            self.synthesizer.lock().unwrap().set_envelope_param(op_idx, "rate2", rate2);
                        }
                        
                        if ui.add(egui::Slider::new(&mut rate3, 0.0..=99.0)
                            .text("R3")
                            .integer()).changed() {
                            self.synthesizer.lock().unwrap().set_envelope_param(op_idx, "rate3", rate3);
                        }
                        
                        if ui.add(egui::Slider::new(&mut rate4, 0.0..=99.0)
                            .text("R4")
                            .integer()).changed() {
                            self.synthesizer.lock().unwrap().set_envelope_param(op_idx, "rate4", rate4);
                        }
                    });
                    
                    columns[1].vertical(|ui| {
                        ui.label("LEVELS");
                        
                        if ui.add(egui::Slider::new(&mut level1, 0.0..=99.0)
                            .text("L1")
                            .integer()).changed() {
                            self.synthesizer.lock().unwrap().set_envelope_param(op_idx, "level1", level1);
                        }
                        
                        if ui.add(egui::Slider::new(&mut level2, 0.0..=99.0)
                            .text("L2")
                            .integer()).changed() {
                            self.synthesizer.lock().unwrap().set_envelope_param(op_idx, "level2", level2);
                        }
                        
                        if ui.add(egui::Slider::new(&mut level3, 0.0..=99.0)
                            .text("L3")
                            .integer()).changed() {
                            self.synthesizer.lock().unwrap().set_envelope_param(op_idx, "level3", level3);
                        }
                        
                        if ui.add(egui::Slider::new(&mut level4, 0.0..=99.0)
                            .text("L4")
                            .integer()).changed() {
                            self.synthesizer.lock().unwrap().set_envelope_param(op_idx, "level4", level4);
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
                    .selected_text(format!("{:02} - {}", 
                        synth.algorithm,
                        algorithms.iter()
                            .find(|a| a.number == synth.algorithm)
                            .map(|a| a.name.as_str())
                            .unwrap_or("Unknown")))
                    .show_ui(ui, |ui| {
                        for alg in &algorithms {
                            if ui.selectable_value(&mut synth.algorithm, alg.number, 
                                format!("{:02} - {}", alg.number, alg.name)).clicked() {
                                synth.set_algorithm(alg.number);
                            }
                        }
                    });
                
                ui.add_space(10.0);
                ui.label("Master Volume:");
                ui.add(egui::Slider::new(&mut synth.master_volume, 0.0..=1.0)
                    .show_value(true));
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
                    if ui.add(egui::Slider::new(&mut master_tune, -150.0..=150.0)
                        .text("cents")
                        .show_value(true)).changed() {
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
                    if ui.add(egui::Slider::new(&mut pb_range, 0.0..=12.0)
                        .text("semitones")
                        .integer()
                        .show_value(true)).changed() {
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
                            if ui.add(egui::Slider::new(&mut porta_time, 0.0..=99.0)
                                .integer()
                                .show_value(true)).changed() {
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
            (Key::Z, 0),  // C
            (Key::S, 1),  // C#
            (Key::X, 2),  // D
            (Key::D, 3),  // D#
            (Key::C, 4),  // E
            (Key::V, 5),  // F
            (Key::G, 6),  // F#
            (Key::B, 7),  // G
            (Key::H, 8),  // G#
            (Key::N, 9),  // A
            (Key::J, 10), // A#
            (Key::M, 11), // B
            
            (Key::Q, 12), // C (octave up)
            (Key::Num2, 13), // C#
            (Key::W, 14), // D
            (Key::Num3, 15), // D#
            (Key::E, 16), // E
            (Key::R, 17), // F
            (Key::Num5, 18), // F#
            (Key::T, 19), // G
            (Key::Num6, 20), // G#
            (Key::Y, 21), // A
            (Key::Num7, 22), // A#
            (Key::U, 23), // B
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
                    // Only show algorithm controls
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            self.draw_algorithm_selector(ui);
                        });
                    });
                }
                DisplayMode::Operator => {
                    // Show operator controls and envelope
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            self.draw_operator_panel(ui);
                            ui.add_space(10.0);
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
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self._midi_handler.is_some() {
                        ui.colored_label(egui::Color32::from_rgb(50, 150, 50), "MIDI OK");
                    } else {
                        ui.colored_label(egui::Color32::from_rgb(150, 50, 50), "NO MIDI");
                    }
                });
            });
        });
        
        ctx.request_repaint();
    }
}