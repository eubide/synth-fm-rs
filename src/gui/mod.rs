mod algorithm;
mod effects;
mod header;
mod lfo;
mod midi;
mod operator;
mod voice;

#[cfg(test)]
mod tests;

use crate::audio_engine::AudioEngine;
use crate::fm_synth::SynthController;
use crate::midi_handler::MidiHandler;
use crate::presets::Dx7Preset;
use crate::state_snapshot::SynthSnapshot;
use eframe::egui;
use std::sync::{Arc, Mutex};

pub struct Dx7App {
    /// Tests drive `process_commands` by hand; the app itself only talks to
    /// the engine through the controller's command queue.
    #[cfg(test)]
    engine: Option<Arc<Mutex<crate::fm_synth::SynthEngine>>>,
    controller: Arc<Mutex<SynthController>>,
    /// Optional so unit tests can construct a `Dx7App` without a real audio
    /// or MIDI device.
    audio_engine: Option<AudioEngine>,
    midi_handler: Option<MidiHandler>,
    /// `None` forces a device poll on the next frame.
    last_device_poll: Option<std::time::Instant>,
    midi_ports: Vec<String>,
    audio_devices: Vec<String>,
    /// Last device change or failure shown in the MIDI panel.
    device_status: String,
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
    /// Last SysEx file loaded or saved; the file dialogs open in its folder.
    sysex_path: String,
    /// Last status line shown in the MIDI panel (load/save feedback).
    sysex_status: String,
    /// Cached MIDI channel selection: None = OMNI, Some(0..15) = specific channel.
    midi_channel_ui: Option<u8>,
    /// Name the next save will use; reset whenever a different voice loads.
    voice_name: String,
    /// Last status line shown under the voice name (save feedback).
    voice_status: String,
    user_patches_dir: std::path::PathBuf,
    /// Edit buffer parked while COMPARE plays the stored original.
    compare_edit: Option<Dx7Preset>,
}

const USER_COLLECTION: &str = "user";

/// How often MIDI ports are rescanned and a lost audio device is retried.
const DEVICE_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);

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
        controller: Arc<Mutex<SynthController>>,
        audio_engine: AudioEngine,
        midi_handler: Option<MidiHandler>,
        presets: Vec<Dx7Preset>,
    ) -> Self {
        Self::build(controller, Some(audio_engine), midi_handler, presets)
    }

    /// Test-only constructor: builds a `Dx7App` without a real audio engine.
    #[cfg(test)]
    pub fn new_for_test(
        engine: Arc<Mutex<crate::fm_synth::SynthEngine>>,
        controller: Arc<Mutex<SynthController>>,
        presets: Vec<Dx7Preset>,
    ) -> Self {
        let mut app = Self::build(controller, None, None, presets);
        app.engine = Some(engine);
        app
    }

    #[cfg(test)]
    fn test_engine(&self) -> &Arc<Mutex<crate::fm_synth::SynthEngine>> {
        self.engine.as_ref().expect("built with new_for_test")
    }

    fn build(
        controller: Arc<Mutex<SynthController>>,
        audio_engine: Option<AudioEngine>,
        midi_handler: Option<MidiHandler>,
        presets: Vec<Dx7Preset>,
    ) -> Self {
        let snapshot = controller.lock().map(|c| c.snapshot()).unwrap_or_default();
        Self {
            #[cfg(test)]
            engine: None,
            controller,
            audio_engine,
            midi_handler,
            last_device_poll: None,
            midi_ports: Vec::new(),
            audio_devices: Vec::new(),
            device_status: String::new(),
            selected_operator: 0,
            display_mode: DisplayMode::Voice,
            display_text: "DX7 FM SYNTH".to_string(),
            last_key_times: std::collections::HashMap::new(),
            current_octave: 4,
            presets,
            selected_preset: 0,
            selected_collection: None,
            preset_search: String::new(),
            sysex_path: String::from("voice.syx"),
            sysex_status: String::new(),
            midi_channel_ui: None,
            voice_name: snapshot.preset_name.clone(),
            voice_status: String::new(),
            user_patches_dir: std::path::Path::new(crate::preset_loader::PATCHES_DIR)
                .join(USER_COLLECTION),
            compare_edit: None,
            snapshot,
        }
    }

    /// Update the cached snapshot from the audio thread (call once per frame)
    fn update_snapshot(&mut self) {
        if let Ok(ctrl) = self.controller.lock() {
            let snapshot = ctrl.snapshot();
            if snapshot.preset_name != self.snapshot.preset_name {
                self.voice_name = snapshot.preset_name.clone();
            }
            self.snapshot = snapshot;
        }
    }

    /// Frame-independent rendering: drives one full GUI frame into the root
    /// `Ui`. Split out from `App::ui` so tests can call it without
    /// constructing an `eframe::Frame`.
    pub(crate) fn render(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.update_snapshot();
        self.poll_devices();
        self.handle_keyboard_input(&ctx);
        ctx.set_visuals(egui::Visuals::light());

        egui::CentralPanel::default().show(ui, |ui| {
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
                    ui.horizontal_top(|ui| {
                        self.draw_algorithm_diagram_compact(ui);
                        ui.add_space(8.0);
                        ui.vertical(|ui| {
                            self.draw_operator_full_panel(ui);
                        });
                    });
                    ui.add_space(4.0);
                    self.draw_operator_selector_strip(ui);
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
        } else {
            ctx.request_repaint_after(DEVICE_POLL_INTERVAL);
        }
    }

    fn poll_devices(&mut self) {
        if self
            .last_device_poll
            .is_some_and(|t| t.elapsed() < DEVICE_POLL_INTERVAL)
        {
            return;
        }
        self.last_device_poll = Some(std::time::Instant::now());
        let panel_open = self.display_mode == DisplayMode::Midi;

        if let Some(midi) = self.midi_handler.as_mut() {
            midi.refresh();
            if panel_open {
                self.midi_ports = midi.port_names();
            }
        }
        if let Some(audio) = self.audio_engine.as_mut() {
            match audio.recover_if_failed() {
                Some(Ok(name)) => {
                    self.device_status = format!("Audio device lost; now playing on {}", name)
                }
                Some(Err(e)) => self.device_status = format!("Audio device lost: {}", e),
                None => {}
            }
            if panel_open {
                self.audio_devices = AudioEngine::output_device_names();
            }
        }
    }

    fn lock_controller(
        &self,
    ) -> Result<
        std::sync::MutexGuard<'_, SynthController>,
        std::sync::PoisonError<std::sync::MutexGuard<'_, SynthController>>,
    > {
        self.controller.lock()
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
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.render(ui);
    }
}

/// egui's default clamping snaps a slider's value to its range, step and
/// decimals on every frame and reports that as a change; every change here
/// becomes a synth command, so only user edits may write.
fn edit_slider<'a, Num: egui::emath::Numeric>(
    value: &'a mut Num,
    range: std::ops::RangeInclusive<Num>,
) -> egui::Slider<'a> {
    egui::Slider::new(value, range).clamping(egui::SliderClamping::Edits)
}
