use eframe::egui;
use std::thread;
use std::time::Duration;

mod algorithms;
mod audio_engine;
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
mod presets;

use audio_engine::AudioEngine;
use fm_synth::FmSynthesizer;
use gui::Dx7App;
use midi_handler::MidiHandler;

fn play_startup_melody(synth: std::sync::Arc<std::sync::Mutex<FmSynthesizer>>) {
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(500));

        // Play a simple C major triad as startup sound
        let notes = [60, 64, 67]; // C4, E4, G4
        let note_duration = Duration::from_millis(300);
        let note_gap = Duration::from_millis(50);

        for &note in &notes {
            if let Ok(mut synth_guard) = synth.lock() {
                synth_guard.note_on(note, 80);
            }

            thread::sleep(note_duration);

            if let Ok(mut synth_guard) = synth.lock() {
                synth_guard.note_off(note);
            }

            thread::sleep(note_gap);
        }
    });
}

fn main() -> Result<(), eframe::Error> {
    // Initialize logging system
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Starting Yamaha DX7 Emulator");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([960.0, 720.0])
            .with_resizable(false)
            .with_title("Yamaha DX7 Emulator"),
        ..Default::default()
    };

    let (audio_engine, synth) = AudioEngine::new_with_synth_setup();

    let _midi_handler = match MidiHandler::new(synth.clone()) {
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
    play_startup_melody(synth.clone());

    eframe::run_native(
        "Yamaha DX7 Emulator",
        options,
        Box::new(move |_cc| Ok(Box::new(Dx7App::new(synth, audio_engine, _midi_handler)))),
    )
}
