use super::{Dx7App, USER_COLLECTION};
use crate::presets::Dx7Preset;
use eframe::egui;

impl Dx7App {
    pub(super) fn draw_preset_selector(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(4.0, 3.0);

            // --- Current voice header ---
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("current:").size(11.0).strong());
                // DX7 voice names are 10 characters (SysEx VCED name field).
                ui.add(
                    egui::TextEdit::singleline(&mut self.voice_name)
                        .char_limit(10)
                        .desired_width(100.0),
                );
                if self.snapshot.voice_edited {
                    ui.colored_label(egui::Color32::from_rgb(220, 140, 40), "* edited");
                }
                if let Some(p) = self.presets.get(self.selected_preset) {
                    ui.label(
                        egui::RichText::new(format!("[{}]", p.collection))
                            .size(10.0)
                            .color(egui::Color32::from_gray(140)),
                    );
                }
                if ui
                    .button("SAVE")
                    .on_hover_text(format!(
                        "Save to {}",
                        self.user_patches_dir.to_string_lossy()
                    ))
                    .clicked()
                {
                    self.save_voice();
                }
            });
            if !self.voice_status.is_empty() {
                ui.label(
                    egui::RichText::new(&self.voice_status)
                        .size(11.0)
                        .color(egui::Color32::from_rgb(120, 120, 120)),
                );
            }
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
                .max_height(300.0)
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
                            self.select_preset(global_idx);
                        }
                    }
                });
        });
    }

    /// The current voice minus global settings (tune, bend range, portamento),
    /// which are not part of a stored voice.
    fn edit_buffer(&self) -> Dx7Preset {
        let mut preset = Dx7Preset::from_snapshot(&self.snapshot);
        preset.master_tune = None;
        preset.pitch_bend_range = None;
        preset.portamento_enable = None;
        preset.portamento_time = None;
        preset
    }

    /// The browser entry the current voice was loaded from, if it still is.
    pub(super) fn compare_original(&self) -> Option<&Dx7Preset> {
        self.presets
            .get(self.selected_preset)
            .filter(|p| p.name == self.snapshot.preset_name)
    }

    pub(super) fn toggle_compare(&mut self) {
        if let Some(edit) = self.compare_edit.take() {
            if let Ok(mut ctrl) = self.lock_controller() {
                ctrl.restore_edit_buffer(edit);
            }
            self.display_text = "EDIT".to_string();
            return;
        }
        if !self.snapshot.voice_edited {
            return;
        }
        let Some(original) = self.compare_original().cloned() else {
            return;
        };
        self.compare_edit = Some(self.edit_buffer());
        if let Ok(mut ctrl) = self.lock_controller() {
            ctrl.load_voice(original);
        }
        self.display_text = "COMPARE".to_string();
    }

    pub(super) fn select_preset(&mut self, index: usize) {
        self.compare_edit = None;
        let preset = self.presets[index].clone();
        self.selected_preset = index;
        self.display_text = format!("LOADED: {}", preset.name);
        if let Ok(mut ctrl) = self.lock_controller() {
            ctrl.load_voice(preset);
        }
    }

    pub(super) fn save_voice(&mut self) {
        let mut preset = self
            .compare_edit
            .take()
            .unwrap_or_else(|| self.edit_buffer());
        preset.name = self.voice_name.trim().to_string();
        preset.collection = USER_COLLECTION.to_string();

        let path = match crate::preset_loader::save_json_file(&preset, &self.user_patches_dir) {
            Ok(path) => path,
            Err(e) => {
                self.voice_status = format!("Save failed: {}", e);
                return;
            }
        };

        let stem = crate::preset_loader::file_stem_for(&preset.name);
        let same_voice = |p: &Dx7Preset| {
            p.collection == USER_COLLECTION && crate::preset_loader::file_stem_for(&p.name) == stem
        };
        let index = match self.presets.iter().position(same_voice) {
            Some(i) => {
                self.presets[i] = preset.clone();
                i
            }
            None => {
                let after_last_user = self
                    .presets
                    .iter()
                    .rposition(|p| p.collection == USER_COLLECTION)
                    .map_or(self.presets.len(), |i| i + 1);
                self.presets.insert(after_last_user, preset.clone());
                after_last_user
            }
        };
        self.selected_preset = index;
        if let Ok(mut ctrl) = self.lock_controller() {
            ctrl.load_voice(preset);
        }
        self.voice_status = format!("Saved to {}", path.to_string_lossy());
    }

    /// Swaps in `presets` as the browser's `collection` (dropping any earlier
    /// load of the same name) and selects its first voice.
    pub(super) fn replace_collection(&mut self, collection: &str, presets: Vec<Dx7Preset>) {
        self.compare_edit = None;
        self.presets.retain(|p| p.collection != collection);
        self.selected_preset = self.presets.len();
        self.presets.extend(presets);
        self.selected_collection = Some(collection.to_string());
    }
}
