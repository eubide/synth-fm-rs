use crate::algorithms;
use crate::audio_engine::AudioEngine;
use crate::command_queue::{EnvelopeParam, LfoParam, OperatorParam};
use crate::fm_synth::{SynthController, SynthEngine};
use crate::midi_handler::MidiHandler;
use crate::presets::{get_dx7_presets, Dx7Preset};
use crate::state_snapshot::SynthSnapshot;
use eframe::egui;
use std::sync::{Arc, Mutex};

pub struct Dx7App {
    engine: Arc<Mutex<SynthEngine>>,
    controller: Arc<Mutex<SynthController>>,
    _audio_engine: AudioEngine,
    _midi_handler: Option<MidiHandler>,
    selected_operator: usize,
    display_mode: DisplayMode,
    display_text: String,
    last_key_times: std::collections::HashMap<egui::Key, std::time::Instant>,
    current_octave: i32,
    presets: Vec<Dx7Preset>,
    selected_preset: usize,
    /// Cached snapshot from audio thread (updated each frame)
    snapshot: SynthSnapshot,
}

#[derive(PartialEq)]
enum DisplayMode {
    Voice,
    Operator,
    LFO,
    Effects,
}


impl Dx7App {
    pub fn new(
        engine: Arc<Mutex<SynthEngine>>,
        controller: Arc<Mutex<SynthController>>,
        audio_engine: AudioEngine,
        midi_handler: Option<MidiHandler>,
    ) -> Self {
        let presets = get_dx7_presets();
        // Get initial snapshot from controller
        let snapshot = controller.lock().map(|c| c.snapshot()).unwrap_or_default();

        Self {
            engine,
            controller,
            _audio_engine: audio_engine,
            _midi_handler: midi_handler,
            selected_operator: 0,
            display_mode: DisplayMode::Voice,
            display_text: "YAMAHA DX7".to_string(),
            last_key_times: std::collections::HashMap::new(),
            current_octave: 4,
            presets,
            selected_preset: 0,
            snapshot,
        }
    }

    /// Update the cached snapshot from the audio thread (call once per frame)
    fn update_snapshot(&mut self) {
        if let Ok(ctrl) = self.controller.lock() {
            self.snapshot = ctrl.snapshot();
        }
    }

    fn lock_engine(
        &self,
    ) -> Result<
        std::sync::MutexGuard<'_, SynthEngine>,
        std::sync::PoisonError<std::sync::MutexGuard<'_, SynthEngine>>,
    > {
        self.engine.lock()
    }

