use eframe::egui;

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
use gui::Dx7App;
use midi_handler::MidiHandler;

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

    eframe::run_native(
        "Yamaha DX7 Emulator",
        options,
        Box::new(move |_cc| Ok(Box::new(Dx7App::new(synth, audio_engine, _midi_handler)))),
    )
}
