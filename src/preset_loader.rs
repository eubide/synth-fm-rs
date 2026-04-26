use crate::lfo::LFOWaveform;
use crate::operator::KeyScaleCurve;
use crate::presets::{Dx7Preset, PresetLfo, PresetOperator, PresetPitchEg};
use serde::{Deserialize, Deserializer};
use std::path::Path;

#[derive(Deserialize, Default)]
#[serde(default)]
struct JsonEg {
    rate1: f32,
    rate2: f32,
    rate3: f32,
    rate4: f32,
    level1: f32,
    level2: f32,
    level3: f32,
    level4: f32,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct JsonKeyboardLevelScaling {
    breakpoint: serde_json::Value,
    left_curve: String,
    right_curve: String,
    left_depth: f32,
    right_depth: f32,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct JsonOperator {
    frequency: f32,
    output_level: f32,
    detune: f32,
    feedback: f32,
    eg: JsonEg,
    key_velocity_sensitivity: u8,
    keyboard_rate_scaling: u8,
    keyboard_level_scaling: Option<JsonKeyboardLevelScaling>,
    am_sensitivity: u8,
    oscillator_mode: String, // "ratio" | "fixed"
    /// DX7 fixed-mode coarse multiplier (0-31). Only used when oscillator_mode == "fixed".
    fixed_frequency_coarse: f32,
    /// DX7 fixed-mode fine value (0-99). Only used when oscillator_mode == "fixed".
    fixed_frequency_fine: f32,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct JsonPitchEg {
    rate1: f32,
    rate2: f32,
    rate3: f32,
    rate4: f32,
    level1: f32,
    level2: f32,
    level3: f32,
    level4: f32,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct JsonLfo {
    wave: String,
    speed: f32,
    delay: f32,
    pitch_mod_depth: f32,
    /// `amDepth` is sometimes encoded as a string in third-party banks; accept both.
    #[serde(deserialize_with = "deserialize_lenient_f32")]
    am_depth: f32,
    sync: String, // "on" | "off"
    pitch_mod_sensitivity: u8,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsonPatch {
    name: String,
    algorithm: u8,
    #[serde(default)]
    feedback: f32,
    #[serde(default)]
    operators: Vec<JsonOperator>,
    #[serde(default)]
    lfo: Option<JsonLfo>,
    /// itsjoesullivan banks use the literal key `pitchEG` (uppercase EG), not
    /// the camelCase `pitchEg` derived by serde.
    #[serde(default, rename = "pitchEG")]
    pitch_eg: Option<JsonPitchEg>,
    #[serde(default)]
    transpose: serde_json::Value,
    #[serde(default)]
    oscillator_key_sync: String,
}

/// Accept either a JSON number or a string-encoded number (some banks use "0" for amDepth).
fn deserialize_lenient_f32<'de, D>(deserializer: D) -> Result<f32, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(match value {
        serde_json::Value::Number(n) => n.as_f64().unwrap_or(0.0) as f32,
        serde_json::Value::String(s) => s.trim().parse::<f32>().unwrap_or(0.0),
        _ => 0.0,
    })
}

/// Parse "C3", "A-1", "F#4", or a raw integer into a MIDI note number.
/// DX7 reference: A-1 = MIDI 21, C3 = MIDI 60.
fn parse_breakpoint(value: &serde_json::Value) -> u8 {
    match value {
        serde_json::Value::Number(n) => n.as_u64().unwrap_or(60).clamp(0, 127) as u8,
        serde_json::Value::String(s) => parse_note_name(s).unwrap_or(60),
        _ => 60,
    }
}

fn parse_note_name(s: &str) -> Option<u8> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }
    let bytes = trimmed.as_bytes();
    let letter = bytes[0].to_ascii_uppercase();
    let mut idx = 1;
    let mut accidental = 0i32;
    if idx < bytes.len() {
        match bytes[idx] {
            b'#' => {
                accidental = 1;
                idx += 1;
            }
            b'b' => {
                accidental = -1;
                idx += 1;
            }
            _ => {}
        }
    }
    let octave_str = &trimmed[idx..];
    let octave: i32 = octave_str.parse().ok()?;
    let semitone = match letter {
        b'C' => 0,
        b'D' => 2,
        b'E' => 4,
        b'F' => 5,
        b'G' => 7,
        b'A' => 9,
        b'B' => 11,
        _ => return None,
    };
    // MIDI: C-1 = 0, C0 = 12, C3 = 48, C4 = 60. The DX7 manual labels middle C
    // as C3, which corresponds to MIDI note 60 — i.e. octave_offset = 12 here.
    let midi = (octave + 2) * 12 + semitone + accidental;
    if (0..=127).contains(&midi) {
        Some(midi as u8)
    } else {
        None
    }
}

