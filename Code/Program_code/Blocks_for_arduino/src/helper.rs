use eframe::egui;

pub fn parse_hex_colour(hex: &str) -> egui::Color32 {
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
