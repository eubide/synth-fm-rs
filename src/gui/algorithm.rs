use super::Dx7App;
use crate::algorithms;
use eframe::egui;

impl Dx7App {
    pub(super) fn draw_algorithm_diagram_compact(&mut self, ui: &mut egui::Ui) {
        let current_alg = self.snapshot.algorithm;
        let alg_info = algorithms::get_algorithm_info(current_alg);
        let enabled_states = [
            self.snapshot.operators[0].enabled,
            self.snapshot.operators[1].enabled,
            self.snapshot.operators[2].enabled,
            self.snapshot.operators[3].enabled,
            self.snapshot.operators[4].enabled,
            self.snapshot.operators[5].enabled,
        ];

        let carrier_color = egui::Color32::from_rgb(70, 130, 180);
        let modulator_color = egui::Color32::from_rgb(100, 160, 100);
        let feedback_color = egui::Color32::from_rgb(200, 100, 50);

        // Constrain the panel so the diagram column doesn't fill the whole
        // half-screen. Leaves the operator panel on the right more room.
        let panel_width = ui.available_width().min(340.0);

        ui.allocate_ui(egui::vec2(panel_width, 0.0), |ui| {
            ui.group(|ui| {
                ui.vertical(|ui| {
                    // Compact header with algorithm selector
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("ALG").strong());
                        if ui.small_button("<").clicked() && current_alg > 1 {
                            if let Ok(mut ctrl) = self.lock_controller() {
                                ctrl.set_algorithm(current_alg - 1);
                            }
                        }
                        ui.label(egui::RichText::new(format!("{:02}", current_alg)).strong());
                        if ui.small_button(">").clicked() && current_alg < 32 {
                            if let Ok(mut ctrl) = self.lock_controller() {
                                ctrl.set_algorithm(current_alg + 1);
                            }
                        }
                        ui.label(
                            egui::RichText::new(algorithms::get_algorithm_name(current_alg))
                                .size(11.0),
                        );
                    });

                    let (response, painter) = ui.allocate_painter(
                        egui::vec2(ui.available_width(), 130.0),
                        egui::Sense::hover(),
                    );
                    let rect = response.rect;

                    // Reserve a strip at the bottom of the canvas for the OUTPUT
                    // bus + label so the carrier row never overlaps it.
                    let bus_strip = 26.0;
                    let layout_rect = egui::Rect::from_min_max(
                        rect.min,
                        egui::pos2(rect.max.x, rect.max.y - bus_strip),
                    );
                    let positions =
                        self.calculate_operator_positions_compact(&alg_info, layout_rect);

                    // Modulation connections
                    let connection_color = egui::Color32::from_rgb(100, 100, 100);
                    for (from, to) in &alg_info.connections {
                        let from_pos = positions[(*from - 1) as usize];
                        let to_pos = positions[(*to - 1) as usize];
                        painter.line_segment(
                            [from_pos, to_pos],
                            egui::Stroke::new(1.5_f32, connection_color),
                        );
                    }

                    // Feedback loop indicator
                    if alg_info.feedback_op > 0 {
                        let fb_pos = positions[(alg_info.feedback_op - 1) as usize];
                        let loop_center = fb_pos + egui::vec2(14.0, -8.0);
                        painter.circle_stroke(
                            loop_center,
                            6.0,
                            egui::Stroke::new(1.5_f32, feedback_color),
                        );
                    }

                    // Operators
                    let op_radius = 11.0;
                    for (i, &pos) in positions.iter().enumerate() {
                        let op_num = (i + 1) as u8;
                        let is_carrier = alg_info.carriers.contains(&op_num);
                        let is_selected = self.selected_operator == i;
                        let is_enabled = enabled_states[i];
                        let activity = self.snapshot.operators[i].live_level.clamp(0.0, 1.0);

                        let (base_fill, stroke_color, text_color) = if !is_enabled {
                            (
                                egui::Color32::from_rgb(80, 80, 80),
                                egui::Color32::from_rgb(60, 60, 60),
                                egui::Color32::from_rgb(120, 120, 120),
                            )
                        } else if is_carrier {
                            (
                                carrier_color,
                                if is_selected {
                                    egui::Color32::from_rgb(255, 200, 0)
                                } else {
                                    egui::Color32::from_rgb(50, 100, 150)
                                },
                                egui::Color32::WHITE,
                            )
                        } else {
                            (
                                modulator_color,
                                if is_selected {
                                    egui::Color32::from_rgb(255, 200, 0)
                                } else {
                                    egui::Color32::from_rgb(70, 130, 70)
                                },
                                egui::Color32::WHITE,
                            )
                        };

                        let fill_color = if is_enabled {
                            base_fill.lerp_to_gamma(
                                egui::Color32::WHITE,
                                activity * ACTIVITY_BRIGHTEN_MAX,
                            )
                        } else {
                            base_fill
                        };

                        painter.circle(
                            pos,
                            op_radius,
                            fill_color,
                            egui::Stroke::new(
                                if is_selected { 2.5_f32 } else { 1.5 },
                                stroke_color,
                            ),
                        );
                        painter.text(
                            pos,
                            egui::Align2::CENTER_CENTER,
                            format!("{}", op_num),
                            egui::FontId::proportional(10.0),
                            text_color,
                        );
                    }

                    // OUTPUT bus: horizontal blue bar with verticals from each
                    // carrier and an OUTPUT label centered just below.
                    let bus_y = rect.bottom() - 16.0;
                    let mut carrier_xs: Vec<f32> = alg_info
                        .carriers
                        .iter()
                        .map(|&c| positions[(c - 1) as usize].x)
                        .collect();
                    if !carrier_xs.is_empty() {
                        carrier_xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        let bar_left = carrier_xs.first().copied().unwrap() - 8.0;
                        let bar_right = carrier_xs.last().copied().unwrap() + 8.0;
                        painter.line_segment(
                            [egui::pos2(bar_left, bus_y), egui::pos2(bar_right, bus_y)],
                            egui::Stroke::new(2.0_f32, carrier_color),
                        );
                        for &carrier in &alg_info.carriers {
                            let carrier_pos = positions[(carrier - 1) as usize];
                            painter.line_segment(
                                [
                                    egui::pos2(carrier_pos.x, carrier_pos.y + op_radius),
                                    egui::pos2(carrier_pos.x, bus_y),
                                ],
                                egui::Stroke::new(1.5_f32, carrier_color),
                            );
                        }
                        painter.text(
                            egui::pos2((bar_left + bar_right) * 0.5, bus_y + 3.0),
                            egui::Align2::CENTER_TOP,
                            "OUTPUT",
                            egui::FontId::proportional(9.0),
                            carrier_color,
                        );
                    }

                    // Color legend
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("●").color(carrier_color).size(12.0));
                        ui.label(egui::RichText::new("Carrier").size(10.0));
                        ui.add_space(6.0);
                        ui.label(egui::RichText::new("●").color(modulator_color).size(12.0));
                        ui.label(egui::RichText::new("Modulator").size(10.0));
                        if alg_info.feedback_op > 0 {
                            ui.add_space(6.0);
                            ui.label(egui::RichText::new("↻").color(feedback_color).size(12.0));
                            ui.label(egui::RichText::new("Feedback").size(10.0));
                        }
                    });
                });
            });
        });
    }

    /// Lay out the 6 operators as a Dexed-style algorithm diagram: each
    /// independent modulation chain becomes its own vertical column, with
    /// carriers at the bottom and modulators stacked directly above their
    /// target(s). Branching siblings spread left/right around the target;
    /// an operator that modulates several targets sits at their centroid.
    pub(super) fn calculate_operator_positions_compact(
        &self,
        alg_info: &algorithms::AlgorithmInfo,
        rect: egui::Rect,
    ) -> [egui::Pos2; 6] {
        // 1. Layer = depth from carriers (carriers at 0, modulators at 1..).
        let mut layer = [0i32; 6];
        for _ in 0..5 {
            for &(from, to) in &alg_info.connections {
                let candidate = layer[(to - 1) as usize] + 1;
                if candidate > layer[(from - 1) as usize] {
                    layer[(from - 1) as usize] = candidate;
                }
            }
        }

        // 2. Stack id = connected component (treating connections as
        //    undirected). Each stack gets its own column on screen.
        let mut stack = [usize::MAX; 6];
        let mut next_id = 0usize;
        for seed in 0..6 {
            if stack[seed] != usize::MAX {
                continue;
            }
            stack[seed] = next_id;
            let mut frontier = vec![seed];
            while let Some(cur) = frontier.pop() {
                let cur_op = (cur + 1) as u8;
                for &(from, to) in &alg_info.connections {
                    let neigh = if from == cur_op {
                        Some((to - 1) as usize)
                    } else if to == cur_op {
                        Some((from - 1) as usize)
                    } else {
                        None
                    };
                    if let Some(n) = neigh {
                        if stack[n] == usize::MAX {
                            stack[n] = next_id;
                            frontier.push(n);
                        }
                    }
                }
            }
            next_id += 1;
        }
        let n_stacks = next_id.max(1);

        // 3. Geometry: horizontal slot per stack, vertical slot per layer.
        let canvas_left = rect.left() + 20.0;
        let canvas_right = rect.right() - 20.0;
        let stack_width = (canvas_right - canvas_left) / n_stacks as f32;
        let max_layer = *layer.iter().max().unwrap_or(&0) as f32;
        let layer_height = rect.height() / (max_layer + 2.0);
        let row_y = |l: i32| rect.bottom() - layer_height * (l as f32 + 1.0);

        let mut pos = [egui::Pos2::ZERO; 6];

        // 4. Carriers: spread evenly across their stack's column at row 0.
        let mut carriers_per_stack: Vec<Vec<u8>> = vec![Vec::new(); n_stacks];
        for &c in &alg_info.carriers {
            carriers_per_stack[stack[(c - 1) as usize]].push(c);
        }
        for (s, carriers) in carriers_per_stack.iter().enumerate() {
            let left = canvas_left + s as f32 * stack_width;
            let n = carriers.len() as f32;
            for (i, &c) in carriers.iter().enumerate() {
                let x = left + stack_width * (i as f32 + 1.0) / (n + 1.0);
                pos[(c - 1) as usize] = egui::pos2(x, row_y(0));
            }
        }

        // 5. Modulators row by row above their target(s).
        let max_l = max_layer as i32;
        let sibling_gap = 30.0_f32.min(stack_width * 0.55);
        for l in 1..=max_l {
            // Pass A: ops with multiple targets sit at the centroid.
            for op in 1..=6u8 {
                if layer[(op - 1) as usize] != l {
                    continue;
                }
                let targets: Vec<u8> = alg_info
                    .connections
                    .iter()
                    .filter(|(f, _)| *f == op)
                    .map(|(_, t)| *t)
                    .collect();
                if targets.len() > 1 {
                    let cx = targets
                        .iter()
                        .map(|t| pos[(*t - 1) as usize].x)
                        .sum::<f32>()
                        / targets.len() as f32;
                    pos[(op - 1) as usize] = egui::pos2(cx, row_y(l));
                }
            }
            // Pass B: single-target ops grouped by target, spread as siblings.
            let mut groups: Vec<(u8, Vec<u8>)> = Vec::new();
            for op in 1..=6u8 {
                if layer[(op - 1) as usize] != l {
                    continue;
                }
                let mut targets = alg_info
                    .connections
                    .iter()
                    .filter(|(f, _)| *f == op)
                    .map(|(_, t)| *t);
                let first = targets.next();
                let only_one = first.is_some() && targets.next().is_none();
                if let (Some(target), true) = (first, only_one) {
                    if let Some(g) = groups.iter_mut().find(|(t, _)| *t == target) {
                        g.1.push(op);
                    } else {
                        groups.push((target, vec![op]));
                    }
                }
            }
            for (target, sibs) in groups {
                let tx = pos[(target - 1) as usize].x;
                let n = sibs.len() as f32;
                for (i, op) in sibs.iter().enumerate() {
                    let offset = (i as f32 - (n - 1.0) / 2.0) * sibling_gap;
                    let x = (tx + offset).clamp(canvas_left + 5.0, canvas_right - 5.0);
                    pos[(*op - 1) as usize] = egui::pos2(x, row_y(l));
                }
            }
        }

        pos
    }
}

/// Max fraction of white blended into an active operator's fill (0..=1).
/// Tunable: lower = subtler highlight, higher = whiter at full envelope.
pub(super) const ACTIVITY_BRIGHTEN_MAX: f32 = 0.6;