/// Parse the DX7 transpose JSON value: `"C3"` ⇒ 0, `"C2"` ⇒ -12, integer ⇒ direct semitones.
fn parse_transpose(value: &serde_json::Value) -> i8 {
    match value {
        serde_json::Value::Number(n) => n.as_i64().unwrap_or(0).clamp(-24, 24) as i8,
        serde_json::Value::String(s) => match parse_note_name(s) {
            // C3 (MIDI 60) is the DX7 reference — no transpose.
            Some(midi) => ((midi as i32) - 60).clamp(-24, 24) as i8,
            None => 0,
        },
        _ => 0,
    }
}

fn parse_lfo_wave(s: &str) -> LFOWaveform {
    match s.trim().to_ascii_lowercase().as_str() {
        "triangle" | "tri" => LFOWaveform::Triangle,
        "sawdown" | "saw down" | "saw-down" | "saw_down" => LFOWaveform::SawDown,
        "sawup" | "saw up" | "saw-up" | "saw_up" => LFOWaveform::SawUp,
        "square" | "sqr" => LFOWaveform::Square,
        "sine" | "sin" => LFOWaveform::Sine,
        "samplehold" | "sample_hold" | "sample-hold" | "s&h" | "sh" => LFOWaveform::SampleHold,
        _ => LFOWaveform::Triangle,
    }
}

fn convert_operator(json_op: &JsonOperator, top_feedback: f32, is_op6: bool) -> PresetOperator {
    // DX7 coarse=0 maps to a 0.5× ratio; everything else is the literal multiplier.
    let frequency_ratio = if json_op.frequency == 0.0 {
        0.5
    } else {
        json_op.frequency
    };

    // Per-operator feedback overrides the top-level value (which by DX7 convention
    // applies only to the last operator in the algorithm — usually op 6).
    let feedback = if json_op.feedback > 0.0 {
        json_op.feedback
    } else if is_op6 {
        top_feedback
    } else {
        0.0
    };

    let fixed_frequency = json_op.oscillator_mode.eq_ignore_ascii_case("fixed");
    // DX7 fixed-mode frequency: f = 10^coarse * (1 + fine/100). Coarse 0..3 maps to
    // 1Hz/10Hz/100Hz/1000Hz, Fine adds up to ~99% on top.
    let fixed_freq_hz = if fixed_frequency {
        let coarse = json_op.fixed_frequency_coarse.clamp(0.0, 3.0);
        let fine = json_op.fixed_frequency_fine.clamp(0.0, 99.0);
        10f32.powf(coarse) * (1.0 + fine / 100.0)
    } else {
        440.0
    };

    let (left_curve, right_curve, left_depth, right_depth, breakpoint) =
        if let Some(kls) = &json_op.keyboard_level_scaling {
            (
                KeyScaleCurve::from_str(&kls.left_curve),
                KeyScaleCurve::from_str(&kls.right_curve),
                kls.left_depth.clamp(0.0, 99.0),
                kls.right_depth.clamp(0.0, 99.0),
                parse_breakpoint(&kls.breakpoint),
            )
        } else {
            (
                KeyScaleCurve::default(),
                KeyScaleCurve::default(),
                0.0,
                0.0,
                60,
            )
        };

    PresetOperator {
        frequency_ratio,
        output_level: json_op.output_level,
        detune: json_op.detune,
        feedback,
        velocity_sensitivity: json_op.key_velocity_sensitivity.min(7) as f32,
        key_scale_rate: json_op.keyboard_rate_scaling.min(7) as f32,
        key_scale_breakpoint: breakpoint,
        key_scale_left_curve: left_curve,
        key_scale_right_curve: right_curve,
        key_scale_left_depth: left_depth,
        key_scale_right_depth: right_depth,
        am_sensitivity: json_op.am_sensitivity.min(3),
        oscillator_key_sync: true, // applied at patch-level below
        fixed_frequency,
        fixed_freq_hz,
        envelope: (
            json_op.eg.rate1,
            json_op.eg.rate2,
            json_op.eg.rate3,
            json_op.eg.rate4,
            json_op.eg.level1,
            json_op.eg.level2,
            json_op.eg.level3,
            json_op.eg.level4,
        ),
    }
}