    fn lock_controller(
        &self,
    ) -> Result<
        std::sync::MutexGuard<'_, SynthController>,
        std::sync::PoisonError<std::sync::MutexGuard<'_, SynthController>>,
    > {
        self.controller.lock()
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

                // Mode-specific sub text (using cached snapshot)
                let sub_text = match self.display_mode {
                    DisplayMode::Voice => {
                        format!(
                            "VOICE: {} | ALG: {:02}",
                            self.snapshot.preset_name,
                            self.snapshot.algorithm
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
                        let chorus = if self.snapshot.chorus_enabled { "CHO" } else { "-" };
                        let delay = if self.snapshot.delay_enabled { "DLY" } else { "-" };
                        let reverb = if self.snapshot.reverb_enabled { "REV" } else { "-" };
                        format!("EFFECTS: {} {} {}", chorus, delay, reverb)
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
                let mode_text = if self.snapshot.mono_mode { "MONO" } else { "POLY" };
                let midi_text = if self._midi_handler.is_some() { "MIDI OK" } else { "NO MIDI" };

                let status_line = if self.snapshot.mono_mode {
                    // Show portamento only in MONO mode
                    let porta_text = if self.snapshot.portamento_enable { "ON" } else { "OFF" };
                    format!(
                        "VOICE: {} | ALG: {:02} | MODE: {} | PORTA: {} | {}",
                        self.snapshot.preset_name,
                        self.snapshot.algorithm,
                        mode_text,
                        porta_text,
                        midi_text
                    )
                } else {
                    // In POLY mode, don't show portamento
                    format!(
                        "VOICE: {} | ALG: {:02} | MODE: {} | {}",
                        self.snapshot.preset_name,
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
                                let mut volume = self.snapshot.master_volume;
                                let slider_response = ui.add(egui::Slider::new(&mut volume, 0.0..=1.0).show_value(false));
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
                                if ui.add(egui::Slider::new(&mut volume, 0.0..=1.0).show_value(false)).changed() {
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
                                if ui.add(egui::Slider::new(&mut master_tune, -150.0..=150.0).show_value(false)).changed() {
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
                                if ui.add(egui::Slider::new(&mut pb_range, 0.0..=12.0).show_value(false)).changed() {
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
                            ui.set_min_width(150.0);
                            let mono_mode = self.snapshot.mono_mode;
                            let porta_enable = self.snapshot.portamento_enable;
                            let porta_time = self.snapshot.portamento_time;

                            ui.horizontal(|ui| {
                                ui.label("MODE:");
                                let mut mode = mono_mode;
                                if ui.selectable_value(&mut mode, false, "POLY").clicked() && mono_mode {
                                    if let Ok(mut ctrl) = self.lock_controller() {
                                        ctrl.set_mono_mode(false);
                                    }
                                }
                                if ui.selectable_value(&mut mode, true, "MONO").clicked() && !mono_mode {
                                    if let Ok(mut ctrl) = self.lock_controller() {
                                        ctrl.set_mono_mode(true);
                                    }
                                }
                            });

                            // Portamento (only visible in MONO mode)
                            if mono_mode {
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
                                        if ui.add(egui::Slider::new(&mut pt, 0.0..=99.0).show_value(false)).changed() {
                                            if let Ok(mut ctrl) = self.lock_controller() {
                                                ctrl.set_portamento_time(pt);
                                            }
                                        }
                                        ui.label(format!("{:.0}", porta_time));
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
        let mono_mode = self.snapshot.mono_mode;
        ui.horizontal(|ui| {
            ui.label("MODE:");
            let mut mode = mono_mode;
            if ui.selectable_value(&mut mode, false, "POLY").clicked() && mono_mode {
                if let Ok(mut ctrl) = self.lock_controller() {
                    ctrl.set_mono_mode(false);
                }
            }
            if ui.selectable_value(&mut mode, true, "MONO").clicked() && !mono_mode {
                if let Ok(mut ctrl) = self.lock_controller() {
                    ctrl.set_mono_mode(true);
                }
            }
        });

        // Portamento (only visible in MONO mode)
        if mono_mode {
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
                    if ui.add(egui::Slider::new(&mut pt, 0.0..=99.0).show_value(false)).changed() {
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
            if ui.add(egui::Slider::new(&mut tune, -150.0..=150.0).show_value(false)).changed() {
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
            if ui.add(egui::Slider::new(&mut pb, 0.0..=12.0).show_value(false)).changed() {
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
                if let Ok(mut ctrl) = self.lock_controller() {
                    ctrl.voice_initialize();
                }
            }
        });
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
                                if let Ok(mut synth) = self.lock_engine() {
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
                if let Ok(mut ctrl) = self.lock_controller() {
                    ctrl.note_on(note, 100);
                }
                self.last_key_times.insert(*key, now);
            } else if ctx.input(|i| i.key_released(*key)) {
                if let Some(&_press_time) = self.last_key_times.get(key) {
                    let note = (self.current_octave * 12 + 12 + semitone) as u8;
                    if let Ok(mut ctrl) = self.lock_controller() {
                        ctrl.note_off(note);
                    }
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
            if let Ok(mut ctrl) = self.lock_controller() {
                ctrl.panic();
            }
        }
    }
}

impl eframe::App for Dx7App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update snapshot from audio thread once per frame
        self.update_snapshot();

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
                DisplayMode::Operator => {
                    // Two-column layout: Left = Algorithm + Op selector, Right = Selected Op details
                    ui.columns(2, |columns| {
                        // LEFT COLUMN: Algorithm diagram + Operator selector strip
                        columns[0].vertical(|ui| {
                            self.draw_algorithm_diagram_compact(ui);
                            ui.add_space(4.0);
                            self.draw_operator_selector_strip(ui);
                        });

                        // RIGHT COLUMN: Selected operator full details with envelope
                        columns[1].vertical(|ui| {
                            self.draw_operator_full_panel(ui);
                        });
                    });
                }
                DisplayMode::LFO => {
                    self.draw_lfo_panel(ui);
                }
                DisplayMode::Effects => {
                    self.draw_effects_panel(ui);
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
    fn draw_lfo_panel(&mut self, ui: &mut egui::Ui) {
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
                            if ui.add(egui::Slider::new(&mut lfo_rate, 0.0..=99.0).integer()).changed() {
                                if let Ok(mut ctrl) = self.lock_controller() {
                                    ctrl.set_lfo_param(LfoParam::Rate, lfo_rate);
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Delay:");
                            if ui.add(egui::Slider::new(&mut lfo_delay, 0.0..=99.0).integer()).changed() {
                                if let Ok(mut ctrl) = self.lock_controller() {
                                    ctrl.set_lfo_param(LfoParam::Delay, lfo_delay);
                                }
                            }
                        });
                        ui.label(format!(
                            "Freq: {:.2} Hz | Delay: {:.2}s",
                            self.snapshot.lfo_frequency_hz,
                            self.snapshot.lfo_delay_seconds
                        ));
                    });

                    // Right column: Modulation
                    columns[1].vertical(|ui| {
                        ui.label("MODULATION");
                        ui.horizontal(|ui| {
                            ui.label("Pitch:");
                            if ui.add(egui::Slider::new(&mut lfo_pitch_depth, 0.0..=99.0).integer()).changed() {
                                if let Ok(mut ctrl) = self.lock_controller() {
                                    ctrl.set_lfo_param(LfoParam::PitchDepth, lfo_pitch_depth);
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Amp:");
                            if ui.add(egui::Slider::new(&mut lfo_amp_depth, 0.0..=99.0).integer()).changed() {
                                if let Ok(mut ctrl) = self.lock_controller() {
                                    ctrl.set_lfo_param(LfoParam::AmpDepth, lfo_amp_depth);
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Wave:");
                            egui::ComboBox::from_id_source("lfo_waveform")
                                .selected_text(lfo_waveform.name())
                                .show_ui(ui, |ui| {
                                    for (i, &waveform) in crate::lfo::LFOWaveform::all().iter().enumerate() {
                                        if ui.selectable_value(&mut lfo_waveform.clone(), waveform, waveform.name()).clicked() {
                                            if let Ok(mut ctrl) = self.lock_controller() {
                                                ctrl.set_lfo_param(LfoParam::Waveform(i as u8), 0.0);
                                            }
                                        }
                                    }
                                });
                        });
                        ui.horizontal(|ui| {
                            ui.label("Key Sync:");
                            if ui.checkbox(&mut lfo_key_sync, "").changed() {
                                if let Ok(mut ctrl) = self.lock_controller() {
                                    ctrl.set_lfo_param(LfoParam::KeySync, if lfo_key_sync { 1.0 } else { 0.0 });
                                }
                            }
                        });
                    });
                });

                ui.separator();
                let mod_pct = (self.snapshot.mod_wheel * 100.0) as i32;
                ui.label(format!(
                    "Mod Wheel: {}%{}",
                    mod_pct,
                    if mod_pct == 0 { " (move to enable)" } else { "" }
                ));
            });
        });
    }

    fn draw_effects_panel(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.label("EFFECTS");
                ui.separator();

                ui.columns(3, |columns| {
                    self.draw_chorus_effect(&mut columns[0]);
                    self.draw_delay_effect(&mut columns[1]);
                    self.draw_reverb_effect(&mut columns[2]);
                });

                ui.separator();
                ui.label("Signal: Input -> Chorus -> Delay -> Reverb -> Output");
            });
        });
    }

    fn draw_chorus_effect(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("CHORUS").strong());

                if let Ok(mut synth) = self.lock_engine() {
                    let chorus = &mut synth.effects.chorus;

                    ui.horizontal(|ui| {
                        ui.label("Enable:");
                        ui.checkbox(&mut chorus.enabled, "");
                    });

                    ui.add_enabled_ui(chorus.enabled, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Rate:");
                            ui.add(
                                egui::Slider::new(&mut chorus.rate, 0.1..=5.0)
                                    .suffix(" Hz")
                                    .show_value(true),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label("Depth:");
                            ui.add(
                                egui::Slider::new(&mut chorus.depth, 0.0..=10.0)
                                    .suffix(" ms")
                                    .show_value(true),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label("Mix:");
                            ui.add(
                                egui::Slider::new(&mut chorus.mix, 0.0..=1.0).show_value(true),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label("Feedback:");
                            ui.add(
                                egui::Slider::new(&mut chorus.feedback, 0.0..=0.7)
                                    .show_value(true),
                            );
                        });
                    });
                }
            });
        });
    }

    fn draw_delay_effect(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("DELAY").strong());

                if let Ok(mut synth) = self.lock_engine() {
                    let delay = &mut synth.effects.delay;

                    ui.horizontal(|ui| {
                        ui.label("Enable:");
                        ui.checkbox(&mut delay.enabled, "");
                    });

                    ui.add_enabled_ui(delay.enabled, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Time:");
                            ui.add(
                                egui::Slider::new(&mut delay.time_ms, 0.0..=1000.0)
                                    .suffix(" ms")
                                    .show_value(true),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label("Feedback:");
                            ui.add(
                                egui::Slider::new(&mut delay.feedback, 0.0..=0.9)
                                    .show_value(true),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label("Mix:");
                            ui.add(
                                egui::Slider::new(&mut delay.mix, 0.0..=1.0).show_value(true),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label("Ping-Pong:");
                            ui.checkbox(&mut delay.ping_pong, "");
                        });
                    });
                }
            });
        });
    }

    fn draw_reverb_effect(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("REVERB").strong());

                if let Ok(mut synth) = self.lock_engine() {
                    let reverb = &mut synth.effects.reverb;

                    ui.horizontal(|ui| {
                        ui.label("Enable:");
                        ui.checkbox(&mut reverb.enabled, "");
                    });

                    ui.add_enabled_ui(reverb.enabled, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Room Size:");
                            ui.add(
                                egui::Slider::new(&mut reverb.room_size, 0.0..=1.0)
                                    .show_value(true),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label("Damping:");
                            ui.add(
                                egui::Slider::new(&mut reverb.damping, 0.0..=1.0)
                                    .show_value(true),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label("Mix:");
                            ui.add(
                                egui::Slider::new(&mut reverb.mix, 0.0..=1.0).show_value(true),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label("Width:");
                            ui.add(
                                egui::Slider::new(&mut reverb.width, 0.0..=1.0).show_value(true),
                            );
                        });
                    });
                }
            });
        });
    }

    fn draw_algorithm_diagram_compact(&mut self, ui: &mut egui::Ui) {
        let current_alg = self.snapshot.algorithm;
        let alg_info = algorithms::get_algorithm_info(current_alg);
        let enabled_states = [
            self.snapshot.operators[0].enabled,
            self.snapshot.operators[1].enabled,
            self.snapshot.operators[2].enabled,
            self.snapshot.operators[3].enabled,
            self.snapshot.operators[4].enabled,
            self.snapshot.operators[5].enabled,
        ];

        ui.group(|ui| {
            ui.vertical(|ui| {
                // Compact header with algorithm selector
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("ALG").strong());
                    if ui.small_button("<").clicked() && current_alg > 1 {
                        if let Ok(mut ctrl) = self.lock_controller() {
                            ctrl.set_algorithm(current_alg - 1);
                        }
                    }
                    ui.label(egui::RichText::new(format!("{:02}", current_alg)).strong());
                    if ui.small_button(">").clicked() && current_alg < 32 {
                        if let Ok(mut ctrl) = self.lock_controller() {
                            ctrl.set_algorithm(current_alg + 1);
                        }
                    }
                    ui.label(algorithms::get_algorithm_name(current_alg));
                });

                // Compact diagram canvas
                let (response, painter) =
                    ui.allocate_painter(egui::vec2(ui.available_width(), 75.0), egui::Sense::hover());
                let rect = response.rect;

                let positions = self.calculate_operator_positions_compact(&alg_info, rect);

                // Draw connections
                let connection_color = egui::Color32::from_rgb(100, 100, 100);
                for (from, to) in &alg_info.connections {
                    let from_pos = positions[(*from - 1) as usize];
                    let to_pos = positions[(*to - 1) as usize];
                    painter.line_segment([from_pos, to_pos], egui::Stroke::new(1.5, connection_color));
                }

                // Draw feedback indicator
                if alg_info.feedback_op > 0 {
                    let fb_pos = positions[(alg_info.feedback_op - 1) as usize];
                    let loop_center = fb_pos + egui::vec2(14.0, -8.0);
                    painter.circle_stroke(loop_center, 6.0, egui::Stroke::new(1.5, egui::Color32::from_rgb(200, 100, 50)));
                }

                // Draw operators (smaller)
                let op_radius = 11.0;
                for (i, &pos) in positions.iter().enumerate() {
                    let op_num = (i + 1) as u8;
                    let is_carrier = alg_info.carriers.contains(&op_num);
                    let is_selected = self.selected_operator == i;
                    let is_enabled = enabled_states[i];

                    let (fill_color, stroke_color, text_color) = if !is_enabled {
                        (egui::Color32::from_rgb(80, 80, 80), egui::Color32::from_rgb(60, 60, 60), egui::Color32::from_rgb(120, 120, 120))
                    } else if is_carrier {
                        (egui::Color32::from_rgb(70, 130, 180),
                         if is_selected { egui::Color32::from_rgb(255, 200, 0) } else { egui::Color32::from_rgb(50, 100, 150) },
                         egui::Color32::WHITE)
                    } else {
                        (egui::Color32::from_rgb(100, 160, 100),
                         if is_selected { egui::Color32::from_rgb(255, 200, 0) } else { egui::Color32::from_rgb(70, 130, 70) },
                         egui::Color32::WHITE)
                    };

                    painter.circle(pos, op_radius, fill_color, egui::Stroke::new(if is_selected { 2.5 } else { 1.5 }, stroke_color));
                    painter.text(pos, egui::Align2::CENTER_CENTER, format!("{}", op_num), egui::FontId::proportional(10.0), text_color);
                }

                // Output indicator
                let output_x = rect.right() - 20.0;
                let output_y = rect.center().y + 20.0;
                painter.text(egui::pos2(output_x, output_y), egui::Align2::CENTER_CENTER, "OUT", egui::FontId::proportional(8.0), egui::Color32::from_rgb(100, 100, 100));

                for &carrier in &alg_info.carriers {
                    let carrier_pos = positions[(carrier - 1) as usize];
                    painter.line_segment([carrier_pos + egui::vec2(op_radius + 2.0, 0.0), egui::pos2(output_x - 10.0, output_y)],
                        egui::Stroke::new(1.0, egui::Color32::from_rgb(70, 130, 180)));
                }
            });
        });
    }

    fn calculate_operator_positions_compact(&self, alg_info: &algorithms::AlgorithmInfo, rect: egui::Rect) -> [egui::Pos2; 6] {
        let mut layers: [i32; 6] = [0; 6];
        for &carrier in &alg_info.carriers {
            layers[(carrier - 1) as usize] = 0;
        }
        for _ in 0..5 {
            for (from, to) in &alg_info.connections {
                let to_layer = layers[(*to - 1) as usize];
                let from_layer = &mut layers[(*from - 1) as usize];
                *from_layer = (*from_layer).max(to_layer + 1);
            }
        }

        let max_layer = *layers.iter().max().unwrap_or(&0);
        let mut ops_per_layer: Vec<Vec<u8>> = vec![vec![]; (max_layer + 1) as usize];
        for (i, &layer) in layers.iter().enumerate() {
            ops_per_layer[layer as usize].push((i + 1) as u8);
        }

        let layer_height = rect.height() / (max_layer + 2) as f32;
        let mut positions: [egui::Pos2; 6] = [egui::Pos2::ZERO; 6];

        for (layer, ops) in ops_per_layer.iter().enumerate() {
            let y = rect.bottom() - layer_height * (layer as f32 + 1.0);
            let layer_width = rect.width() - 50.0;
            let spacing = layer_width / (ops.len() + 1) as f32;
            for (i, &op) in ops.iter().enumerate() {
                let x = rect.left() + spacing * (i as f32 + 1.0);
                positions[(op - 1) as usize] = egui::pos2(x, y);
            }
        }
        positions
    }

    /// Minimal operator selector strip - just clickable buttons to select operator
    fn draw_operator_selector_strip(&mut self, ui: &mut egui::Ui) {
        let current_alg = self.snapshot.algorithm;
        let alg_info = algorithms::get_algorithm_info(current_alg);

        ui.group(|ui| {
            ui.label(egui::RichText::new("SELECT OPERATOR").size(10.0));
            ui.horizontal_wrapped(|ui| {
                for op_idx in 0..6 {
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

                    // Vertical mini-panel per operator
                    ui.allocate_ui(egui::vec2(65.0, 70.0), |ui| {
                        let frame = egui::Frame::none()
                            .fill(if is_selected { egui::Color32::from_rgb(240, 248, 255) } else { egui::Color32::from_rgb(250, 250, 250) })
                            .stroke(egui::Stroke::new(if is_selected { 2.5 } else { 1.0 },
                                if is_selected { egui::Color32::from_rgb(255, 180, 0) } else { base_color }))
                            .rounding(4.0)
                            .inner_margin(4.0);

                        frame.show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                // OP label with role
                                let role = if is_carrier { "C" } else { "M" };
                                let fb = if has_feedback { " F" } else { "" };
                                let label_text = format!("OP{} {}{}", op_num, role, fb);

                                if ui.selectable_label(is_selected, egui::RichText::new(label_text).size(11.0).color(base_color)).clicked() {
                                    self.selected_operator = op_idx;
                                }

                                // Level bar (vertical)
                                let bar_width = 40.0;
                                let bar_height = 10.0;
                                let (bar_rect, _) = ui.allocate_exact_size(egui::vec2(bar_width, bar_height), egui::Sense::hover());
                                ui.painter().rect_filled(bar_rect, 2.0, egui::Color32::from_rgb(40, 40, 40));
                                let fill_width = (level / 99.0) * bar_width;
                                let fill_rect = egui::Rect::from_min_size(bar_rect.min, egui::vec2(fill_width, bar_height));
                                ui.painter().rect_filled(fill_rect, 2.0, if enabled { base_color } else { egui::Color32::from_rgb(60, 60, 60) });

                                // Level value
                                ui.label(egui::RichText::new(format!("{:.0}", level)).size(10.0));
                            });
                        });
                    });
                }
            });
        });
    }

    /// Full operator panel with all parameters and envelope
    fn draw_operator_full_panel(&mut self, ui: &mut egui::Ui) {
        let op_idx = self.selected_operator;
        let current_alg = self.snapshot.algorithm;
        let alg_info = algorithms::get_algorithm_info(current_alg);
        let op_num = (op_idx + 1) as u8;
        let is_carrier = alg_info.carriers.contains(&op_num);
        let has_feedback = alg_info.feedback_op == op_num;

        // Read all operator parameters from snapshot (lock-free)
        let op_snap = &self.snapshot.operators[op_idx];
        let mut enabled = op_snap.enabled;
        let mut freq_ratio = op_snap.frequency_ratio;
        let mut output_level = op_snap.output_level;
        let mut detune = op_snap.detune;
        let mut feedback = op_snap.feedback;
        let mut vel_sens = op_snap.velocity_sensitivity;
        let mut key_scale_lvl = op_snap.key_scale_level;
        let mut key_scale_rt = op_snap.key_scale_rate;
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
                ui.label(egui::RichText::new(format!("OPERATOR {} - {}{}", op_num, role, fb_text)).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.checkbox(&mut enabled, "ON").changed() {
                        if let Ok(mut ctrl) = self.lock_controller() {
                            ctrl.set_operator_param(op_idx as u8, OperatorParam::Enabled, if enabled { 1.0 } else { 0.0 });
                        }
                    }
                });
            });
            ui.separator();

            ui.add_enabled_ui(enabled, |ui| {
                // Parameters section
                ui.label(egui::RichText::new("PARAMETERS").size(10.0));
                egui::Grid::new("op_params_grid").num_columns(4).spacing([8.0, 4.0]).show(ui, |ui| {
                    ui.label("Ratio:");
                    if ui.add(egui::Slider::new(&mut freq_ratio, 0.5..=31.0).step_by(1.0)
                        .custom_formatter(|n, _| format!("{:.2}", crate::dx7_frequency::quantize_frequency_ratio(n as f32)))).changed() {
                        let q = crate::dx7_frequency::quantize_frequency_ratio(freq_ratio);
                        if let Ok(mut ctrl) = self.lock_controller() {
                            ctrl.set_operator_param(op_idx as u8, OperatorParam::Ratio, q);
                        }
                    }
                    ui.label("Level:");
                    if ui.add(egui::Slider::new(&mut output_level, 0.0..=99.0).integer()).changed() {
                        if let Ok(mut ctrl) = self.lock_controller() {
                            ctrl.set_operator_param(op_idx as u8, OperatorParam::Level, output_level);
                        }
                    }
                    ui.end_row();

                    ui.label("Detune:");
                    if ui.add(egui::Slider::new(&mut detune, -7.0..=7.0).integer()).changed() {
                        if let Ok(mut ctrl) = self.lock_controller() {
                            ctrl.set_operator_param(op_idx as u8, OperatorParam::Detune, detune);
                        }
                    }
                    ui.label("Vel Sens:");
                    if ui.add(egui::Slider::new(&mut vel_sens, 0.0..=7.0).integer()).changed() {
                        if let Ok(mut ctrl) = self.lock_controller() {
                            ctrl.set_operator_param(op_idx as u8, OperatorParam::VelocitySensitivity, vel_sens);
                        }
                    }
                    ui.end_row();

                    if has_feedback {
                        ui.label("Feedback:");
                        if ui.add(egui::Slider::new(&mut feedback, 0.0..=7.0).integer()).changed() {
                            if let Ok(mut ctrl) = self.lock_controller() {
                                ctrl.set_operator_param(op_idx as u8, OperatorParam::Feedback, feedback);
                            }
                        }
                    } else {
                        ui.label("");
                        ui.label("");
                    }
                    ui.label("Key Lvl:");
                    if ui.add(egui::Slider::new(&mut key_scale_lvl, 0.0..=99.0).integer()).changed() {
                        if let Ok(mut ctrl) = self.lock_controller() {
                            ctrl.set_operator_param(op_idx as u8, OperatorParam::KeyScaleLevel, key_scale_lvl);
                        }
                    }
                    ui.end_row();

                    ui.label("");
                    ui.label("");
                    ui.label("Key Rate:");
                    if ui.add(egui::Slider::new(&mut key_scale_rt, 0.0..=7.0).integer()).changed() {
                        if let Ok(mut ctrl) = self.lock_controller() {
                            ctrl.set_operator_param(op_idx as u8, OperatorParam::KeyScaleRate, key_scale_rt);
                        }
                    }
                    ui.end_row();
                });

                ui.add_space(8.0);

                // Envelope section
                ui.label(egui::RichText::new("ENVELOPE").size(10.0));
                egui::Grid::new("op_env_grid").num_columns(4).spacing([8.0, 4.0]).show(ui, |ui| {
                    // Row 1: Rates
                    ui.label("R1:");
                    if ui.add(egui::Slider::new(&mut rate1, 0.0..=99.0).integer()).changed() {
                        if let Ok(mut ctrl) = self.lock_controller() {
                            ctrl.set_envelope_param(op_idx as u8, EnvelopeParam::Rate1, rate1);
                        }
                    }
                    ui.label("R2:");
                    if ui.add(egui::Slider::new(&mut rate2, 0.0..=99.0).integer()).changed() {
                        if let Ok(mut ctrl) = self.lock_controller() {
                            ctrl.set_envelope_param(op_idx as u8, EnvelopeParam::Rate2, rate2);
                        }
                    }
                    ui.end_row();

                    ui.label("L1:");
                    if ui.add(egui::Slider::new(&mut level1, 0.0..=99.0).integer()).changed() {
                        if let Ok(mut ctrl) = self.lock_controller() {
                            ctrl.set_envelope_param(op_idx as u8, EnvelopeParam::Level1, level1);
                        }
                    }
                    ui.label("L2:");
                    if ui.add(egui::Slider::new(&mut level2, 0.0..=99.0).integer()).changed() {
                        if let Ok(mut ctrl) = self.lock_controller() {
                            ctrl.set_envelope_param(op_idx as u8, EnvelopeParam::Level2, level2);
                        }
                    }
                    ui.end_row();

                    ui.label("R3:");
                    if ui.add(egui::Slider::new(&mut rate3, 0.0..=99.0).integer()).changed() {
                        if let Ok(mut ctrl) = self.lock_controller() {
                            ctrl.set_envelope_param(op_idx as u8, EnvelopeParam::Rate3, rate3);
                        }
                    }
                    ui.label("R4:");
                    if ui.add(egui::Slider::new(&mut rate4, 0.0..=99.0).integer()).changed() {
                        if let Ok(mut ctrl) = self.lock_controller() {
                            ctrl.set_envelope_param(op_idx as u8, EnvelopeParam::Rate4, rate4);
                        }
                    }
                    ui.end_row();

                    ui.label("L3:");
                    if ui.add(egui::Slider::new(&mut level3, 0.0..=99.0).integer()).changed() {
                        if let Ok(mut ctrl) = self.lock_controller() {
                            ctrl.set_envelope_param(op_idx as u8, EnvelopeParam::Level3, level3);
                        }
                    }
                    ui.label("L4:");
                    if ui.add(egui::Slider::new(&mut level4, 0.0..=99.0).integer()).changed() {
                        if let Ok(mut ctrl) = self.lock_controller() {
                            ctrl.set_envelope_param(op_idx as u8, EnvelopeParam::Level4, level4);
                        }
                    }
                    ui.end_row();
                });
            });
        });
    }
}
