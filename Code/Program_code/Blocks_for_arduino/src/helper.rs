// Helper functions for UI and color processing
use eframe::egui;

/// Parses a hexadecimal color string and converts it to an egui Color32.
/// 
/// Expects a 6-character hex string (with or without '#' prefix).
/// Returns the parsed color or LIGHT_GRAY if parsing fails.
/// 
/// # Arguments
/// * `hex` - A hex color string (e.g., "#FF5733" or "FF5733")
/// 
/// # Returns
/// An egui::Color32 representing the parsed color
pub fn parse_hex_colour(hex: &str) -> egui::Color32 {
    let hex = hex.trim_start_matches('#');

    // Only process valid 6-character hex strings
    if hex.len() == 6 {
        if let Ok(v) = u32::from_str_radix(hex, 16) {
            return egui::Color32::from_rgb(
                ((v >> 16) & 0xFF) as u8,
                ((v >> 8) & 0xFF) as u8,
                (v & 0xFF) as u8,
            );
        }
    }

    // Return default color if parsing fails
    egui::Color32::LIGHT_GRAY
}