fn load_json_file(path: &Path, collection: &str) -> Option<Dx7Preset> {
    let content = std::fs::read_to_string(path).ok()?;
    let patch: JsonPatch = serde_json::from_str(&content)
        .map_err(|e| log::warn!("Failed to parse {:?}: {}", path, e))
        .ok()?;

    if patch.operators.len() != 6 || patch.name.trim().is_empty() {
        return None;
    }

    let osc_key_sync = !patch.oscillator_key_sync.eq_ignore_ascii_case("off");
    let mut operators: [PresetOperator; 6] = std::array::from_fn(|_| PresetOperator::default());
    for (i, op) in patch.operators.iter().enumerate() {
        let mut converted = convert_operator(op, patch.feedback, i == 5);
        converted.oscillator_key_sync = osc_key_sync;
        operators[i] = converted;
    }

    let lfo = patch.lfo.as_ref().map(|l| PresetLfo {
        waveform: parse_lfo_wave(&l.wave),
        rate: l.speed,
        delay: l.delay,
        pitch_mod_depth: l.pitch_mod_depth,
        amp_mod_depth: l.am_depth,
        key_sync: l.sync.eq_ignore_ascii_case("on"),
    });

    let pms = patch
        .lfo
        .as_ref()
        .map(|l| l.pitch_mod_sensitivity)
        .unwrap_or(0)
        .min(7);

    let pitch_eg = patch.pitch_eg.as_ref().map(|p| PresetPitchEg {
        rate1: p.rate1,
        rate2: p.rate2,
        rate3: p.rate3,
        rate4: p.rate4,
        level1: p.level1,
        level2: p.level2,
        level3: p.level3,
        level4: p.level4,
    });

    Some(Dx7Preset {
        name: patch.name.trim().to_string(),
        collection: collection.to_string(),
        algorithm: patch.algorithm,
        operators,
        master_tune: None,
        pitch_bend_range: None,
        portamento_enable: None,
        portamento_time: None,
        mono_mode: None,
        transpose_semitones: parse_transpose(&patch.transpose),
        pitch_mod_sensitivity: pms,
        pitch_eg,
        lfo,
    })
}

