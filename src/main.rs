use eframe::egui;
use std::sync::{Arc, Mutex};

mod fm_synth;
mod audio_engine;
mod gui;
mod midi_handler;
mod operator;
mod envelope;
mod algorithms;
mod presets;

use fm_synth::FmSynthesizer;
use audio_engine::AudioEngine;
use gui::Dx7App;
use midi_handler::MidiHandler;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 500.0])
            .with_title("Yamaha DX7 Emulator"),
        ..Default::default()
    };

    let synth = Arc::new(Mutex::new(FmSynthesizer::new()));
    let audio_engine = AudioEngine::new(synth.clone());
    
    let _midi_handler = match MidiHandler::new(synth.clone()) {
        Ok(handler) => {
            println!("MIDI input initialized successfully");
            Some(handler)
        },
        Err(e) => {
            println!("Failed to initialize MIDI input: {}", e);
            println!("Continuing without MIDI support...");
            None
        }
    };
    
    eframe::run_native(
        "Yamaha DX7 Emulator",
        options,
        Box::new(move |_cc| Ok(Box::new(Dx7App::new(synth, audio_engine, _midi_handler)))),
    )
}