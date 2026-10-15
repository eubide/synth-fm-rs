use super::algorithm::ACTIVITY_BRIGHTEN_MAX;
use super::*;
use crate::algorithms;
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
        transpose_semitones: 0,
        pitch_mod_sensitivity: 0,
        pitch_eg: Some(PresetPitchEg::default()),
        lfo: Some(PresetLfo::default()),
    }
}

thread_local! {
    // Building egui's font atlas dominates a frame in debug builds; reuse
    // one context per test thread.
    static TEST_CTX: egui::Context = egui::Context::default();
}

/// Run one egui frame against this thread's test context.
fn run_one_frame<F: FnMut(&mut egui::Ui)>(mut f: F) {
    TEST_CTX.with(|ctx| {
        // No renderer consumes the font atlas upload here; egui asserts on
        // dropping it unhandled.
        ctx.run_ui(egui::RawInput::default(), |ui| f(ui))
            .textures_delta
            .clear();
    });
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
    assert!(app.audio_engine.is_none());
    assert!(app.midi_handler.is_none());
    assert!(app.presets.is_empty());
    assert!(app.midi_channel_ui.is_none());
}

#[test]
fn new_for_test_keeps_provided_presets() {
    let presets = vec![make_preset("FOO", 1, "edu"), make_preset("BAR", 2, "mark")];
    let app = make_app_with_presets(presets);
    assert_eq!(app.presets.len(), 2);
    assert_eq!(app.presets[0].name, "FOO");
}

#[test]
fn lock_controller_succeeds() {
    let app = make_app();
    assert!(app.lock_controller().is_ok());
}

#[test]
fn select_preset_reaches_the_engine_through_the_command_queue() {
    let mut app = make_app_with_presets(vec![make_preset("SEVEN", 7, "edu")]);
    app.select_preset(0);

    let algorithm_seen = |app: &mut Dx7App| {
        if let Ok(eng) = app.test_engine().lock() {
            eng.update_snapshot();
        }
        app.update_snapshot();
        app.snapshot.algorithm
    };
    assert_ne!(
        algorithm_seen(&mut app),
        7,
        "GUI wrote to the engine directly"
    );

    if let Ok(mut eng) = app.test_engine().lock() {
        eng.process_commands();
    }
    assert_eq!(algorithm_seen(&mut app), 7);
    assert_eq!(app.selected_preset, 0);
    assert_eq!(app.display_text, "LOADED: SEVEN");
}

