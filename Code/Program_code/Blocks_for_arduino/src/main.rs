use std::fs;

use eframe::egui;
use serde::Deserialize;

fn main() -> eframe::Result<()> {
    let mut files = Vec::new();

    for entry in fs::read_dir("Json_files").unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() {
            files.push(path);
        }
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 650.0])
            .with_title("Blocks for Arduino"),
        ..Default::default()
    };

    eframe::run_native(
        "Blocks for Arduino",
        options,
        Box::new(|_cc| {
            let mut app = BlocksForArduino::default();
            for path in files.iter().map(|p| p.to_str().unwrap()) {
                app.load_block_json(path);
            }
            Ok(Box::new(app))
        }),
    )
}

/* ---------- DATA ---------- */

#[derive(Deserialize, Clone)]
struct BlockFile {
    block: BlockContainer,
}

#[derive(Deserialize, Clone)]
struct BlockContainer {
    sections: Vec<BlockSection>,
}

#[derive(Deserialize, Clone)]
struct BlockSection {
    id: String,

    #[serde(default)]
    Block_colour: Option<String>,

    #[serde(default)]
    Shown_element: Option<String>,

    #[serde(default)]
    child_offset: Option<Offset>,

    #[serde(skip)]
    pos: egui::Pos2,

    #[serde(skip)]
    attached_to: Option<usize>,

    #[serde(skip)]
    children: Vec<usize>,
}

#[derive(Deserialize, Clone, Copy)]
struct Offset {
    x: f32,
    y: f32,
}

impl Offset {
    fn vec2(self) -> egui::Vec2 {
        egui::vec2(self.x, self.y)
    }
}

#[derive(Default)]
struct BlocksForArduino {
    sections: Vec<BlockSection>,
    was_mouse_down: bool,
}

/* ---------- HELPERS ---------- */

fn parse_hex_colour(hex: &str) -> egui::Color32 {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        if let Ok(v) = u32::from_str_radix(hex, 16) {
            return egui::Color32::from_rgb(
                ((v >> 16) & 0xFF) as u8,
                ((v >> 8) & 0xFF) as u8,
                (v & 0xFF) as u8,
            );
        }
    }
    egui::Color32::LIGHT_GRAY
}

/* ---------- APP LOGIC ---------- */

impl BlocksForArduino {
    fn load_block_json(&mut self, path: &str) {
        let raw = std::fs::read_to_string(path).unwrap();
        let file: BlockFile = serde_json::from_str(&raw).unwrap();

        let mut block = file.block.sections.into_iter().next().unwrap();

        block.pos = egui::pos2(
            20.0,
            60.0 + self.sections.len() as f32 * 80.0,
        );

        block.attached_to = None;
        block.children = Vec::new();

        self.sections.push(block);
    }

    fn spawn_code_block(&mut self, source: usize) {
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
        let size = egui::vec2(120.0, 60.0);
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
        let size = egui::vec2(120.0, 60.0);
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

        for block in &mut self.sections {
            if let Some(p) = block.attached_to {
                if p >= idx {
                    block.attached_to = Some(p.saturating_sub(1));
                }
            }

            for c in &mut block.children {
                if *c >= idx {
                    *c = c.saturating_sub(1);
                }
            }
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
            let size = egui::vec2(120.0, 60.0);

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
                let rect = egui::Rect::from_min_size(self.sections[i].pos, size);

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
                    .Block_colour
                    .as_ref()
                    .map(|c| parse_hex_colour(c))
                    .unwrap_or(egui::Color32::from_rgb(80, 160, 240));

                painter.rect_filled(rect, 6.0, color);

                let label = self.sections[i]
                    .Shown_element
                    .clone()
                    .unwrap_or_else(|| self.sections[i].id.clone());

                painter.text(
                    self.sections[i].pos + egui::vec2(10.0, 10.0),
                    egui::Align2::LEFT_TOP,
                    label,
                    egui::TextStyle::Body.resolve(ui.style()),
                    ui.style().visuals.text_color(),
                );
            }
        });

        if let Some(idx) = delete_request {
            self.delete_block(idx);
        }
    }
}
