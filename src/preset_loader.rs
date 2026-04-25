use crate::presets::Dx7Preset;
use serde::Deserialize;
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
#[serde(rename_all = "camelCase")]
struct JsonOperator {
    #[serde(default)]
    frequency: f32,
    #[serde(default)]
    output_level: f32,
    #[serde(default)]
    detune: f32,
    #[serde(default)]
    feedback: f32,
    #[serde(default)]
    eg: JsonEg,
}

#[derive(Deserialize)]
struct JsonPatch {
    name: String,
    algorithm: u8,
    #[serde(default)]
    feedback: f32,
    #[serde(default)]
    operators: Vec<JsonOperator>,
}

fn load_json_file(path: &Path, collection: &str) -> Option<Dx7Preset> {
    let content = std::fs::read_to_string(path).ok()?;
    let patch: JsonPatch = serde_json::from_str(&content)
        .map_err(|e| log::warn!("Failed to parse {:?}: {}", path, e))
        .ok()?;

    if patch.operators.len() != 6 || patch.name.trim().is_empty() {
        return None;
    }

    let mut operators = [(0.0f32, 0.0f32, 0.0f32, 0.0f32); 6];
    let mut envelopes = [(0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32); 6];

    for (i, op) in patch.operators.iter().enumerate() {
        // DX7 coarse 0 → ratio 0.5; any other value is used directly.
        let ratio = if op.frequency == 0.0 { 0.5 } else { op.frequency };

        // Per-operator feedback (our extension) takes precedence over the top-level
        // feedback which, by DX7 convention, applies only to the last operator (index 5).
        let feedback = if op.feedback > 0.0 {
            op.feedback
        } else if i == 5 {
            patch.feedback
        } else {
            0.0
        };

        operators[i] = (ratio, op.output_level, op.detune, feedback);

        let eg = &op.eg;
        envelopes[i] = (
            eg.rate1, eg.rate2, eg.rate3, eg.rate4,
            eg.level1, eg.level2, eg.level3, eg.level4,
        );
    }

    Some(Dx7Preset {
        name: patch.name.trim().to_string(),
        collection: collection.to_string(),
        algorithm: patch.algorithm,
        operators,
        envelopes,
        master_tune: None,
        mono_mode: None,
        pitch_bend_range: None,
        portamento_enable: None,
        portamento_time: None,
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
