// Module declarations for the application
mod app;
mod model;
mod helper;

use std::fs;
use eframe::egui;
use app::BlocksForArduino;

/// Entry point for the Blocks for Arduino application.
/// 
/// Initializes the egui GUI framework with a 1000x650 window and loads all
/// block definitions from JSON files in the "Json_files" directory. Also
/// generates and syncs valid sequences based on loaded blocks.
fn main() -> eframe::Result<()> {
    // Collect all JSON files from the "Json_files" directory
    let mut files = Vec::new();

    if let Ok(entries) = fs::read_dir("Json_files") {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                files.push(path);
            }
        }
    }

    // Configure the application window
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 650.0])
            .with_title("Blocks for Arduino"),
        ..Default::default()
    };

    // Launch the application
    eframe::run_native(
        "Blocks for Arduino",
        options,
        Box::new(|_cc| {
            let mut app = BlocksForArduino::default();

            // Load all block JSON files to populate the palette
            for path in files.iter().filter_map(|p| p.to_str()) {
                app.load_block_json(path);
            }

            // Generate valid block sequences based on loaded blocks
            app.sync_valid_sequences_with_loaded_blocks();

            Ok(Box::new(app))
        }),
    )
}