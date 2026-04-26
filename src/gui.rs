use crate::algorithms;
use crate::audio_engine::AudioEngine;
use crate::command_queue::{
    EffectParam, EffectType, EnvelopeParam, LfoParam, OperatorParam, PitchEgParam,
};
use crate::fm_synth::{SynthController, SynthEngine};
use crate::midi_handler::MidiHandler;
use crate::presets::Dx7Preset;
use crate::state_snapshot::SynthSnapshot;
use eframe::egui;
use std::sync::{Arc, Mutex};

pub struct Dx7App {
    engine: Arc<Mutex<SynthEngine>>,
    controller: Arc<Mutex<SynthController>>,
    /// Owned to keep the audio stream alive. Optional so unit tests can
    /// construct a `Dx7App` without a real audio device.
    _audio_engine: Option<AudioEngine>,
    _midi_handler: Option<MidiHandler>,
    selected_operator: usize,
    display_mode: DisplayMode,
    display_text: String,
    last_key_times: std::collections::HashMap<egui::Key, std::time::Instant>,
    current_octave: i32,
    presets: Vec<Dx7Preset>,
    selected_preset: usize,
    /// Active collection filter; None = show all collections.
    selected_collection: Option<String>,
    preset_search: String,
    /// Cached snapshot from audio thread (updated each frame)
    snapshot: SynthSnapshot,
    /// Path edited in the MIDI panel for SysEx load/save.
    sysex_path: String,
    /// Last status line shown in the MIDI panel (load/save feedback).
    sysex_status: String,
    /// Cached MIDI channel selection: None = OMNI, Some(0..15) = specific channel.
    midi_channel_ui: Option<u8>,
}

#[derive(PartialEq)]
#[allow(clippy::upper_case_acronyms)]
enum DisplayMode {
    Voice,
    Operator,
    LFO,
    Effects,
    Midi,
}

impl Dx7App {
    pub fn new(
        engine: Arc<Mutex<SynthEngine>>,
        controller: Arc<Mutex<SynthController>>,
        audio_engine: AudioEngine,
        midi_handler: Option<MidiHandler>,
        presets: Vec<Dx7Preset>,
    ) -> Self {
        Self::build(engine, controller, Some(audio_engine), midi_handler, presets)
    }

    /// Test-only constructor: builds a `Dx7App` without a real audio engine.
    #[cfg(test)]
    pub fn new_for_test(
        engine: Arc<Mutex<SynthEngine>>,
        controller: Arc<Mutex<SynthController>>,
        presets: Vec<Dx7Preset>,
    ) -> Self {
        Self::build(engine, controller, None, None, presets)
    }

    fn build(
        engine: Arc<Mutex<SynthEngine>>,
        controller: Arc<Mutex<SynthController>>,
        audio_engine: Option<AudioEngine>,
        midi_handler: Option<MidiHandler>,
        presets: Vec<Dx7Preset>,
    ) -> Self {
        let snapshot = controller.lock().map(|c| c.snapshot()).unwrap_or_default();
        Self {
            engine,
            controller,
            _audio_engine: audio_engine,
            _midi_handler: midi_handler,
            selected_operator: 0,
            display_mode: DisplayMode::Voice,
            display_text: "DX7 FM SYNTH".to_string(),
            last_key_times: std::collections::HashMap::new(),
            current_octave: 4,
            presets,
            selected_preset: 0,
            selected_collection: None,
            preset_search: String::new(),
            snapshot,
            sysex_path: String::from("voice.syx"),
            sysex_status: String::new(),
            midi_channel_ui: None,
        }
    }

    /// Update the cached snapshot from the audio thread (call once per frame)
    fn update_snapshot(&mut self) {
        if let Ok(ctrl) = self.controller.lock() {
            self.snapshot = ctrl.snapshot();
        }
    }

