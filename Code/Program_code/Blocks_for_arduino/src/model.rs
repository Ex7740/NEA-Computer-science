// Data structures for block definitions, workspace management, and serialization
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a list entry for block sequences - either a single block or a group
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(untagged)]
pub enum BlockListEntry {
    /// A single block ID
    Single(String),
    /// A group of connected blocks
    Group(Vec<String>),
}

/// Wrapper structure for deserializing block definitions from JSON files
#[derive(Deserialize, Clone)]
pub struct BlockFile {
    pub block: BlockContainer,
}

/// Container for block sections loaded from JSON
#[derive(Deserialize, Clone)]
pub struct BlockContainer {
    pub sections: Vec<BlockSection>,
}

/// Represents a single block in the application. Contains both static properties
/// (loaded from JSON) and runtime properties (created during execution).
#[derive(Deserialize, Clone)]
pub struct BlockSection {
    /// Unique identifier for the block type
    pub id: String,

    /// Optional unique ID used in valid sequences (falls back to id if not specified)
    #[serde(default)]
    pub unique_id: Option<String>,

    /// Hex color code for the block appearance (e.g., "#FF5733")
    #[serde(default)]
    #[serde(rename = "Block_colour")]
    pub block_colour: Option<String>,

    /// Display label shown on the block in the UI
    #[serde(default)]
    #[serde(rename = "Shown_element")]
    pub shown_element: Option<String>,

    /// Arduino code template with {placeholder} tokens for input substitution
    #[serde(default)]
    #[serde(rename = "Code_Equivelant")]
    pub code_equivelant: Option<String>,

    /// Offset for positioning child blocks relative to this block
    #[serde(default)]
    pub child_offset: Option<Offset>,

    /// Input definitions specifying which values the block requires
    #[serde(default)]
    pub inputs: Vec<InputDefinition>,

    /* ---------- RUNTIME ONLY (not serialized) ---------- */

    /// Current position of this block in the editor canvas
    #[serde(skip)]
    pub pos: egui::Pos2,

    /// Index of the block this one is attached to (parent block)
    #[serde(skip)]
    pub attached_to: Option<usize>,

    /// Indices of blocks attached below this one (child blocks)
    #[serde(skip)]
    pub children: Vec<usize>,

    /// Input values entered by the user for this block instance
    #[serde(skip)]
    pub input_values: HashMap<String, String>,

    /// Unique instance identifier for this block (persisted across save/load)
    #[serde(skip)]
    pub instance_id: String,

    /// True if this is a palette block (template), false if code block (instance)
    #[serde(skip)]
    pub is_palette: bool,
}

/// Definition for an input field on a block, including validation rules
#[derive(Deserialize, Clone)]
pub struct InputDefinition {
    /// Name of the input field
    pub name: String,
    /// Optional validation rule. Supported values:
    ///   "arduino_pin"       – 0–13 or A0–A5
    ///   "arduino_state"     – HIGH / LOW / 1 / 0
    ///   "positive_integer"  – whole number > 0
    #[serde(default)]
    pub validation: Option<String>,
}

/// X and Y offset coordinates for positioning child blocks
#[derive(Deserialize, Clone, Copy)]
pub struct Offset {
    /// X-axis offset in pixels
    pub x: f32,
    /// Y-axis offset in pixels
    pub y: f32,
}

// ---------- WORKSPACE PERSISTENCE ----------

/// Snapshot of a single block's state for saving to disk
#[derive(Serialize, Deserialize)]
pub struct BlockSnapshot {
    /// Block type unique identifier
    pub unique_id: String,
    /// X-coordinate on the canvas
    pub pos_x: f32,
    /// Y-coordinate on the canvas
    pub pos_y: f32,
    /// Input values entered by the user
    pub input_values: HashMap<String, String>,
    /// Unique instance ID for this block
    pub instance_id: String,
    /// Instance ID of the parent block (if attached)
    pub attached_to_instance_id: Option<String>,
}

/// Complete workspace snapshot including all blocks and metadata
#[derive(Serialize, Deserialize)]
pub struct WorkspaceSnapshot {
    /// Name of the workspace
    pub name: String,
    /// All non-palette blocks in the workspace
    pub blocks: Vec<BlockSnapshot>,
}

impl Offset {
    /// Converts this Offset to an egui::Vec2 for use in UI calculations
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