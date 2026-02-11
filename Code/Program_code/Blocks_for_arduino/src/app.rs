use eframe::egui;
use crate::model::*;
use crate::helper::*;
use std::collections::HashMap;

const GLOBAL_X: f32 = 140.0;
const GLOBAL_Y: f32 = 90.0;

#[derive(Default)]
pub struct BlocksForArduino {
    pub sections: Vec<BlockSection>,
    pub was_mouse_down: bool,
}

/* ---------- APP LOGIC ---------- */

impl BlocksForArduino {
    pub fn load_block_json(&mut self, path: &str) {
        let raw = match std::fs::read_to_string(path) {
            Ok(r) => r,
            Err(e) => {
                println!("Failed to read {}: {}", path, e);
                return;
            }
        };

        let file: BlockFile = match serde_json::from_str(&raw) {
            Ok(f) => f,
            Err(e) => {
                println!("Invalid JSON in {}: {}", path, e);
                return;
            }
        };

        let mut block = match file.block.sections.into_iter().next() {
            Some(b) => b,
            None => return,
        };

        block.pos = egui::pos2(
            20.0,
            60.0 + self.sections.len() as f32 * 80.0,
        );

        block.attached_to = None;
        block.children.clear();

        /* ---------- BUILD INPUT STORAGE ---------- */

        let mut values = HashMap::new();
        for input in &block.inputs {
            values.insert(input.name.clone(), String::new());
        }

        block.input_values = values;

        self.sections.push(block);
    }

    pub fn spawn_code_block(&mut self, source: usize) {
        let mut new_block = self.sections[source].clone();

        new_block.pos = egui::pos2(
            320.0,
            60.0 + self.sections.len() as f32 * 80.0,
        );

        new_block.attached_to = None;
        new_block.children.clear();

        self.sections.push(new_block);
    }

    fn try_snap(&mut self, idx: usize) {
        let size = egui::vec2(GLOBAL_X, GLOBAL_Y);
        let snap = 12.0;
        let my_pos = self.sections[idx].pos;

        for j in 0..self.sections.len() {
            if j == idx {
                continue;
            }

            let parent = &self.sections[j];

            let offset = parent
                .child_offset
                .map(|o| o.vec2())
                .unwrap_or(egui::vec2(0.0, 0.0));

            let target_x = parent.pos.x + offset.x;
            let target_y = parent.pos.y + size.y + offset.y;

            if (my_pos.y - target_y).abs() < snap
                && (my_pos.x - target_x).abs() < snap
            {
                self.sections[idx].pos = egui::pos2(target_x, target_y);
                self.sections[idx].attached_to = Some(j);
                self.sections[j].children.push(idx);
                break;
            }
        }
    }

    fn move_children(&mut self, parent: usize) {
        let size = egui::vec2(GLOBAL_X, GLOBAL_Y);
        let base = self.sections[parent].pos;

        let offset = self.sections[parent]
            .child_offset
            .map(|o| o.vec2())
            .unwrap_or(egui::vec2(0.0, 0.0));

        let mut y = base.y + size.y + offset.y;
        let children = self.sections[parent].children.clone();

        for child in children {
            self.sections[child].pos =
                egui::pos2(base.x + offset.x, y);

            y += size.y;
            self.move_children(child);
        }
    }

    fn collect_descendants(&self, idx: usize, out: &mut Vec<usize>) {
        out.push(idx);
        for &child in &self.sections[idx].children {
            self.collect_descendants(child, out);
        }
    }

    fn delete_block(&mut self, idx: usize) {
        if let Some(parent) = self.sections[idx].attached_to {
            self.sections[parent].children.retain(|&c| c != idx);
        }

        let mut to_delete = Vec::new();
        self.collect_descendants(idx, &mut to_delete);

        to_delete.sort_unstable_by(|a, b| b.cmp(a));

        for i in to_delete {
            self.sections.remove(i);
        }
    }
}

/* ---------- UI ---------- */

impl eframe::App for BlocksForArduino {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mouse_down = ctx.input(|i| i.pointer.primary_down());
        let mouse_released = self.was_mouse_down && !mouse_down;
        self.was_mouse_down = mouse_down;

        let mut delete_request: Option<usize> = None;

        egui::CentralPanel::default().show(ctx, |ui| {
            let size = egui::vec2(GLOBAL_X, GLOBAL_Y);

            {
                let painter = ui.painter();
                let screen = ui.max_rect();

                painter.text(
                    egui::pos2(10.0, 10.0),
                    egui::Align2::LEFT_TOP,
                    "Block section",
                    egui::TextStyle::Heading.resolve(ui.style()),
                    ui.style().visuals.text_color(),
                );

                painter.text(
                    egui::pos2(320.0, 10.0),
                    egui::Align2::LEFT_TOP,
                    "Code section",
                    egui::TextStyle::Heading.resolve(ui.style()),
                    ui.style().visuals.text_color(),
                );

                painter.line_segment(
                    [
                        egui::pos2(300.0, screen.top()),
                        egui::pos2(300.0, screen.bottom()),
                    ],
                    egui::Stroke::new(
                        1.0,
                        ui.style().visuals.widgets.noninteractive.bg_stroke.color,
                    ),
                );
            }

            for i in 0..self.sections.len() {
                let is_palette = self.sections[i].pos.x < 300.0;

                let rect = egui::Rect::from_min_size(
                    self.sections[i].pos,
                    size,
                );

                let response =
                    ui.allocate_rect(rect, egui::Sense::click_and_drag());

                if response.clicked() && is_palette {
                    self.spawn_code_block(i);
                }

                if response.dragged() && !is_palette {
                    let delta = response.drag_delta();
                    self.sections[i].pos += delta;
                    self.move_children(i);
                }

                if mouse_released && response.hovered() && !is_palette {
                    self.try_snap(i);
                }

                if response.secondary_clicked() && !is_palette {
                    delete_request = Some(i);
                }

                let painter = ui.painter();

                let color = self.sections[i]
                    .block_colour
                    .as_ref()
                    .map(|c| parse_hex_colour(c))
                    .unwrap_or(egui::Color32::from_rgb(80, 160, 240));

                painter.rect_filled(rect, 6.0, color);

                let label = self.sections[i]
                    .shown_element
                    .clone()
                    .unwrap_or_else(|| self.sections[i].id.clone());

                painter.text(
                    self.sections[i].pos + egui::vec2(10.0, 8.0),
                    egui::Align2::LEFT_TOP,
                    label,
                    egui::TextStyle::Body.resolve(ui.style()),
                    ui.style().visuals.text_color(),
                );

                let block = &mut self.sections[i];
                let mut y_offset = 26.0;

                for (key, value) in block.input_values.iter_mut() {
                    let input_rect = egui::Rect::from_min_size(
                        block.pos + egui::vec2(10.0, y_offset),
                        egui::vec2(100.0, 15.0),
                    );

                    ui.scope_builder(
                        egui::UiBuilder::new().max_rect(input_rect),
                        |ui| {
                            ui.add(
                                egui::TextEdit::singleline(value)
                                    .hint_text(key),
                            );
                        },
                    );

                    y_offset += 22.0;
                }
            }
        });

        if let Some(idx) = delete_request {
            self.delete_block(idx);
        }
    }
}