    /// Frame-independent rendering: drives one full GUI frame against the given
    /// `egui::Context`. Split out from `App::update` so tests can call it
    /// without constructing an `eframe::Frame`.
    pub(crate) fn render(&mut self, ctx: &egui::Context) {
        self.update_snapshot();
        self.handle_keyboard_input(ctx);
        ctx.set_visuals(egui::Visuals::light());

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("DX7-STYLE DIGITAL FM SYNTHESIZER");
            });
            ui.separator();

            self.draw_dx7_display(ui);
            ui.add_space(8.0);
            self.draw_global_controls(ui);
            ui.add_space(8.0);
            self.draw_membrane_buttons(ui);
            ui.add_space(8.0);

            match self.display_mode {
                DisplayMode::Voice => self.draw_preset_selector(ui),
                DisplayMode::Operator => {
                    ui.columns(2, |columns| {
                        columns[0].vertical(|ui| {
                            self.draw_algorithm_diagram_compact(ui);
                            ui.add_space(4.0);
                            self.draw_operator_selector_strip(ui);
                        });
                        columns[1].vertical(|ui| {
                            self.draw_operator_full_panel(ui);
                        });
                    });
                }
                DisplayMode::LFO => self.draw_lfo_panel(ui),
                DisplayMode::Effects => self.draw_effects_panel(ui),
                DisplayMode::Midi => self.draw_midi_panel(ui),
            }

            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Keyboard: Z-M (lower octave), Q-U (upper octave)");
                ui.label(format!("| Octave: {}", self.current_octave));
                ui.label("| Space: Panic");
                ui.label("| Up/Down: Change octave");
            });
        });

        if ctx.input(|i| !i.events.is_empty()) {
            ctx.request_repaint_after(std::time::Duration::from_millis(16)); // ~60 FPS
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
                let midi_text = if self._midi_handler.is_some() {
                    "MIDI OK"
                } else {
                    "NO MIDI"
                };

                let is_mono = self.snapshot.voice_mode != crate::state_snapshot::VoiceMode::Poly;
                let status_line = if is_mono {
                    // Show portamento only in MONO modes
                    let porta_text = if self.snapshot.portamento_enable {
                        "ON"
                    } else {
                        "OFF"
                    };
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
                        self.snapshot.preset_name, self.snapshot.algorithm, mode_text, midi_text
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
                                let slider_response = ui.add(
                                    egui::Slider::new(&mut volume, 0.0..=1.0).show_value(false),
                                );
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
                                    .add(
                                        egui::Slider::new(&mut volume, 0.0..=1.0).show_value(false),
                                    )
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
                                        egui::Slider::new(&mut master_tune, -150.0..=150.0)
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
                                    .add(
                                        egui::Slider::new(&mut pb_range, 0.0..=12.0)
                                            .show_value(false),
                                    )
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
                                            .add(
                                                egui::Slider::new(&mut pt, 0.0..=99.0)
                                                    .show_value(false),
                                            )
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
                        .add(egui::Slider::new(&mut pt, 0.0..=99.0).show_value(false))
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
                .add(egui::Slider::new(&mut tune, -150.0..=150.0).show_value(false))
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
                .add(egui::Slider::new(&mut pb, 0.0..=12.0).show_value(false))
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
                }
            });
        });
    }

    fn draw_preset_selector(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(4.0, 3.0);

            // --- Current voice header ---
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("current:").size(11.0).strong());
                if let Some(p) = self.presets.get(self.selected_preset) {
                    ui.colored_label(egui::Color32::from_rgb(100, 220, 100), p.name.as_str());
                    ui.label(
                        egui::RichText::new(format!("[{}]", p.collection))
                            .size(10.0)
                            .color(egui::Color32::from_gray(140)),
                    );
                } else {
                    ui.colored_label(egui::Color32::GRAY, "(none)");
                }
            });
            ui.separator();

            // --- Search + collection filter ---
            ui.horizontal(|ui| {
                ui.label("search:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.preset_search)
                        .hint_text("filter by name…")
                        .desired_width(140.0),
                );
                if ui.small_button("×").on_hover_text("Clear").clicked() {
                    self.preset_search.clear();
                }
            });

            let collections: Vec<String> = {
                let mut seen = std::collections::HashSet::new();
                self.presets
                    .iter()
                    .map(|p| p.collection.clone())
                    .filter(|c| seen.insert(c.clone()))
                    .collect()
            };

            if collections.len() > 1 {
                ui.horizontal(|ui| {
                    ui.label("collection:");
                    if ui
                        .selectable_label(self.selected_collection.is_none(), "all")
                        .clicked()
                    {
                        self.selected_collection = None;
                    }
                    for coll in &collections {
                        let active = self.selected_collection.as_deref() == Some(coll.as_str());
                        if ui.selectable_label(active, coll.as_str()).clicked() {
                            self.selected_collection = Some(coll.clone());
                        }
                    }
                });
            }
            ui.separator();

            // --- Scrollable preset list grouped by collection ---
            // Collect indices to avoid holding borrows across mutable self access.
            let search_lower = self.preset_search.to_lowercase();
            let filter_coll = self.selected_collection.clone();
            let filtered_indices: Vec<usize> = self
                .presets
                .iter()
                .enumerate()
                .filter(|(_, p)| {
                    let coll_ok = filter_coll.as_deref().is_none_or(|c| p.collection == c);
                    let name_ok =
                        search_lower.is_empty() || p.name.to_lowercase().contains(&search_lower);
                    coll_ok && name_ok
                })
                .map(|(i, _)| i)
                .collect();

            if filtered_indices.is_empty() {
                ui.colored_label(egui::Color32::GRAY, "no presets match");
                return;
            }

            egui::ScrollArea::vertical()
                .max_height(320.0)
                .show(ui, |ui| {
                    let mut last_coll: Option<String> = None;
                    for &global_idx in &filtered_indices {
                        let coll = self.presets[global_idx].collection.clone();
                        let name = self.presets[global_idx].name.clone();
                        let is_current = global_idx == self.selected_preset;

                        // Section header when collection changes
                        let new_section = last_coll.as_deref() != Some(coll.as_str());
                        if new_section {
                            if last_coll.is_some() {
                                ui.add_space(4.0);
                            }
                            ui.label(
                                egui::RichText::new(coll.to_uppercase())
                                    .size(10.0)
                                    .color(egui::Color32::from_rgb(180, 180, 80))
                                    .strong(),
                            );
                            last_coll = Some(coll);
                        }

                        let button = egui::Button::new(name.as_str())
                            .wrap_mode(egui::TextWrapMode::Truncate);
                        let button = if is_current {
                            button.fill(egui::Color32::from_rgb(60, 110, 60))
                        } else {
                            button
                        };

                        if ui.add_sized([ui.available_width(), 18.0], button).clicked() {
                            let preset = self.presets[global_idx].clone();
                            self.selected_preset = global_idx;
                            if let Ok(mut synth) = self.lock_engine() {
                                preset.apply_to_synth(&mut synth);
                            }
                            self.display_text = format!("LOADED: {}", name);
                        }
                    }
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
        self.render(ctx);
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
                            if ui
                                .add(egui::Slider::new(&mut lfo_rate, 0.0..=99.0).integer())
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
                                .add(egui::Slider::new(&mut lfo_delay, 0.0..=99.0).integer())
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
                                .add(egui::Slider::new(&mut lfo_pitch_depth, 0.0..=99.0).integer())
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
                                .add(egui::Slider::new(&mut lfo_amp_depth, 0.0..=99.0).integer())
                                .changed()
                            {
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
                        if ui
                            .add(egui::Slider::new(&mut pms, 0.0..=7.0).integer())
                            .changed()
                        {
                            if let Ok(mut ctrl) = self.lock_controller() {
                                ctrl.set_pitch_mod_sensitivity(pms as u8);
                            }
                        }
                    });
                    columns[1].horizontal(|ui| {
                        ui.label("EG Bias:");
                        if ui
                            .add(egui::Slider::new(&mut eg_bias, 0.0..=7.0).integer())
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
                            .add(egui::Slider::new(&mut pitch_bias, 0.0..=7.0).integer())
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
        if ui
            .add(egui::Slider::new(&mut v, 0.0..=99.0).integer())
            .changed()
        {
            if let Ok(mut ctrl) = self.lock_controller() {
                ctrl.set_pitch_eg_param(param, v);
            }
        }
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
                                egui::Slider::new(&mut rate, 0.1..=5.0)
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
                                egui::Slider::new(&mut depth, 0.0..=10.0)
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
                            .add(egui::Slider::new(&mut mix, 0.0..=1.0).show_value(true))
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
                            .add(egui::Slider::new(&mut feedback, 0.0..=0.7).show_value(true))
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
                                egui::Slider::new(&mut time_ms, 0.0..=1000.0)
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
                            .add(egui::Slider::new(&mut feedback, 0.0..=0.9).show_value(true))
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
                            .add(egui::Slider::new(&mut mix, 0.0..=1.0).show_value(true))
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
                            .add(egui::Slider::new(&mut room_size, 0.0..=1.0).show_value(true))
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
                            .add(egui::Slider::new(&mut damping, 0.0..=1.0).show_value(true))
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
                            .add(egui::Slider::new(&mut mix, 0.0..=1.0).show_value(true))
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
                            .add(egui::Slider::new(&mut width, 0.0..=1.0).show_value(true))
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
                let (response, painter) = ui
                    .allocate_painter(egui::vec2(ui.available_width(), 75.0), egui::Sense::hover());
                let rect = response.rect;

                let positions = self.calculate_operator_positions_compact(&alg_info, rect);

                // Draw connections
                let connection_color = egui::Color32::from_rgb(100, 100, 100);
                for (from, to) in &alg_info.connections {
                    let from_pos = positions[(*from - 1) as usize];
                    let to_pos = positions[(*to - 1) as usize];
                    painter
                        .line_segment([from_pos, to_pos], egui::Stroke::new(1.5, connection_color));
                }

                // Draw feedback indicator
                if alg_info.feedback_op > 0 {
                    let fb_pos = positions[(alg_info.feedback_op - 1) as usize];
                    let loop_center = fb_pos + egui::vec2(14.0, -8.0);
                    painter.circle_stroke(
                        loop_center,
                        6.0,
                        egui::Stroke::new(1.5, egui::Color32::from_rgb(200, 100, 50)),
                    );
                }

                // Draw operators (smaller)
                let op_radius = 11.0;
                for (i, &pos) in positions.iter().enumerate() {
                    let op_num = (i + 1) as u8;
                    let is_carrier = alg_info.carriers.contains(&op_num);
                    let is_selected = self.selected_operator == i;
                    let is_enabled = enabled_states[i];
                    let activity = self.snapshot.operators[i].live_level.clamp(0.0, 1.0);

                    let (base_fill, stroke_color, text_color) = if !is_enabled {
                        (
                            egui::Color32::from_rgb(80, 80, 80),
                            egui::Color32::from_rgb(60, 60, 60),
                            egui::Color32::from_rgb(120, 120, 120),
                        )
                    } else if is_carrier {
                        (
                            egui::Color32::from_rgb(70, 130, 180),
                            if is_selected {
                                egui::Color32::from_rgb(255, 200, 0)
                            } else {
                                egui::Color32::from_rgb(50, 100, 150)
                            },
                            egui::Color32::WHITE,
                        )
                    } else {
                        (
                            egui::Color32::from_rgb(100, 160, 100),
                            if is_selected {
                                egui::Color32::from_rgb(255, 200, 0)
                            } else {
                                egui::Color32::from_rgb(70, 130, 70)
                            },
                            egui::Color32::WHITE,
                        )
                    };

                    let fill_color = if is_enabled {
                        base_fill
                            .lerp_to_gamma(egui::Color32::WHITE, activity * ACTIVITY_BRIGHTEN_MAX)
                    } else {
                        base_fill
                    };

                    painter.circle(
                        pos,
                        op_radius,
                        fill_color,
                        egui::Stroke::new(if is_selected { 2.5 } else { 1.5 }, stroke_color),
                    );
                    painter.text(
                        pos,
                        egui::Align2::CENTER_CENTER,
                        format!("{}", op_num),
                        egui::FontId::proportional(10.0),
                        text_color,
                    );
                }

                // Output indicator
                let output_x = rect.right() - 20.0;
                let output_y = rect.center().y + 20.0;
                painter.text(
                    egui::pos2(output_x, output_y),
                    egui::Align2::CENTER_CENTER,
                    "OUT",
                    egui::FontId::proportional(8.0),
                    egui::Color32::from_rgb(100, 100, 100),
                );

                for &carrier in &alg_info.carriers {
                    let carrier_pos = positions[(carrier - 1) as usize];
                    painter.line_segment(
                        [
                            carrier_pos + egui::vec2(op_radius + 2.0, 0.0),
                            egui::pos2(output_x - 10.0, output_y),
                        ],
                        egui::Stroke::new(1.0, egui::Color32::from_rgb(70, 130, 180)),
                    );
                }
            });
        });
    }

    fn calculate_operator_positions_compact(
        &self,
        alg_info: &algorithms::AlgorithmInfo,
        rect: egui::Rect,
    ) -> [egui::Pos2; 6] {
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
                            .fill(if is_selected {
                                egui::Color32::from_rgb(240, 248, 255)
                            } else {
                                egui::Color32::from_rgb(250, 250, 250)
                            })
                            .stroke(egui::Stroke::new(
                                if is_selected { 2.5 } else { 1.0 },
                                if is_selected {
                                    egui::Color32::from_rgb(255, 180, 0)
                                } else {
                                    base_color
                                },
                            ))
                            .rounding(4.0)
                            .inner_margin(4.0);

                        frame.show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                // OP label with role
                                let role = if is_carrier { "C" } else { "M" };
                                let fb = if has_feedback { " F" } else { "" };
                                let label_text = format!("OP{} {}{}", op_num, role, fb);

                                if ui
                                    .selectable_label(
                                        is_selected,
                                        egui::RichText::new(label_text)
                                            .size(11.0)
                                            .color(base_color),
                                    )
                                    .clicked()
                                {
                                    self.selected_operator = op_idx;
                                }

                                // Level bar (vertical)
                                let bar_width = 40.0;
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
        // Display the larger of the two side depths so a single slider can
        // drive both the left and the right scaling jointly. Power users
        // can still tweak each side via JSON / future detail panel.
        let mut key_scale_lvl = op_snap
            .key_scale_left_depth
            .max(op_snap.key_scale_right_depth);
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
                    egui::RichText::new(format!("OPERATOR {} - {}{}", op_num, role, fb_text))
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
                // Parameters section
                ui.label(egui::RichText::new("PARAMETERS").size(10.0));
                egui::Grid::new("op_params_grid")
                    .num_columns(4)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("Ratio:");
                        if ui
                            .add(
                                egui::Slider::new(&mut freq_ratio, 0.5..=31.0)
                                    .step_by(1.0)
                                    .custom_formatter(|n, _| {
                                        format!(
                                            "{:.2}",
                                            crate::dx7_frequency::quantize_frequency_ratio(
                                                n as f32
                                            )
                                        )
                                    }),
                            )
                            .changed()
                        {
                            let q = crate::dx7_frequency::quantize_frequency_ratio(freq_ratio);
                            if let Ok(mut ctrl) = self.lock_controller() {
                                ctrl.set_operator_param(op_idx as u8, OperatorParam::Ratio, q);
                            }
                        }
                        ui.label("Level:");
                        if ui
                            .add(egui::Slider::new(&mut output_level, 0.0..=99.0).integer())
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
                            .add(egui::Slider::new(&mut detune, -7.0..=7.0).integer())
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
                        ui.label("Vel Sens:");
                        if ui
                            .add(egui::Slider::new(&mut vel_sens, 0.0..=7.0).integer())
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
                                .add(egui::Slider::new(&mut feedback, 0.0..=7.0).integer())
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
                        } else {
                            ui.label("");
                            ui.label("");
                        }
                        ui.label("Key Lvl:");
                        if ui
                            .add(egui::Slider::new(&mut key_scale_lvl, 0.0..=99.0).integer())
                            .changed()
                        {
                            if let Ok(mut ctrl) = self.lock_controller() {
                                // Drive both left and right depth identically from this single slider.
                                ctrl.set_operator_param(
                                    op_idx as u8,
                                    OperatorParam::KeyScaleLeftDepth,
                                    key_scale_lvl,
                                );
                                ctrl.set_operator_param(
                                    op_idx as u8,
                                    OperatorParam::KeyScaleRightDepth,
                                    key_scale_lvl,
                                );
                            }
                        }
                        ui.end_row();

                        ui.label("AM Sens:");
                        if ui
                            .add(egui::Slider::new(&mut am_sens, 0.0..=3.0).integer())
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
                        ui.label("Key Rate:");
                        if ui
                            .add(egui::Slider::new(&mut key_scale_rt, 0.0..=7.0).integer())
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
                                    egui::Slider::new(&mut fixed_hz, 1.0..=4000.0)
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
                            ui.label("");
                            ui.label("");
                            ui.end_row();
                        }
                    });

                ui.add_space(8.0);

                // Envelope section
                ui.label(egui::RichText::new("ENVELOPE").size(10.0));
                egui::Grid::new("op_env_grid")
                    .num_columns(4)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        // Row 1: Rates
                        ui.label("R1:");
                        if ui
                            .add(egui::Slider::new(&mut rate1, 0.0..=99.0).integer())
                            .changed()
                        {
                            if let Ok(mut ctrl) = self.lock_controller() {
                                ctrl.set_envelope_param(op_idx as u8, EnvelopeParam::Rate1, rate1);
                            }
                        }
                        ui.label("R2:");
                        if ui
                            .add(egui::Slider::new(&mut rate2, 0.0..=99.0).integer())
                            .changed()
                        {
                            if let Ok(mut ctrl) = self.lock_controller() {
                                ctrl.set_envelope_param(op_idx as u8, EnvelopeParam::Rate2, rate2);
                            }
                        }
                        ui.end_row();

                        ui.label("L1:");
                        if ui
                            .add(egui::Slider::new(&mut level1, 0.0..=99.0).integer())
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
                        ui.label("L2:");
                        if ui
                            .add(egui::Slider::new(&mut level2, 0.0..=99.0).integer())
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
                            .add(egui::Slider::new(&mut rate3, 0.0..=99.0).integer())
                            .changed()
                        {
                            if let Ok(mut ctrl) = self.lock_controller() {
                                ctrl.set_envelope_param(op_idx as u8, EnvelopeParam::Rate3, rate3);
                            }
                        }
                        ui.label("R4:");
                        if ui
                            .add(egui::Slider::new(&mut rate4, 0.0..=99.0).integer())
                            .changed()
                        {
                            if let Ok(mut ctrl) = self.lock_controller() {
                                ctrl.set_envelope_param(op_idx as u8, EnvelopeParam::Rate4, rate4);
                            }
                        }
                        ui.end_row();

                        ui.label("L3:");
                        if ui
                            .add(egui::Slider::new(&mut level3, 0.0..=99.0).integer())
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
                        ui.label("L4:");
                        if ui
                            .add(egui::Slider::new(&mut level4, 0.0..=99.0).integer())
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
    }

    fn draw_midi_panel(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new("MIDI / CONTROLLERS")
                        .size(14.0)
                        .strong(),
                );
                ui.separator();

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
            egui::ComboBox::from_id_source("midi_channel_combo")
                .selected_text(label)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(self.midi_channel_ui.is_none(), "OMNI (all channels)")
                        .clicked()
                    {
                        self.midi_channel_ui = None;
                        if let Some(handler) = self._midi_handler.as_ref() {
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
                            if let Some(handler) = self._midi_handler.as_ref() {
                                handler.set_channel(Some(ch));
                            }
                        }
                    }
                });
            ui.label(if self._midi_handler.is_some() {
                "MIDI device connected"
            } else {
                "(no MIDI device)"
            });
        });
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
                .add(egui::Slider::new(&mut v, 0..=max as i32).show_value(true))
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
            ui.label("file:");
            ui.add(egui::TextEdit::singleline(&mut self.sysex_path).desired_width(280.0));
        });
        ui.horizontal(|ui| {
            if ui.button("Load .syx").clicked() {
                self.load_sysex_from_path();
            }
            if ui.button("Save current voice").clicked() {
                self.save_sysex_to_path();
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

    fn load_sysex_from_path(&mut self) {
        let path = self.sysex_path.trim().to_string();
        match std::fs::read(&path) {
            Ok(bytes) => match crate::sysex::parse_message(&bytes) {
                Ok(crate::sysex::SysexResult::SingleVoice(preset)) => {
                    let name = preset.name.clone();
                    if let Ok(mut ctrl) = self.lock_controller() {
                        ctrl.load_sysex_single_voice(*preset);
                    }
                    self.sysex_status = format!("Loaded single voice '{}' from {}", name, path);
                }
                Ok(crate::sysex::SysexResult::Bulk(presets)) => {
                    let count = presets.len();
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

    fn save_sysex_to_path(&mut self) {
        let path = self.sysex_path.trim().to_string();
        let preset = Dx7Preset::from_snapshot(&self.snapshot);
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

/// Max fraction of white blended into an active operator's fill (0..=1).
/// Tunable: lower = subtler highlight, higher = whiter at full envelope.
const ACTIVITY_BRIGHTEN_MAX: f32 = 0.6;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fm_synth::create_synth;
    use crate::presets::{PresetLfo, PresetOperator, PresetPitchEg};

    fn make_app() -> Dx7App {
        make_app_with_presets(Vec::new())
    }

    fn make_app_with_presets(presets: Vec<Dx7Preset>) -> Dx7App {
        let (engine, controller) = create_synth(44_100.0);
        let engine = Arc::new(Mutex::new(engine));
        let controller = Arc::new(Mutex::new(controller));
        Dx7App::new_for_test(engine, controller, presets)
    }

    fn make_preset(name: &str, alg: u8, collection: &str) -> Dx7Preset {
        Dx7Preset {
            name: name.to_string(),
            collection: collection.to_string(),
            algorithm: alg,
            operators: std::array::from_fn(|_| PresetOperator::default()),
            master_tune: None,
            pitch_bend_range: None,
            portamento_enable: None,
            portamento_time: None,
            mono_mode: None,
            transpose_semitones: 0,
            pitch_mod_sensitivity: 0,
            pitch_eg: Some(PresetPitchEg::default()),
            lfo: Some(PresetLfo::default()),
        }
    }

    /// Run one egui frame against a fresh test context.
    fn run_one_frame<F: FnOnce(&egui::Context)>(f: F) {
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| f(ctx));
    }

    // ---------------------------------------------------------------------
    // Constructor / state
    // ---------------------------------------------------------------------

    #[test]
    fn new_for_test_initialises_default_state() {
        let app = make_app();
        assert_eq!(app.selected_operator, 0);
        assert_eq!(app.current_octave, 4);
        assert_eq!(app.display_text, "DX7 FM SYNTH");
        assert!(app._audio_engine.is_none());
        assert!(app._midi_handler.is_none());
        assert!(app.presets.is_empty());
        assert!(app.midi_channel_ui.is_none());
    }

    #[test]
    fn new_for_test_keeps_provided_presets() {
        let presets = vec![
            make_preset("FOO", 1, "edu"),
            make_preset("BAR", 2, "mark"),
        ];
        let app = make_app_with_presets(presets);
        assert_eq!(app.presets.len(), 2);
        assert_eq!(app.presets[0].name, "FOO");
    }

    #[test]
    fn lock_engine_and_controller_succeed() {
        let app = make_app();
        assert!(app.lock_engine().is_ok());
        assert!(app.lock_controller().is_ok());
    }

    #[test]
    fn update_snapshot_refreshes_field_from_controller() {
        let mut app = make_app();
        if let Ok(mut eng) = app.engine.lock() {
            eng.set_algorithm(11);
            eng.update_snapshot();
        }
        app.update_snapshot();
        assert_eq!(app.snapshot.algorithm, 11);
    }

    // ---------------------------------------------------------------------
    // Pure helper: calculate_operator_positions_compact
    // ---------------------------------------------------------------------

    #[test]
    fn operator_positions_lay_out_inside_rect_for_algorithm_1() {
        let app = make_app();
        let alg_info = algorithms::get_algorithm_info(1);
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(400.0, 280.0));
        let positions = app.calculate_operator_positions_compact(&alg_info, rect);
        // Every operator must land inside the rect.
        for (i, p) in positions.iter().enumerate() {
            assert!(rect.contains(*p), "op {} position {:?} outside rect", i + 1, p);
        }
    }

    #[test]
    fn operator_positions_unique_per_operator() {
        let app = make_app();
        for alg in 1..=32u8 {
            let alg_info = algorithms::get_algorithm_info(alg);
            let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(400.0, 280.0));
            let positions = app.calculate_operator_positions_compact(&alg_info, rect);
            // No two operators should occupy the exact same point.
            for i in 0..positions.len() {
                for j in (i + 1)..positions.len() {
                    let dx = (positions[i].x - positions[j].x).abs();
                    let dy = (positions[i].y - positions[j].y).abs();
                    assert!(
                        dx > 0.001 || dy > 0.001,
                        "alg {}: ops {} and {} overlap at {:?}",
                        alg, i + 1, j + 1, positions[i]
                    );
                }
            }
        }
    }

    #[test]
    fn operator_positions_carriers_at_bottom_layer() {
        let app = make_app();
        // Algorithm 32: all carriers — they should all share the bottom y.
        let alg_info = algorithms::get_algorithm_info(32);
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(400.0, 280.0));
        let positions = app.calculate_operator_positions_compact(&alg_info, rect);
        let bottom_y = positions[0].y;
        for p in &positions[1..] {
            assert!((p.y - bottom_y).abs() < 0.5, "alg 32: all ops should share bottom row");
        }
    }

    #[test]
    fn operator_positions_modulators_above_carriers() {
        let app = make_app();
        // Algorithm 1: ops 1 & 3 are carriers, the others are higher in the tree.
        let alg_info = algorithms::get_algorithm_info(1);
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(400.0, 280.0));
        let positions = app.calculate_operator_positions_compact(&alg_info, rect);
        // Op2 modulates Op1 → must sit above (smaller y) Op1.
        assert!(positions[1].y < positions[0].y);
        // Op6 → Op5 → Op4 → Op3 stack. Op6 should be the topmost.
        assert!(positions[5].y < positions[4].y);
        assert!(positions[4].y < positions[3].y);
    }

    // ---------------------------------------------------------------------
    // SysEx load / save
    // ---------------------------------------------------------------------

    fn temp_path(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("synth-fm-rs-gui-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("mkdir");
        dir.join(name)
    }

    #[test]
    fn save_sysex_writes_file_with_voice_name_in_status() {
        let mut app = make_app();
        let path = temp_path("save_voice.syx");
        app.sysex_path = path.to_string_lossy().into_owned();
        app.save_sysex_to_path();
        assert!(path.exists(), "save did not create file");
        assert!(app.sysex_status.contains("Saved"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_sysex_round_trips_a_saved_voice() {
        let mut app = make_app();
        let path = temp_path("roundtrip_voice.syx");
        app.sysex_path = path.to_string_lossy().into_owned();
        app.save_sysex_to_path();
        app.sysex_status.clear();
        app.load_sysex_from_path();
        assert!(app.sysex_status.contains("Loaded single voice"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_sysex_reports_read_error_for_missing_file() {
        let mut app = make_app();
        app.sysex_path = "/nonexistent/nope.syx".to_string();
        app.load_sysex_from_path();
        assert!(app.sysex_status.starts_with("Read error"));
    }

    #[test]
    fn load_sysex_reports_parse_error_for_garbage_content() {
        let mut app = make_app();
        let path = temp_path("garbage.syx");
        std::fs::write(&path, b"not a sysex message").expect("write");
        app.sysex_path = path.to_string_lossy().into_owned();
        app.load_sysex_from_path();
        assert!(app.sysex_status.starts_with("Parse error"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_sysex_handles_bulk_dump() {
        let msg = crate::sysex::build_sysex_message(9, &vec![0u8; crate::sysex::VMEM_LEN]);
        let path = temp_path("bulk.syx");
        std::fs::write(&path, &msg).expect("write");
        let mut app = make_app();
        app.sysex_path = path.to_string_lossy().into_owned();
        app.load_sysex_from_path();
        assert!(app.sysex_status.contains("bulk dump"));
        let _ = std::fs::remove_file(&path);
    }

    // ---------------------------------------------------------------------
    // Render path coverage — drives the full GUI for one frame per mode.
    // ---------------------------------------------------------------------

    #[test]
    fn render_voice_mode_completes_without_panic() {
        let mut app = make_app_with_presets(vec![
            make_preset("ONE", 1, "edu"),
            make_preset("TWO", 5, "mark"),
        ]);
        app.display_mode = DisplayMode::Voice;
        run_one_frame(|ctx| app.render(ctx));
    }

    #[test]
    fn render_operator_mode_completes_without_panic() {
        let mut app = make_app();
        app.display_mode = DisplayMode::Operator;
        run_one_frame(|ctx| app.render(ctx));
    }

    #[test]
    fn render_lfo_mode_completes_without_panic() {
        let mut app = make_app();
        app.display_mode = DisplayMode::LFO;
        run_one_frame(|ctx| app.render(ctx));
    }

    #[test]
    fn render_effects_mode_completes_without_panic() {
        let mut app = make_app();
        app.display_mode = DisplayMode::Effects;
        run_one_frame(|ctx| app.render(ctx));
    }

    #[test]
    fn render_midi_mode_completes_without_panic() {
        let mut app = make_app();
        app.display_mode = DisplayMode::Midi;
        run_one_frame(|ctx| app.render(ctx));
    }

    #[test]
    fn render_each_algorithm_in_operator_mode() {
        // Cycles through all 32 algorithms so the diagram layout / drawing code
        // is exercised on every routing.
        let mut app = make_app();
        app.display_mode = DisplayMode::Operator;
        for alg in 1..=32u8 {
            if let Ok(mut eng) = app.engine.lock() {
                eng.set_algorithm(alg);
                eng.update_snapshot();
            }
            run_one_frame(|ctx| app.render(ctx));
        }
    }

    #[test]
    fn render_with_collection_filter_active() {
        let presets = vec![
            make_preset("A1", 1, "edu"),
            make_preset("A2", 1, "mark"),
            make_preset("A3", 1, "edu"),
        ];
        let mut app = make_app_with_presets(presets);
        app.selected_collection = Some("edu".to_string());
        app.display_mode = DisplayMode::Voice;
        run_one_frame(|ctx| app.render(ctx));
    }

    #[test]
    fn render_with_search_filter_active() {
        let presets = vec![
            make_preset("PIANO 1", 1, "edu"),
            make_preset("BRASS 1", 1, "edu"),
            make_preset("PIANO 2", 1, "edu"),
        ];
        let mut app = make_app_with_presets(presets);
        app.preset_search = "piano".to_string();
        app.display_mode = DisplayMode::Voice;
        run_one_frame(|ctx| app.render(ctx));
    }

    #[test]
    fn render_with_active_voices_for_meter_path() {
        let mut app = make_app();
        if let Ok(mut ctrl) = app.controller.lock() {
            ctrl.note_on(60, 100);
        }
        if let Ok(mut eng) = app.engine.lock() {
            eng.process_commands();
            eng.update_snapshot();
        }
        app.display_mode = DisplayMode::Operator;
        run_one_frame(|ctx| app.render(ctx));
    }

    #[test]
    fn render_with_effects_enabled_exercises_effect_drawers() {
        let mut app = make_app();
        if let Ok(mut eng) = app.engine.lock() {
            eng.effects.chorus.enabled = true;
            eng.effects.delay.enabled = true;
            eng.effects.reverb.enabled = true;
            eng.update_snapshot();
        }
        app.display_mode = DisplayMode::Effects;
        run_one_frame(|ctx| app.render(ctx));
    }

    #[test]
    fn render_each_voice_mode_for_global_controls() {
        for mode in [
            crate::state_snapshot::VoiceMode::Poly,
            crate::state_snapshot::VoiceMode::Mono,
            crate::state_snapshot::VoiceMode::MonoLegato,
        ] {
            let mut app = make_app();
            if let Ok(mut ctrl) = app.controller.lock() {
                ctrl.set_voice_mode(mode);
            }
            if let Ok(mut eng) = app.engine.lock() {
                eng.process_commands();
                eng.update_snapshot();
            }
            app.update_snapshot();
            run_one_frame(|ctx| app.render(ctx));
        }
    }

    #[test]
    fn render_with_midi_channel_filter_set() {
        let mut app = make_app();
        app.display_mode = DisplayMode::Midi;
        app.midi_channel_ui = Some(3);
        run_one_frame(|ctx| app.render(ctx));
        app.midi_channel_ui = None;
        run_one_frame(|ctx| app.render(ctx));
    }

    #[test]
    fn render_with_pitch_eg_active_in_lfo_panel() {
        let mut app = make_app();
        if let Ok(mut eng) = app.engine.lock() {
            eng.pitch_eg.enabled = true;
            eng.pitch_eg.level1 = 80.0;
            eng.update_snapshot();
        }
        app.display_mode = DisplayMode::LFO;
        run_one_frame(|ctx| app.render(ctx));
    }

    // ---------------------------------------------------------------------
    // Constants are stable
    // ---------------------------------------------------------------------

    #[test]
    fn activity_brighten_max_in_unit_range() {
        assert!((0.0..=1.0).contains(&ACTIVITY_BRIGHTEN_MAX));
    }
}
