mod app;
mod model;
mod helper;

use std::fs;
use eframe::egui;
use app::BlocksForArduino;

fn main() -> eframe::Result<()> {
    let mut files = Vec::new();

    if let Ok(entries) = fs::read_dir("Json_files") {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                files.push(path);
            }
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

            for path in files.iter().filter_map(|p| p.to_str()) {
                app.load_block_json(path);
            }

            Ok(Box::new(app))
        }),
    )
}
