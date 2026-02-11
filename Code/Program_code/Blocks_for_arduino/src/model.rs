use eframe::egui;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Clone)]
pub struct BlockFile {
    pub block: BlockContainer,
}

#[derive(Deserialize, Clone)]
pub struct BlockContainer {
    pub sections: Vec<BlockSection>,
}

#[derive(Deserialize, Clone)]
pub struct BlockSection {
    pub id: String,

    #[serde(default)]
    #[serde(rename = "Block_colour")]
    pub block_colour: Option<String>,

    #[serde(default)]
    #[serde(rename = "Shown_element")]
    pub shown_element: Option<String>,

    // #[serde(default)]
    // pub descriptor: Option<String>,

    #[serde(default)]
    pub child_offset: Option<Offset>,

    #[serde(default)]
    pub inputs: Vec<InputDefinition>,

    /* ---------- RUNTIME ONLY ---------- */

    #[serde(skip)]
    pub pos: egui::Pos2,

    #[serde(skip)]
    pub attached_to: Option<usize>,

    #[serde(skip)]
    pub children: Vec<usize>,

    #[serde(skip)]
    pub input_values: HashMap<String, String>,
}

#[derive(Deserialize, Clone)]
pub struct InputDefinition {
    pub name: String,
}

#[derive(Deserialize, Clone, Copy)]
pub struct Offset {
    pub x: f32,
    pub y: f32,
}

impl Offset {
    pub fn vec2(self) -> egui::Vec2 {
        egui::vec2(self.x, self.y)
    }
}
