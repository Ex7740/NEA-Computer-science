use eframe::egui;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(untagged)]
pub enum BlockListEntry {
    Single(String),
    Group(Vec<String>),
}

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
    pub unique_id: Option<String>,

    #[serde(default)]
    #[serde(rename = "Block_colour")]
    pub block_colour: Option<String>,

    #[serde(default)]
    #[serde(rename = "Shown_element")]
    pub shown_element: Option<String>,

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

    #[serde(skip)]
    pub instance_id: String,

    #[serde(skip)]
    pub is_palette: bool,

}

#[derive(Deserialize, Clone)]
pub struct InputDefinition {
    pub name: String,
    /// Optional validation rule. Supported values:
    ///   "arduino_pin"       – 0–13 or A0–A5
    ///   "arduino_state"     – HIGH / LOW / 1 / 0
    ///   "positive_integer"  – whole number > 0
    #[serde(default)]
    pub validation: Option<String>,
}

#[derive(Deserialize, Clone, Copy)]
pub struct Offset {
    pub x: f32,
    pub y: f32,
}

// ---------- WORKSPACE PERSISTENCE ----------

#[derive(Serialize, Deserialize)]
pub struct BlockSnapshot {
    pub unique_id: String,
    pub pos_x: f32,
    pub pos_y: f32,
    pub input_values: HashMap<String, String>,
    pub instance_id: String,
    pub attached_to_instance_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct WorkspaceSnapshot {
    pub name: String,
    pub blocks: Vec<BlockSnapshot>,
}

impl Offset {
    pub fn vec2(self) -> egui::Vec2 {
        egui::vec2(self.x, self.y)
    }
}

/*
#[derive(Serialize, Deserialize, Clone)]
pub struct WorkspaceSnapshot {
    pub blocks: Vec<SavedBlock>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SavedBlock {
    pub instance_id: String,
    pub definition_id: String,
    pub unique_id: Option<String>,
    pub pos: SavedPos,
    pub attached_to: Option<String>,
    pub input_values: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct SavedPos {
    pub x: f32,
    pub y: f32,
}


impl SavedPos {
    pub fn from_pos2(pos: egui::Pos2) -> Self {
        Self { x: pos.x, y: pos.y }
    }

    pub fn to_pos2(self) -> egui::Pos2 {
        egui::pos2(self.x, self.y)
    }
}
*/