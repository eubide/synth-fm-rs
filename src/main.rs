use eframe::egui;
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

mod algorithms;
mod audio_engine;
mod command_queue;
mod dx7_frequency;
mod effects;
mod envelope;
mod fm_synth;
mod gui;
mod lfo;
mod lock_free;
mod midi_handler;
mod operator;
mod optimization;
mod pitch_eg;
mod preset_loader;
mod presets;
mod state_snapshot;
mod sysex;

use audio_engine::AudioEngine;
use fm_synth::{create_synth, SynthController};
use gui::Dx7App;
use midi_handler::MidiHandler;

fn play_startup_melody(controller: Arc<Mutex<SynthController>>) {
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(500));

        let notes = [60, 64, 67]; // C4, E4, G4
        let note_duration = Duration::from_millis(300);
        let note_gap = Duration::from_millis(50);

        for &note in &notes {
            if let Ok(mut ctrl) = controller.lock() {
                ctrl.note_on(note, 80);
            }

            thread::sleep(note_duration);

            if let Ok(mut ctrl) = controller.lock() {
                ctrl.note_off(note);
            }

            thread::sleep(note_gap);
        }
    });
}

fn main() -> Result<(), eframe::Error> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Starting DX7-Style FM Synthesizer");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([960.0, 720.0])
            .with_resizable(false)
            .with_title("DX7-Style FM Synthesizer"),
        ..Default::default()
    };

    // Get sample rate from audio device
    let sample_rate = AudioEngine::get_default_sample_rate();

    // Create synthesizer engine and controller
    let (engine, controller) = create_synth(sample_rate);
    let engine = Arc::new(Mutex::new(engine));
    let controller = Arc::new(Mutex::new(controller));

    let patches_dir = std::path::Path::new("patches");
    let presets = preset_loader::scan_patches_dir(patches_dir);
    if presets.is_empty() {
        log::warn!("No presets found in {:?} — add JSON files to patches/ subdirectories", patches_dir);
    }

    // Apply the first preset and hand the full list to the engine (for MIDI PC).
    if let Ok(mut eng) = engine.lock() {
        eng.set_presets(presets.clone());
        if let Some(first) = presets.first() {
            first.apply_to_synth(&mut eng);
        }
    }

    // Create audio engine
    let underrun_counter = Arc::new(AtomicUsize::new(0));
    let audio_engine = AudioEngine::new(engine.clone(), underrun_counter);

    // Create MIDI handler
    let _midi_handler = match MidiHandler::new(controller.clone()) {
        Ok(handler) => {
            log::info!("MIDI input initialized successfully");
            Some(handler)
        }
        Err(e) => {
            log::warn!("Failed to initialize MIDI input: {}", e);
            log::info!("Continuing without MIDI support...");
            None
        }
    };

    // Play startup melody
    play_startup_melody(controller.clone());

    eframe::run_native(
        "DX7-Style FM Synthesizer",
        options,
        Box::new(move |_cc| {
            Ok(Box::new(Dx7App::new(
                engine,
                controller,
                audio_engine,
                _midi_handler,
                presets,
            )))
        }),
    )
}
