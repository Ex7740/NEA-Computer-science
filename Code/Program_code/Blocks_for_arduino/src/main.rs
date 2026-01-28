use eframe::{egui};
use serde::Deserialize;

fn main() -> eframe::Result<()> {
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
            
            // ===== NEW =====
            // Load block definitions from JSON
            app.load_block_json("Json_files/test.json");
            // ===== END NEW =====

            Ok(Box::new(app))
        }),
    )
}

#[derive(Deserialize, Debug, Clone)]
struct BlockFile{
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
    Block_colour: Option<String>,    // hex color

    #[serde(default)]
    descriptor: Option<String>,

    #[serde(default)]
    Shown_element: Option<String>,

    #[serde(default)]
    Code_Equivelant: Option<String>,
}




#[derive(Default)]
struct BlocksForArduino {
    // ===== NEW =====
    sections: Vec<BlockSection>, // store parsed blocks
    // ===== END NEW =====
}

fn parse_hex_colour(hex: &str) -> egui::Color32{

    let hex = hex.trim_start_matches("#");

    if hex.len() == 6 {
        if let Ok(value) = u32::from_str_radix(hex, 16) {
            let r = ((value >> 16) & 0xFF) as u8;
            let g = ((value >> 8) & 0xFF) as u8;
            let b = (value & 0xFF) as u8;
            return egui::Color32::from_rgb(r, g, b);
        }
    }

    egui::Color32::LIGHT_GRAY
    
}

impl BlocksForArduino {
    fn load_block_json(&mut self, path: &str) {
        if let Ok(raw) = std::fs::read_to_string(path) {
            if let Ok(file) = serde_json::from_str::<BlockFile>(&raw) {

                // ===== NEW: Load only the FIRST section =====
                if let Some(first) = file.block.sections.into_iter().next() {
                    self.sections = vec![first];   // store only one section
                } else {
                    eprintln!("JSON contains no sections");
                }
                // ===== END NEW =====

            } else {
                eprintln!("Failed to parse JSON");
            }
        } else {
            eprintln!("Failed to read {path}");
        }
    }
}

impl eframe::App for BlocksForArduino {
    fn update( &mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        
        let screen = _ctx.screen_rect();
        let x = 300.0;

        let painter = _ctx.debug_painter();

        let block_label = egui::pos2(10.0, 10.0);
        let code_label = egui::pos2(320.0, 10.0);

        painter.text(
            block_label,
            egui::Align2::LEFT_TOP,
            "Block section",
            egui::TextStyle::Heading.resolve(&_ctx.style()),
            _ctx.style().visuals.text_color(),
        );        

        painter.text(
            code_label,
            egui::Align2::LEFT_TOP,
            "Code section",
            egui::TextStyle::Heading.resolve(&_ctx.style()),
            _ctx.style().visuals.text_color(),
        );

        painter.line_segment(
            [
                egui::pos2(x, screen.top()),
                egui::pos2(x, screen.bottom()),
            ],

            egui::Stroke {
                width: 1.0,
                color: _ctx.style().visuals.widgets.noninteractive.bg_stroke.color,
            }
        );
        let mut y_offset = 60.0;

        for sec in &self.sections {

            // Text on the block (Shown_element or fallback to id)
            let label = sec.Shown_element.clone().unwrap_or(sec.id.clone());

            // Block color
            let color = sec
                .Block_colour
                .as_ref()
                .map(|s| parse_hex_colour(s))
                .unwrap_or(egui::Color32::from_rgb(80, 160, 240));

            // Block rectangle
            painter.rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(20.0, y_offset),
                    egui::vec2(100.0, 100.0),
                ),
                6.0,
                color,
            );

            // Block text
            painter.text(
                egui::pos2(30.0, y_offset + 15.0),
                egui::Align2::LEFT_TOP,
                label,
                egui::TextStyle::Body.resolve(&_ctx.style()),
                _ctx.style().visuals.text_color(),
            );

            y_offset += 70.0; // move down for next block
        }
    }
}

