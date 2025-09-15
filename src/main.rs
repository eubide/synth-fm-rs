use eframe::egui;
use std::thread;
use std::time::Duration;

mod algorithms;
mod audio_engine;
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

        let notes = [60, 64, 67]; // C4, E4, G4 - simple C major triad
        let _note_duration = Duration::from_millis(800);
        let note_gap = Duration::from_millis(100);

        for &note in &notes {
            println!("Playing note {} (velocity 80)", note);
            if let Ok(mut synth_guard) = synth.lock() {
                synth_guard.note_on(note, 80);
                // Test if synth is producing output
                let output = synth_guard.process();
                println!("Initial synth output: {:.6}", output);
            } else {
                println!("Failed to lock synth for note_on");
            }

            // Sample synth output during note duration
            for i in 0..10 {
                thread::sleep(Duration::from_millis(80));
                if let Ok(mut synth_guard) = synth.lock() {
                    let output = synth_guard.process();
                    if i == 0 || i == 9 || output.abs() > 0.001 {
                        println!("Sample {}: {:.6}", i, output);
                    }
                }
            }

            println!("Releasing note {}", note);
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
            .with_inner_size([900.0, 700.0])
            .with_title("Yamaha DX7 Emulator"),
        ..Default::default()
    };

    let (audio_engine, synth) = AudioEngine::new_with_synth_setup();

    let _midi_handler = match MidiHandler::new(synth.clone()) {
        Ok(handler) => {
            println!("MIDI input initialized successfully");
            Some(handler)
        }
        Err(e) => {
            println!("Failed to initialize MIDI input: {}", e);
            println!("Continuing without MIDI support...");
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