#[test]
fn update_snapshot_refreshes_field_from_controller() {
    let mut app = make_app();
    if let Ok(mut eng) = app.test_engine().lock() {
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
        assert!(
            rect.contains(*p),
            "op {} position {:?} outside rect",
            i + 1,
            p
        );
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
                    alg,
                    i + 1,
                    j + 1,
                    positions[i]
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
        assert!(
            (p.y - bottom_y).abs() < 0.5,
            "alg 32: all ops should share bottom row"
        );
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

#[test]
fn load_sysex_bulk_shows_the_bank_as_a_browser_collection() {
    let msg = crate::sysex::build_sysex_message(9, &vec![0u8; crate::sysex::VMEM_LEN]);
    let path = temp_path("MyBank.syx");
    std::fs::write(&path, &msg).expect("write");
    let mut app = make_app_with_presets(vec![make_preset("JSON", 1, "edu")]);
    app.sysex_path = path.to_string_lossy().into_owned();

    app.load_sysex_from_path();
    app.load_sysex_from_path();

    let bank: Vec<_> = app
        .presets
        .iter()
        .filter(|p| p.collection == "MyBank")
        .collect();
    assert_eq!(bank.len(), 32, "reloading must not duplicate the bank");
    assert_eq!(app.presets.len(), 33);
    assert_eq!(app.presets[app.selected_preset].collection, "MyBank");
    assert_eq!(app.selected_collection.as_deref(), Some("MyBank"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn load_sysex_single_voice_is_selectable_in_the_browser() {
    let mut app = make_app();
    let path = temp_path("OneVoice.syx");
    app.sysex_path = path.to_string_lossy().into_owned();
    app.save_sysex_to_path();

    app.load_sysex_from_path();

    assert_eq!(app.presets.len(), 1);
    assert_eq!(app.presets[app.selected_preset].collection, "OneVoice");
    let _ = std::fs::remove_file(&path);
}

// ---------------------------------------------------------------------
// Render path coverage — drives the full GUI for one frame per mode.
// ---------------------------------------------------------------------

fn user_dir_for_test(app: &mut Dx7App, name: &str) -> std::path::PathBuf {
    let dir = temp_path(name).join("user");
    let _ = std::fs::remove_dir_all(&dir);
    app.user_patches_dir = dir.clone();
    dir
}

#[test]
fn save_voice_writes_json_and_selects_it_in_the_user_collection() {
    let mut app = make_app_with_presets(vec![make_preset("JSON", 1, "edu")]);
    let dir = user_dir_for_test(&mut app, "save-voice");
    app.voice_name = "MY PAD".to_string();

    app.save_voice();
    app.save_voice();

    assert!(dir.join("my-pad.json").exists());
    let user: Vec<_> = app
        .presets
        .iter()
        .filter(|p| p.collection == "user")
        .collect();
    assert_eq!(user.len(), 1, "saving the same name twice must overwrite");
    assert_eq!(app.presets[app.selected_preset].name, "MY PAD");
    assert!(app.voice_status.contains("my-pad.json"));

    let mut eng = app.test_engine().lock().unwrap();
    eng.process_commands();
    assert_eq!(eng.preset_name, "MY PAD");
    assert!(!eng.voice_edited);
    drop(eng);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn voice_name_follows_the_loaded_voice() {
    let mut app = make_app_with_presets(vec![make_preset("FIRST", 1, "edu")]);
    app.select_preset(0);
    if let Ok(mut eng) = app.test_engine().lock() {
        eng.process_commands();
        eng.update_snapshot();
    }
    app.update_snapshot();
    assert_eq!(app.voice_name, "FIRST");

    app.voice_name = "TYPED".to_string();
    app.update_snapshot();
    assert_eq!(app.voice_name, "TYPED", "typing must survive repaints");
}

/// Lets the engine consume queued commands and the GUI see the result.
fn sync(app: &mut Dx7App) {
    if let Ok(mut eng) = app.test_engine().lock() {
        eng.process_commands();
        eng.update_snapshot();
    }
    app.update_snapshot();
}

fn app_with_edited_voice() -> Dx7App {
    let mut app = make_app_with_presets(vec![
        make_preset("ORIG", 3, "edu"),
        make_preset("OTHER", 5, "edu"),
    ]);
    app.select_preset(0);
    sync(&mut app);
    app.lock_controller().unwrap().set_algorithm(9);
    sync(&mut app);
    app
}

#[test]
fn compare_toggles_between_the_original_and_the_edit_buffer() {
    let mut app = app_with_edited_voice();

    app.toggle_compare();
    sync(&mut app);
    assert!(app.compare_edit.is_some());
    assert_eq!(app.snapshot.algorithm, 3);
    assert!(!app.snapshot.voice_edited);

    app.toggle_compare();
    sync(&mut app);
    assert!(app.compare_edit.is_none());
    assert_eq!(app.snapshot.algorithm, 9);
    assert!(app.snapshot.voice_edited);
}

#[test]
fn compare_needs_an_edited_voice() {
    let mut app = make_app_with_presets(vec![make_preset("ORIG", 3, "edu")]);
    app.select_preset(0);
    sync(&mut app);
    app.toggle_compare();
    assert!(app.compare_edit.is_none());
}

#[test]
fn loading_another_voice_leaves_compare() {
    let mut app = app_with_edited_voice();
    app.toggle_compare();
    sync(&mut app);

    app.select_preset(1);
    sync(&mut app);

    assert!(app.compare_edit.is_none());
    assert_eq!(app.snapshot.algorithm, 5);
}

#[cfg(target_os = "macos")]
#[test]
fn opening_the_midi_panel_lists_a_newly_plugged_port() {
    use midir::os::unix::VirtualOutput;
    let name = format!("synth-fm-rs gui test {}", std::process::id());
    let mut app = make_app();
    app.midi_handler = Some(MidiHandler::new(app.controller.clone()));
    app.poll_devices();

    let _keyboard = midir::MidiOutput::new("synth-fm-rs gui test")
        .expect("midi out")
        .create_virtual(&name)
        .expect("virtual port");
    app.display_mode = DisplayMode::Midi;
    app.last_device_poll = None;
    app.poll_devices();

    assert!(app.midi_ports.contains(&name));
}

#[test]
fn idle_frames_do_not_mark_the_voice_edited() {
    // Off-grid values as found in bundled patches (e.g. detune 4.2 in
    // edu/BASS 1, ratio 1.41 from SysEx coarse+fine).
    let mut voice = make_preset("FRACTIONAL", 5, "edu");
    voice.operators[0].frequency_ratio = 1.41;
    voice.operators[0].detune = 4.2;
    voice.operators[0].output_level = 75.5;
    let mut app = make_app();
    app.lock_controller().unwrap().load_voice(voice);
    app.test_engine().lock().unwrap().process_commands();
    for mode in [
        DisplayMode::Voice,
        DisplayMode::Operator,
        DisplayMode::LFO,
        DisplayMode::Effects,
        DisplayMode::Midi,
    ] {
        app.display_mode = mode;
        run_one_frame(|ui| app.render(ui));
    }
    let mut eng = app.test_engine().lock().unwrap();
    eng.process_commands();
    assert!(!eng.voice_edited);
}

#[test]
fn render_voice_mode_completes_without_panic() {
    let mut app = make_app_with_presets(vec![
        make_preset("ONE", 1, "edu"),
        make_preset("TWO", 5, "mark"),
    ]);
    app.display_mode = DisplayMode::Voice;
    run_one_frame(|ui| app.render(ui));
}

#[test]
fn render_operator_mode_completes_without_panic() {
    let mut app = make_app();
    app.display_mode = DisplayMode::Operator;
    run_one_frame(|ui| app.render(ui));
}

#[test]
fn render_lfo_mode_completes_without_panic() {
    let mut app = make_app();
    app.display_mode = DisplayMode::LFO;
    run_one_frame(|ui| app.render(ui));
}

#[test]
fn render_effects_mode_completes_without_panic() {
    let mut app = make_app();
    app.display_mode = DisplayMode::Effects;
    run_one_frame(|ui| app.render(ui));
}

#[test]
fn render_midi_mode_completes_without_panic() {
    let mut app = make_app();
    app.display_mode = DisplayMode::Midi;
    run_one_frame(|ui| app.render(ui));
}

#[test]
fn render_each_algorithm_in_operator_mode() {
    // Cycles through all 32 algorithms so the diagram layout / drawing code
    // is exercised on every routing.
    let mut app = make_app();
    app.display_mode = DisplayMode::Operator;
    for alg in 1..=32u8 {
        if let Ok(mut eng) = app.test_engine().lock() {
            eng.set_algorithm(alg);
            eng.update_snapshot();
        }
        run_one_frame(|ui| app.render(ui));
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
    run_one_frame(|ui| app.render(ui));
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
    run_one_frame(|ui| app.render(ui));
}

#[test]
fn render_with_active_voices_for_meter_path() {
    let mut app = make_app();
    if let Ok(mut ctrl) = app.controller.lock() {
        ctrl.note_on(60, 100);
    }
    if let Ok(mut eng) = app.test_engine().lock() {
        eng.process_commands();
        eng.update_snapshot();
    }
    app.display_mode = DisplayMode::Operator;
    run_one_frame(|ui| app.render(ui));
}

#[test]
fn render_with_effects_enabled_exercises_effect_drawers() {
    let mut app = make_app();
    if let Ok(mut eng) = app.test_engine().lock() {
        eng.effects.chorus.enabled = true;
        eng.effects.delay.enabled = true;
        eng.effects.reverb.enabled = true;
        eng.update_snapshot();
    }
    app.display_mode = DisplayMode::Effects;
    run_one_frame(|ui| app.render(ui));
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
        if let Ok(mut eng) = app.test_engine().lock() {
            eng.process_commands();
            eng.update_snapshot();
        }
        app.update_snapshot();
        run_one_frame(|ui| app.render(ui));
    }
}

#[test]
fn render_with_midi_channel_filter_set() {
    let mut app = make_app();
    app.display_mode = DisplayMode::Midi;
    app.midi_channel_ui = Some(3);
    run_one_frame(|ui| app.render(ui));
    app.midi_channel_ui = None;
    run_one_frame(|ui| app.render(ui));
}

#[test]
fn render_with_pitch_eg_active_in_lfo_panel() {
    let mut app = make_app();
    if let Ok(mut eng) = app.test_engine().lock() {
        eng.pitch_eg.enabled = true;
        eng.pitch_eg.level1 = 80.0;
        eng.update_snapshot();
    }
    app.display_mode = DisplayMode::LFO;
    run_one_frame(|ui| app.render(ui));
}

// ---------------------------------------------------------------------
// Constants are stable
// ---------------------------------------------------------------------

#[test]
fn activity_brighten_max_in_unit_range() {
    assert!((0.0..=1.0).contains(&ACTIVITY_BRIGHTEN_MAX));
}