/// Scan `base_dir` for collection subdirectories and load every `.json` file inside.
/// Collections are loaded in alphabetical order; files within each collection are also sorted.
pub fn scan_patches_dir(base_dir: &Path) -> Vec<Dx7Preset> {
    let mut presets = Vec::new();

    let Ok(dir_entries) = std::fs::read_dir(base_dir) else {
        return presets;
    };

    let mut subdirs: Vec<_> = dir_entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();

    subdirs.sort_by_key(|e| e.file_name());

    for subdir in subdirs {
        let collection_name = subdir.file_name().to_string_lossy().to_string();

        let Ok(files) = std::fs::read_dir(subdir.path()) else {
            continue;
        };

        let mut json_files: Vec<_> = files
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
            .collect();

        json_files.sort_by_key(|e| e.file_name());

        for file in json_files {
            if let Some(preset) = load_json_file(&file.path(), &collection_name) {
                presets.push(preset);
            }
        }
    }

    log::info!("Loaded {} presets from {:?}", presets.len(), base_dir);
    presets
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_note_names() {
        assert_eq!(parse_note_name("C3"), Some(60));
        assert_eq!(parse_note_name("C2"), Some(48));
        assert_eq!(parse_note_name("A-1"), Some(21));
        assert_eq!(parse_note_name("F#4"), Some(78));
        assert_eq!(parse_note_name("Bb2"), Some(58));
        assert!(parse_note_name("foo").is_none());
    }

    #[test]
    fn parse_transpose_handles_strings_and_integers() {
        assert_eq!(parse_transpose(&serde_json::json!("C3")), 0);
        assert_eq!(parse_transpose(&serde_json::json!("C2")), -12);
        assert_eq!(parse_transpose(&serde_json::json!("C4")), 12);
        assert_eq!(parse_transpose(&serde_json::json!(-7)), -7);
        assert_eq!(parse_transpose(&serde_json::json!(50)), 24); // clamped
    }

    fn write_temp_patch(dir: &std::path::Path, name: &str, content: &str) {
        std::fs::write(dir.join(name), content).expect("write");
    }

    #[test]
    fn parse_note_name_handles_negative_octaves() {
        // Convention: C3 = MIDI 60, so the formula `(octave + 2) * 12 + semitone`
        // gives C-1 = 12, C0 = 24, C2 = 48, C3 = 60.
        assert_eq!(parse_note_name("A-1"), Some(21));
        assert_eq!(parse_note_name("C-1"), Some(12));
        assert_eq!(parse_note_name("C0"), Some(24));
    }

    #[test]
    fn parse_note_name_rejects_empty_and_invalid() {
        assert!(parse_note_name("").is_none());
        assert!(parse_note_name("   ").is_none());
        assert!(parse_note_name("X3").is_none());
        assert!(parse_note_name("Czz").is_none());
    }

    #[test]
    fn parse_breakpoint_handles_number_string_and_other() {
        assert_eq!(parse_breakpoint(&serde_json::json!(72)), 72);
        assert_eq!(parse_breakpoint(&serde_json::json!("C3")), 60);
        // Boolean falls through to default (60)
        assert_eq!(parse_breakpoint(&serde_json::json!(true)), 60);
    }

    #[test]
    fn parse_breakpoint_clamps_to_midi_range() {
        assert_eq!(parse_breakpoint(&serde_json::json!(200)), 127);
    }

    #[test]
    fn parse_lfo_wave_recognises_all_aliases() {
        assert_eq!(parse_lfo_wave("triangle"), LFOWaveform::Triangle);
        assert_eq!(parse_lfo_wave("TRI"), LFOWaveform::Triangle);
        assert_eq!(parse_lfo_wave("sawdown"), LFOWaveform::SawDown);
        assert_eq!(parse_lfo_wave("saw_down"), LFOWaveform::SawDown);
        assert_eq!(parse_lfo_wave("saw-up"), LFOWaveform::SawUp);
        assert_eq!(parse_lfo_wave("Square"), LFOWaveform::Square);
        assert_eq!(parse_lfo_wave("sin"), LFOWaveform::Sine);
        assert_eq!(parse_lfo_wave("S&H"), LFOWaveform::SampleHold);
        assert_eq!(parse_lfo_wave("garbage"), LFOWaveform::Triangle); // default
    }

    #[test]
    fn deserialize_lenient_f32_accepts_string_or_number() {
        // The helper is tested indirectly via JSON roundtrip.
        let json = r#"{
            "wave": "triangle",
            "speed": 35,
            "delay": 0,
            "pitchModDepth": 5,
            "amDepth": "0",
            "sync": "off",
            "pitchModSensitivity": 0
        }"#;
        let lfo: JsonLfo = serde_json::from_str(json).expect("parse lfo");
        assert_eq!(lfo.am_depth, 0.0);
        assert_eq!(lfo.speed, 35.0);
    }

    #[test]
    fn convert_operator_uses_top_feedback_only_for_op6() {
        let json_op = JsonOperator {
            frequency: 1.0,
            output_level: 99.0,
            am_sensitivity: 0,
            ..Default::default()
        };
        let op_a = convert_operator(&json_op, 7.0, false);
        assert_eq!(op_a.feedback, 0.0); // not op 6 → top feedback ignored
        let op_b = convert_operator(&json_op, 7.0, true);
        assert_eq!(op_b.feedback, 7.0); // op 6 → top feedback applied
    }

    #[test]
    fn convert_operator_zero_frequency_maps_to_half_ratio() {
        let json_op = JsonOperator {
            frequency: 0.0,
            output_level: 99.0,
            ..Default::default()
        };
        let op = convert_operator(&json_op, 0.0, false);
        assert_eq!(op.frequency_ratio, 0.5);
    }

    #[test]
    fn convert_operator_per_op_feedback_overrides_top() {
        let json_op = JsonOperator {
            frequency: 2.0,
            feedback: 4.0,
            ..Default::default()
        };
        let op = convert_operator(&json_op, 7.0, true);
        assert_eq!(op.feedback, 4.0);
    }

    #[test]
    fn convert_operator_fixed_frequency_uses_coarse_fine() {
        let json_op = JsonOperator {
            oscillator_mode: "fixed".to_string(),
            fixed_frequency_coarse: 2.0,
            fixed_frequency_fine: 50.0,
            ..Default::default()
        };
        let op = convert_operator(&json_op, 0.0, false);
        assert!(op.fixed_frequency);
        // 10^2 * (1 + 50/100) = 150
        assert!((op.fixed_freq_hz - 150.0).abs() < 0.1);
    }

    #[test]
    fn scan_patches_dir_returns_sorted_results_for_real_subdirs() {
        let path = std::path::Path::new("patches");
        if !path.exists() {
            eprintln!("Skipping: no patches directory");
            return;
        }
        let presets = scan_patches_dir(path);
        // We expect at least one preset to load successfully.
        assert!(!presets.is_empty());
        // Collections sort alphabetically.
        if presets.len() >= 2 {
            assert!(presets[0].collection <= presets[1].collection);
        }
    }

    #[test]
    fn scan_patches_dir_handles_missing_directory_gracefully() {
        let presets = scan_patches_dir(std::path::Path::new("/nonexistent_path_xyz"));
        assert!(presets.is_empty());
    }

    #[test]
    fn load_json_file_returns_none_for_missing_operators() {
        let dir = std::env::temp_dir().join(format!("synth-fm-rs-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("mkdir");
        let json = r#"{"name": "BAD", "algorithm": 1, "operators": []}"#;
        write_temp_patch(&dir, "bad.json", json);
        let result = load_json_file(&dir.join("bad.json"), "test");
        assert!(result.is_none());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_json_file_returns_none_for_invalid_json() {
        let dir = std::env::temp_dir().join(format!("synth-fm-rs-test-bad-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("mkdir");
        write_temp_patch(&dir, "bad.json", "{not valid json");
        let result = load_json_file(&dir.join("bad.json"), "test");
        assert!(result.is_none());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_json_file_supports_oscillator_key_sync_off() {
        let dir =
            std::env::temp_dir().join(format!("synth-fm-rs-test-osc-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("mkdir");
        let json = r#"{
            "name": "TEST",
            "algorithm": 5,
            "feedback": 7,
            "oscillatorKeySync": "off",
            "operators": [
                {"frequency": 1.0, "outputLevel": 99},
                {"frequency": 1.0, "outputLevel": 99},
                {"frequency": 1.0, "outputLevel": 99},
                {"frequency": 1.0, "outputLevel": 99},
                {"frequency": 1.0, "outputLevel": 99},
                {"frequency": 1.0, "outputLevel": 99}
            ]
        }"#;
        write_temp_patch(&dir, "good.json", json);
        let preset = load_json_file(&dir.join("good.json"), "test").expect("parse");
        assert_eq!(preset.name, "TEST");
        assert!(!preset.operators[0].oscillator_key_sync);
        assert_eq!(preset.operators[5].feedback, 7.0);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_json_file_with_keyboard_level_scaling_block() {
        let dir = std::env::temp_dir().join(format!("synth-fm-rs-test-kls-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("mkdir");
        let json = r#"{
            "name": "KLS",
            "algorithm": 5,
            "operators": [
                {"frequency": 1.0, "outputLevel": 99,
                 "keyboardLevelScaling": {
                     "breakpoint": "C4",
                     "leftCurve": "-exp",
                     "rightCurve": "+lin",
                     "leftDepth": 50,
                     "rightDepth": 50
                 }},
                {"frequency": 1.0, "outputLevel": 99},
                {"frequency": 1.0, "outputLevel": 99},
                {"frequency": 1.0, "outputLevel": 99},
                {"frequency": 1.0, "outputLevel": 99},
                {"frequency": 1.0, "outputLevel": 99}
            ]
        }"#;
        write_temp_patch(&dir, "kls.json", json);
        let preset = load_json_file(&dir.join("kls.json"), "test").expect("parse");
        assert_eq!(preset.operators[0].key_scale_breakpoint, 72);
        assert_eq!(preset.operators[0].key_scale_left_depth, 50.0);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn parse_brasshorns_patch_full_fidelity() {
        let path = std::path::Path::new("patches/mark/brasshorns.json");
        if !path.exists() {
            eprintln!("Skipping: {:?} not present", path);
            return;
        }
        let preset = load_json_file(path, "mark").expect("brasshorns.json must parse");

        // Header / structural data
        assert_eq!(preset.name, "BRASSHORNS");
        assert_eq!(preset.algorithm, 18);
        assert_eq!(preset.transpose_semitones, 0); // "C3" → 0

        // LFO
        let lfo = preset.lfo.as_ref().expect("lfo should be present");
        assert_eq!(lfo.rate, 35.0);
        assert_eq!(lfo.delay, 0.0);
        assert_eq!(lfo.pitch_mod_depth, 5.0);
        assert_eq!(lfo.amp_mod_depth, 0.0); // string "0" → 0
        assert!(!lfo.key_sync); // "off"
        assert_eq!(preset.pitch_mod_sensitivity, 1);

        // Pitch EG
        let peg = preset
            .pitch_eg
            .as_ref()
            .expect("pitch eg should be present");
        assert_eq!(peg.rate1, 94.0);
        assert_eq!(peg.level1, 53.0);
        assert!(peg.is_active()); // level1=53 ≠ 50

        // Op 5 (index 4): freq=3, detune=-1, kvs=1, krs=0
        let op5 = &preset.operators[4];
        assert_eq!(op5.frequency_ratio, 3.0);
        assert_eq!(op5.detune, -1.0);
        assert_eq!(op5.velocity_sensitivity, 1.0);
        assert_eq!(op5.key_scale_rate, 0.0);

        // Op 6 (index 5) is the feedback op: top-level feedback=7 must propagate
        // since the per-op feedback is absent.
        let op6 = &preset.operators[5];
        assert_eq!(op6.frequency_ratio, 8.0);
        assert_eq!(op6.feedback, 7.0);
        assert_eq!(op6.key_scale_rate, 7.0);
    }
}
