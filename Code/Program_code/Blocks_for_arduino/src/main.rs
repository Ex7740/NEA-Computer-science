use std::fs;

use eframe::egui;
// use egui::epaint::tessellator::path;
use serde::Deserialize;

fn main() -> eframe::Result<()> {
    
    let mut files = Vec::new(); 

    for entry in fs::read_dir("Json_files").unwrap() {

        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() {
            println!("Found JSON file: {:?}", path);
            files.push(path);
        }
    }
    
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_title("Blocks for arduino"),
        ..Default::default()
    };

    

    eframe::run_native(
        "Blocks for arduino",
        options,
        Box::new(|_cc| {
            let mut app = BlocksForArduino::default();
            for path in files.iter().map(|p| p.to_str().unwrap()) {
                app.load_block_json(path);
            }
            println!("Loaded {} block sections", app.sections.len()
                    
            
            );
            Ok(Box::new(app))
        }),
    )
}

#[derive(Deserialize, Debug, Clone)]
struct BlockFile {
    block: BlockContainer,
}

#[derive(Deserialize, Debug, Clone)]
struct BlockContainer {
    sections: Vec<BlockSection>,
}

#[derive(Deserialize, Debug, Clone)]
struct BlockSection {
    id: String,

    #[serde(default)]
    Block_colour: Option<String>,

    #[serde(default)]
    descriptor: Option<String>,

    #[serde(default)]
    Shown_element: Option<String>,

    #[serde(default)]
    code_equivelant: Option<String>,

    #[serde(skip)]
    pos: egui::Pos2,
}

#[derive(Default)]
struct BlocksForArduino {
    sections: Vec<BlockSection>,
}

fn parse_hex_colour(hex: &str) -> egui::Color32 {
    let hex = hex.trim_start_matches('#');

    if hex.len() == 6 {
        if let Ok(value) = u32::from_str_radix(hex, 16) {
            return egui::Color32::from_rgb(
                ((value >> 16) & 0xFF) as u8,
                ((value >> 8) & 0xFF) as u8,
                (value & 0xFF) as u8,
            );
        }
    }

    egui::Color32::LIGHT_GRAY
}

impl BlocksForArduino {
    fn load_block_json(&mut self, path: &str) {
        if let Ok(raw) = std::fs::read_to_string(path) {
            match serde_json::from_str::<BlockFile>(&raw) {
                Ok(file) => {
                    if let Some(mut first) = file.block.sections.into_iter().next() {
                        // position blocks vertically
                        first.pos = egui::pos2(
                            20.0,
                            60.0 + self.sections.len() as f32 * 80.0,
                        );

                        //Only adds the show section from the json file
                        self.sections.push(first);
                    }
                }
                Err(e) => eprintln!("JSON parse error in {path}: {e}"),
            }
        } else {
            eprintln!("Failed to read file: {path}");
        }
    }
}




impl eframe::App for BlocksForArduino {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        egui::CentralPanel::default().show(ctx, |ui| {
            let painter = ui.painter();
            let screen = ui.max_rect();

            // Titles
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

            // Divider
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

            for sec in &mut self.sections {
                let size = egui::vec2(120.0, 60.0);
                let rect = egui::Rect::from_min_size(sec.pos, size);
            
                // 1️⃣ Interaction FIRST (mutable borrow)
                let response = ui.allocate_rect(
                    rect,
                    egui::Sense::click_and_drag(),
                );
            
                if response.dragged() {
                    sec.pos += response.drag_delta();
                }
            
                // 2️⃣ Painting AFTER (immutable borrow)
                let painter = ui.painter();
            
                let color = sec
                    .Block_colour
                    .as_ref()
                    .map(|c| parse_hex_colour(c))
                    .unwrap_or(egui::Color32::from_rgb(80, 160, 240));
            
                painter.rect_filled(rect, 6.0, color);
            
                let label = sec
                    .Shown_element
                    .clone()
                    .unwrap_or_else(|| sec.id.clone());
            
                painter.text(
                    sec.pos + egui::vec2(10.0, 10.0),
                    egui::Align2::LEFT_TOP,
                    label,
                    egui::TextStyle::Body.resolve(ui.style()),
                    ui.style().visuals.text_color(),
                );
            }
        });
    }
}
