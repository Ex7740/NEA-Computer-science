// Main application logic for Blocks for Arduino
// This file contains the core state management, block manipulation, validation, 
// workspace persistence, and UI rendering for the visual block-based Arduino code editor.

use crate::helper::*;
use crate::model::*;
use eframe::egui;
use std::collections::{HashMap, HashSet};

// UI Layout Constants
const GLOBAL_X: f32 = 140.0;           // Block width in pixels
const GLOBAL_Y: f32 = 90.0;            // Block height in pixels
const VALID_SEQUENCES_PATH: &str = "Valid_sequences.txt";
const INO_OUTPUT_DIR: &str = "INO";
const BLOCKS_START_Y: f32 = 120.0;     // Y-position where blocks start rendering
const PALETTE_BLOCK_GAP: f32 = 10.0;   // Vertical gap between palette blocks
const MAX_SEQUENCE_BLOCKS_FOR_GENERATION: usize = 7; // Max blocks to use in sequence generation

/// Main application state for the Blocks for Arduino editor.
/// 
/// Manages all blocks (both palette templates and code instances), block connections,
/// user interactions, validation, and workspace management. The application uses a
/// tree structure where blocks can be attached as children to form executable sequences.
pub struct BlocksForArduino {
    /// All blocks in the editor (both palette and code blocks)
    pub sections: Vec<BlockSection>,
    
    /// Tracks whether mouse was pressed in previous frame (for detecting releases)
    pub was_mouse_down: bool,
    
    /// Current top-level block connections (can be Single or Group)
    pub current_blocks: Vec<BlockListEntry>,
    
    /// Status message displayed in the toolbar
    pub status_message: String,
    
    /// Valid block sequences loaded from Valid_sequences.txt
    pub valid_sequences: Vec<Vec<String>>,
    
    /// Popup state for sequence validation suggestions
    pub show_sequence_popup: bool,
    pub sequence_popup_text: String,
    
    /// Popup state for input validation errors
    pub show_validation_popup: bool,
    pub validation_popup_text: String,
    
    // Workspace management
    /// Name of the currently open workspace
    pub workspace_name: String,
    
    /// State for Open Workspace dialog
    pub show_open_dialog: bool,
    pub available_workspaces: Vec<String>,
    
    /// State for Save As dialog
    pub show_save_as_dialog: bool,
    pub save_as_name_input: String,
    
    /// Scroll position for palette blocks (left side)
    pub palette_scroll_offset: f32,
}

impl Default for BlocksForArduino {
    fn default() -> Self {
        Self {
            sections: Vec::new(),
            was_mouse_down: false,
            current_blocks: Vec::new(),
            status_message: String::new(),
            valid_sequences: Self::load_valid_sequences(VALID_SEQUENCES_PATH),
            show_sequence_popup: false,
            sequence_popup_text: String::new(),
            show_validation_popup: false,
            validation_popup_text: String::new(),
            workspace_name: String::new(),
            show_open_dialog: false,
            available_workspaces: Vec::new(),
            show_save_as_dialog: false,
            save_as_name_input: String::new(),
            palette_scroll_offset: 0.0,
        }
    }
}

/* ---------- APP LOGIC ---------- */
impl BlocksForArduino {
    /// Generates a unique instance identifier using UUID v4
    fn new_instance_id() -> String {
        uuid::Uuid::new_v4().to_string()
    }

    /// Gets the unique_id for a block, falling back to id if unique_id is not set
    fn block_unique_id(&self, idx: usize) -> String {
        self.sections[idx]
            .unique_id
            .clone()
            .unwrap_or_else(|| self.sections[idx].id.clone())
    }

    /// Counts the number of code blocks (excludes palette blocks)
    fn code_block_count(&self) -> usize {
        self.sections.iter().filter(|block| !block.is_palette).count()
    }

    /// Creates an empty HashMap of input values based on the block's input definitions
    fn build_input_values(block: &BlockSection) -> HashMap<String, String> {
        let mut values = HashMap::new();
        for input in &block.inputs {
            values.insert(input.name.clone(), String::new());
        }
        values
    }

    /// Initializes all runtime-only fields on a block (position, connections, input values, etc.)
    /// This is called when creating new blocks or loading from snapshots.
    fn initialise_runtime_fields(block: &mut BlockSection, pos: egui::Pos2, is_palette: bool) {
        block.pos = pos;
        block.attached_to = None;
        block.children.clear();
        block.input_values = Self::build_input_values(block);
        block.instance_id = Self::new_instance_id();
        block.is_palette = is_palette;
    }

    /// Loads a block definition from a JSON file and adds it to the palette.
    /// 
    /// The JSON file should contain a "block" key with "sections" containing block definitions.
    /// Only the first "show" section or the section with shown_element is displayed in the palette.
    /// Looks for an "A_C_E" section to extract the code template.
    pub fn load_block_json(&mut self, path: &str) {
        let raw = match std::fs::read_to_string(path) {
            Ok(r) => r,
            Err(e) => {
                println!("Failed to read {}: {}", path, e);
                return;
            }
        };

        let file: BlockFile = match serde_json::from_str(&raw) {
            Ok(f) => f,
            Err(e) => {
                println!("Invalid JSON in {}: {}", path, e);
                return;
            }
        };

        let mut sections = file.block.sections.into_iter();

        let mut show_section = None;
        let mut ace_template = None;

        for section in sections.by_ref() {
            let section_id = section.id.to_ascii_lowercase();
            if section_id == "show" || section.shown_element.is_some() {
                if show_section.is_none() {
                    show_section = Some(section.clone());
                }
            }

            if section_id == "a_c_e" {
                ace_template = section.code_equivelant.clone();
            }
        }

        let mut block = match show_section {
            Some(mut b) => {
                b.code_equivelant = ace_template;
                b
            }
            None => return,
        };

        // If no unique_id provided, use the block's id field
        if block.unique_id.is_none() {
            block.unique_id = Some(block.id.clone());
        }

        Self::initialise_runtime_fields(
            &mut block,
            egui::pos2(
                20.0,
                BLOCKS_START_Y
                    + self.sections.iter().filter(|b| b.is_palette).count() as f32
                        * (GLOBAL_Y + PALETTE_BLOCK_GAP),
            ),
            true,
        );
        self.sections.push(block);
    }

    fn loaded_palette_block_ids(&self) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut ids = Vec::new();

        for block in self.sections.iter().filter(|block| block.is_palette) {
            let block_id = block
                .unique_id
                .as_ref()
                .cloned()
                .unwrap_or_else(|| block.id.clone());
            if seen.insert(block_id.clone()) {
                ids.push(block_id);
            }
        }

        ids
    }

    /// Recursively generates all possible permutations of block sequences up to MAX_SEQUENCE_BLOCKS_FOR_GENERATION
    /// Used for validating block connection orders
    fn generate_sequences_recursive(
        block_ids: &[String],
        used: &mut [bool],
        current: &mut Vec<String>,
        out: &mut Vec<Vec<String>>,
    ) {
        for idx in 0..block_ids.len() {
            if used[idx] {
                continue;
            }

            used[idx] = true;
            current.push(block_ids[idx].clone());
            out.push(current.clone());
            Self::generate_sequences_recursive(block_ids, used, current, out);
            current.pop();
            used[idx] = false;
        }
    }

    /// Generates all valid block sequence permutations from available palette blocks
    /// Useful for initial setup and validation of block order constraints
    fn generate_valid_sequences_from_blocks(block_ids: &[String]) -> Vec<Vec<String>> {
        if block_ids.is_empty() {
            return Vec::new();
        }

        let limited_ids: Vec<String> = block_ids
            .iter()
            .take(MAX_SEQUENCE_BLOCKS_FOR_GENERATION)
            .cloned()
            .collect();

        let mut used = vec![false; limited_ids.len()];
        let mut current = Vec::new();
        let mut sequences = Vec::new();
        Self::generate_sequences_recursive(&limited_ids, &mut used, &mut current, &mut sequences);
        sequences
    }

    pub fn sync_valid_sequences_with_loaded_blocks(&mut self) {
        let palette_ids = self.loaded_palette_block_ids();
        let generated = Self::generate_valid_sequences_from_blocks(&palette_ids);
        self.valid_sequences = generated.clone();

        match serde_json::to_string_pretty(&generated) {
            Ok(serialized) => {
                if let Err(err) = std::fs::write(VALID_SEQUENCES_PATH, serialized) {
                    eprintln!("Failed to write {}: {}", VALID_SEQUENCES_PATH, err);
                }
            }
            Err(err) => {
                eprintln!("Failed to serialise generated sequences: {}", err);
            }
        }
    }

    /// Creates a new code block instance from a palette block.
    /// The new block is positioned offset from existing code blocks.
    pub fn spawn_code_block(&mut self, source: usize) {
        let mut new_block = self.sections[source].clone();
        let offset = self.code_block_count() as f32 * 15.0;
        Self::initialise_runtime_fields(
            &mut new_block,
            egui::pos2(320.0 + offset, BLOCKS_START_Y + offset),
            false,
        );

        self.sections.push(new_block);
        self.refresh_current_blocks();
    }

    /// Detaches a block from its parent, removing it from the parent's children list.
    /// Used when the user drags a block away or before deletion.
    fn detach_block(&mut self, idx: usize) {
        if let Some(parent) = self.sections[idx].attached_to.take() {
            self.sections[parent].children.retain(|&child| child != idx);
            self.refresh_current_blocks();
        }
    }

    /// Attempts to snap a block to a parent if it's positioned within snap distance.
    /// Searches for compatible parents and establishes parent-child connection if appropriate.
    fn try_snap(&mut self, idx: usize) {
        let size = egui::vec2(GLOBAL_X, GLOBAL_Y);
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

            if (my_pos.y - target_y).abs() < snap && (my_pos.x - target_x).abs() < snap {
                self.detach_block(idx);
                self.sections[idx].pos = egui::pos2(target_x, target_y);
                self.sections[idx].attached_to = Some(j);
                if !self.sections[j].children.contains(&idx) {
                    self.sections[j].children.push(idx);
                }
                self.refresh_current_blocks();
                break;
            }
        }
    }

    /// Repositions all child blocks in a vertical line below their parent.
    /// Called after parent block is moved to maintain proper child alignment.
    fn move_children(&mut self, parent: usize) {
        let size = egui::vec2(GLOBAL_X, GLOBAL_Y);
        let base = self.sections[parent].pos;
        let offset = self.sections[parent]
            .child_offset
            .map(|o| o.vec2())
            .unwrap_or(egui::vec2(0.0, 0.0));

        let mut y = base.y + size.y + offset.y;
        let children = self.sections[parent].children.clone();

        for child in children {
            self.sections[child].pos = egui::pos2(base.x + offset.x, y);
            y += size.y;
            self.move_children(child);
        }
    }

    /// Recursively collects all descendants (children, grandchildren, etc.) of a block
    /// as a flat list of indices. Used for deletion and serialization.
    fn collect_descendants(&self, idx: usize, out: &mut Vec<usize>) {
        out.push(idx);
        for &child in &self.sections[idx].children {
            self.collect_descendants(child, out);
        }
    }

    /// Recursively collects the unique IDs of a block and all its connected descendants
    /// Used to build the current block sequence representation.
    fn collect_connected_unique_ids(&self, idx: usize, out: &mut Vec<String>) {
        out.push(self.block_unique_id(idx));
        for &child in &self.sections[idx].children {
            self.collect_connected_unique_ids(child, out);
        }
    }

    fn build_current_blocks(&self) -> Vec<BlockListEntry> {
        let mut roots: Vec<usize> = self
            .sections
            .iter()
            .enumerate()
            .filter(|(_, block)| !block.is_palette && block.attached_to.is_none())
            .map(|(idx, _)| idx)
            .collect();
        roots.sort_by(|a, b| self.sections[*a].pos.y.total_cmp(&self.sections[*b].pos.y));

        roots
            .into_iter()
            .map(|root| {
                let mut group = Vec::new();
                self.collect_connected_unique_ids(root, &mut group);
                if group.len() == 1 {
                    BlockListEntry::Single(group.remove(0))
                } else {
                    BlockListEntry::Group(group)
                }
            })
            .collect()
    }

    /// Converts the current blocks to a JSON string representation for display/debugging
    fn current_blocks_json(&self) -> String {
        serde_json::to_string(&self.current_blocks).unwrap_or_else(|_| "[]".to_string())
    }

    /// Loads valid block sequences from the specified JSON file.
    /// Returns empty Vec if file is missing or contains invalid JSON.
    fn load_valid_sequences(path: &str) -> Vec<Vec<String>> {
        let raw = match std::fs::read_to_string(path) {
            Ok(raw) => raw,
            Err(err) => {
                eprintln!("Failed to read {}: {}", path, err);
                return Vec::new();
            }
        };

        match serde_json::from_str::<Vec<Vec<String>>>(&raw) {
            Ok(sequences) => sequences,
            Err(err) => {
                eprintln!("Invalid sequence JSON in {}: {}", path, err);
                Vec::new()
            }
        }
    }

    /// Converts current block connections into a flat list of sequences.
    /// Each sequence is either a single block or a group of connected blocks.
    fn flatten_current_sequences(&self) -> Vec<Vec<String>> {
        self.current_blocks
            .iter()
            .map(|entry| match entry {
                BlockListEntry::Single(id) => vec![id.clone()],
                BlockListEntry::Group(ids) => ids.clone(),
            })
            .collect()
    }

    /// Calculates the edit distance (Levenshtein-like) between two block sequences.
    /// Used for suggesting corrections when validation fails.
    fn sequence_distance(a: &[String], b: &[String]) -> usize {
        let min_len = a.len().min(b.len());
        let mut distance = a.len().abs_diff(b.len());
        for i in 0..min_len {
            if a[i] != b[i] {
                distance += 1;
            }
        }
        distance
    }

    /// Gathers all available block IDs (both palette and code blocks)
    fn available_block_ids(&self) -> HashSet<String> {
        self.sections
            .iter()
            .map(|block| {
                block
                    .unique_id
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| block.id.clone())
            })
            .collect()
    }

    /// Filters valid sequences to only include those where all blocks are available
    fn valid_sequences_for_blocks<'a>(
        &'a self,
        available_blocks: &HashSet<String>,
    ) -> Vec<&'a Vec<String>> {
        self.valid_sequences
            .iter()
            .filter(|sequence| {
                sequence
                    .iter()
                    .all(|block_id| available_blocks.contains(block_id))
            })
            .collect()
    }

    /// Suggests the closest valid sequence to the current one using sequence distance
    fn suggest_sequence(sequence: &[String], candidates: &[&Vec<String>]) -> Option<Vec<String>> {
        candidates
            .iter()
            .min_by_key(|candidate| Self::sequence_distance(sequence, candidate))
            .map(|candidate| (*candidate).clone())
    }

    /// Formats a sequence into a readable string like "Block1 -> Block2 -> Block3"
    fn join_sequence(sequence: &[String]) -> String {
        sequence.join(" -> ")
    }

    /// Validates that the current block sequence matches one of the valid sequences.
    /// Returns detailed error messages with suggestions if validation fails.
    fn validate_current_sequences(&self) -> Result<(), String> {
        if self.valid_sequences.is_empty() {
            return Err(format!(
                "Could not validate because {} is missing or invalid JSON",
                VALID_SEQUENCES_PATH
            ));
        }

        let available_blocks = self.available_block_ids();
        let valid_sequences = self.valid_sequences_for_blocks(&available_blocks);
        if valid_sequences.is_empty() {
            return Err(format!(
                "No valid sequences can be checked because every entry in {} references blocks that are not currently loaded.",
                VALID_SEQUENCES_PATH
            ));
        }

        let sequences = self.flatten_current_sequences();

        for (idx, sequence) in sequences.iter().enumerate() {
            if let Some(missing_block) =
                sequence.iter().find(|block_id| !available_blocks.contains(*block_id))
            {
                return Err(format!(
                    "Sequence {} contains unknown block '{}'.\nCurrent: {}\nSuggested: add '{}' to {} or remove it from the sequence.",
                    idx + 1,
                    missing_block,
                    Self::join_sequence(sequence),
                    missing_block,
                    VALID_SEQUENCES_PATH
                ));
            }

            if valid_sequences.iter().any(|valid| *valid == sequence) {
                continue;
            }

            let current_text = Self::join_sequence(sequence);
            let suggestion = Self::suggest_sequence(sequence, &valid_sequences)
                .map(|s| Self::join_sequence(&s))
                .unwrap_or_else(|| "No suggestion available".to_string());

            return Err(format!(
                "Sequence {} is not valid.\nCurrent: {}\nSuggested: {}",
                idx + 1,
                current_text,
                suggestion
            ));
        }

        Ok(())
    }

    // ------------------------------------------------------------------
    // Input value validation
    // ------------------------------------------------------------------

    /// Checks a single raw value against a validation rule and returns a
    /// human-readable error with a concrete suggestion when invalid.
    fn validate_input_value(value: &str, rule: &str, field_name: &str) -> Result<(), String> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(format!(
                "'{field_name}' must not be empty.\nSuggested: fill in a valid value."
            ));
        }
        match rule {
            "arduino_pin" => {
                let upper = trimmed.to_uppercase();
                let valid = upper
                    .parse::<u8>()
                    .map_or(false, |n| n <= 13)
                    || (upper.starts_with('A')
                        && upper[1..].parse::<u8>().map_or(false, |n| n <= 5));
                if !valid {
                    return Err(format!(
                        "'{field_name}' has value '{trimmed}' which is not a valid Arduino pin.\n\
                         Suggested: use a number 0–13 for digital pins (e.g. 13),\n\
                         or A0–A5 for analog pins (e.g. A0)."
                    ));
                }
            }
            "arduino_state" => {
                let upper = trimmed.to_uppercase();
                if !matches!(upper.as_str(), "HIGH" | "LOW" | "1" | "0") {
                    return Err(format!(
                        "'{field_name}' has value '{trimmed}' which is not a valid pin state.\n\
                         Suggested: use HIGH or LOW (or the numeric equivalents 1 / 0)."
                    ));
                }
            }
            "positive_integer" => match trimmed.parse::<u32>() {
                Ok(0) => {
                    return Err(format!(
                        "'{field_name}' must be greater than 0.\n\
                         Suggested: enter a positive whole number in milliseconds (e.g. 1000)."
                    ));
                }
                Err(_) => {
                    return Err(format!(
                        "'{field_name}' has value '{trimmed}' which is not a valid number.\n\
                         Suggested: enter a positive whole number in milliseconds (e.g. 1000)."
                    ));
                }
                Ok(_) => {}
            },
            "arduino_condition" => {
                if trimmed.eq_ignore_ascii_case("true") && trimmed != "true" {
                    return Err(format!(
                        "'{field_name}' uses '{trimmed}', but Arduino boolean literals are lowercase.\n\
                         Suggested: use true (lowercase)."
                    ));
                }

                if trimmed.eq_ignore_ascii_case("false") && trimmed != "false" {
                    return Err(format!(
                        "'{field_name}' uses '{trimmed}', but Arduino boolean literals are lowercase.\n\
                         Suggested: use false (lowercase)."
                    ));
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Validates all required inputs across every non-palette block.
    /// Returns the first error found, formatted ready for the popup.
    fn validate_block_inputs(&self) -> Result<(), String> {
        for block in &self.sections {
            if block.is_palette {
                continue;
            }
            let block_name = block
                .unique_id
                .as_deref()
                .unwrap_or(&block.id)
                .to_string();
            for input in &block.inputs {
                if let Some(ref rule) = input.validation {
                    let value = block
                        .input_values
                        .get(&input.name)
                        .map(|s| s.as_str())
                        .unwrap_or("");
                    if let Err(msg) =
                        Self::validate_input_value(value, rule, &input.name)
                    {
                        return Err(format!("Block '{block_name}':\n{msg}"));
                    }
                }
            }
        }
        Ok(())
    }

    /// Validates that all blocks use the same Pin value (if they have one)
    /// Prevents mixing of pins across different code blocks
    fn validate_pin_consistency(&self) -> Result<(), String> {
        let mut expected_pin: Option<String> = None;
        let mut pin_sources: Vec<String> = Vec::new();

        for block in self.sections.iter().filter(|b| !b.is_palette) {
            let pin_value = Self::resolve_input_value(&block.input_values, "Pin")
                .map(|v| v.trim())
                .filter(|v| !v.is_empty());

            let Some(pin) = pin_value else {
                continue;
            };

            let block_name = block.unique_id.as_deref().unwrap_or(&block.id);
            pin_sources.push(format!("{} -> {}", block_name, pin));

            if let Some(ref expected) = expected_pin {
                if !expected.eq_ignore_ascii_case(pin) {
                    return Err(format!(
                        "Pin mismatch detected across code blocks.\nAll blocks with a Pin field must use the same value.\nFound: {}\nExpected (first value): {}\nDetails:\n{}",
                        pin,
                        expected,
                        pin_sources.join("\n")
                    ));
                }
            } else {
                expected_pin = Some(pin.to_string());
            }
        }

        Ok(())
    }

    fn root_block_indices(&self) -> Vec<usize> {
        let mut roots: Vec<usize> = self
            .sections
            .iter()
            .enumerate()
            .filter(|(_, block)| !block.is_palette && block.attached_to.is_none())
            .map(|(idx, _)| idx)
            .collect();

        roots.sort_by(|a, b| {
            self.sections[*a]
                .pos
                .y
                .total_cmp(&self.sections[*b].pos.y)
                .then_with(|| self.sections[*a].pos.x.total_cmp(&self.sections[*b].pos.x))
        });

        roots
    }

    /// Gets all child block indices sorted by position (top-to-bottom, left-to-right)
    fn sorted_child_indices(&self, idx: usize) -> Vec<usize> {
        let mut children = self.sections[idx].children.clone();
        children.sort_by(|a, b| {
            self.sections[*a]
                .pos
                .y
                .total_cmp(&self.sections[*b].pos.y)
                .then_with(|| self.sections[*a].pos.x.total_cmp(&self.sections[*b].pos.x))
        });
        children
    }

    /// Indents a line of code by the specified number of levels (2 spaces per level)
    /// Empty lines are returned unchanged
    fn indent_line(line: &str, level: usize) -> String {
        if line.trim().is_empty() {
            return String::new();
        }

        let mut out = String::new();
        out.push_str(&"  ".repeat(level));
        out.push_str(line);
        out
    }

    /// Recursively renders a block and its children to Arduino code.
    /// Handles template substitution, indentation, and brace nesting for control structures.
    fn render_block_recursive(
        &self,
        idx: usize,
        indent_level: usize,
        out: &mut Vec<String>,
    ) -> Result<(), String> {
        let block = &self.sections[idx];
        let block_name = block.unique_id.as_deref().unwrap_or(&block.id);
        let template = block.code_equivelant.as_deref().ok_or_else(|| {
            format!(
                "Block '{}' does not define an A_C_E Code_Equivelant template.",
                block_name
            )
        })?;

        let (resolved, missing) = Self::fill_template_with_inputs(template, block);
        if !missing.is_empty() {
            return Err(format!(
                "Block '{}' is missing values for placeholders: {}.",
                block_name,
                missing.join(", ")
            ));
        }

        let template_lines: Vec<String> = resolved.lines().map(|line| line.trim_end().to_string()).collect();
        let children = self.sorted_child_indices(idx);
        let close_idx = template_lines.iter().rposition(|line| line.trim() == "}");
        let has_open_brace = template_lines.iter().any(|line| line.contains('{'));

        // For blocks with braces (like if/loop), insert children inside the braces
        if !children.is_empty() && has_open_brace {
            if let Some(close_idx) = close_idx {
                for line in template_lines.iter().take(close_idx) {
                    out.push(Self::indent_line(line, indent_level));
                }

                for child in children {
                    self.render_block_recursive(child, indent_level + 1, out)?;
                }

                for line in template_lines.iter().skip(close_idx) {
                    out.push(Self::indent_line(line, indent_level));
                }

                return Ok(());
            }
        }

        for line in &template_lines {
            out.push(Self::indent_line(line, indent_level));
        }

        for child in children {
            self.render_block_recursive(child, indent_level, out)?;
        }

        Ok(())
    }

    /// Looks up an input value case-insensitively (first exact match, then case-insensitive match)
    fn resolve_input_value<'a>(
        input_values: &'a HashMap<String, String>,
        key: &str,
    ) -> Option<&'a str> {
        if let Some(value) = input_values.get(key) {
            return Some(value.as_str());
        }

        let lower_key = key.to_ascii_lowercase();
        input_values
            .iter()
            .find(|(candidate, _)| candidate.to_ascii_lowercase() == lower_key)
            .map(|(_, value)| value.as_str())
    }

    /// Checks if a string is a valid placeholder token (alphanumeric + underscore + spaces)
    fn is_placeholder_token(token: &str) -> bool {
        !token.is_empty()
            && !token.contains('\n')
            && !token.contains('\r')
            && token
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ' ')
    }

    /// Normalizes input values according to validation rules
    /// (e.g., converts HIGH/1 to 1 for arduino_state validation)
    fn normalize_value_for_rule(value: &str, rule: Option<&str>) -> String {
        match rule {
            Some("arduino_state") => {
                let upper = value.trim().to_ascii_uppercase();
                match upper.as_str() {
                    "HIGH" | "1" => "1".to_string(),
                    "LOW" | "0" => "0".to_string(),
                    _ => value.trim().to_string(),
                }
            }
            _ => value.trim().to_string(),
        }
    }

    /// Finds the input definition for a key (case-insensitive)
    fn resolve_input_definition<'a>(block: &'a BlockSection, key: &str) -> Option<&'a InputDefinition> {
        let lower_key = key.to_ascii_lowercase();
        block
            .inputs
            .iter()
            .find(|candidate| candidate.name.to_ascii_lowercase() == lower_key)
    }

    /// Substitutes placeholder values in a code template.
    /// Placeholders are marked with curly braces like {variable_name}.
    /// Returns the filled template and a list of any missing placeholders.
    fn fill_template_with_inputs(
        template: &str,
        block: &BlockSection,
    ) -> (String, Vec<String>) {
        let mut out = String::with_capacity(template.len());
        let mut missing = Vec::new();
        let mut cursor = 0usize;

        while cursor < template.len() {
            let Some(rel_open) = template[cursor..].find('{') else {
                out.push_str(&template[cursor..]);
                break;
            };

            let open_idx = cursor + rel_open;
            let after_open = open_idx + 1;

            out.push_str(&template[cursor..open_idx]);

            let Some(rel_close) = template[after_open..].find('}') else {
                out.push('{');
                cursor = after_open;
                continue;
            };

            let close_idx = after_open + rel_close;
            let raw_key = &template[after_open..close_idx];
            let key = raw_key.trim();

            if Self::is_placeholder_token(key) {
                if let Some(value) = Self::resolve_input_value(&block.input_values, key) {
                    let rule = Self::resolve_input_definition(block, key)
                        .and_then(|input| input.validation.as_deref());
                    out.push_str(&Self::normalize_value_for_rule(value, rule));
                } else {
                    missing.push(key.to_string());
                    out.push('{');
                    out.push_str(raw_key);
                    out.push('}');
                }
                cursor = close_idx + 1;
            } else {
                out.push('{');
                cursor = after_open;
            }
        }

        (out, missing)
    }

    /// Generates code lines from all root blocks to be placed in the loop() function
    fn render_ino_loop_lines(&self) -> Result<Vec<String>, String> {
        let roots = self.root_block_indices();
        if roots.is_empty() {
            return Err("No code blocks in workspace. Add blocks to generate an .ino file.".to_string());
        }

        let mut lines = Vec::new();

        for root in roots {
            self.render_block_recursive(root, 0, &mut lines)?;
        }

        Ok(lines)
    }

    /// Checks if a line is a pinMode() call, which should be in the setup() method
    fn is_setup_line(line: &str) -> bool {
        line.trim_start().to_ascii_lowercase().starts_with("pinmode(")
    }

    /// Generates the complete Arduino .ino source code from the current blocks.
    /// Separates pinMode() calls into setup() and other code into loop().
    fn build_ino_source(&self) -> Result<String, String> {
        let generated_lines = self.render_ino_loop_lines()?;
        let mut setup_lines = Vec::new();
        let mut loop_lines = Vec::new();

        for line in generated_lines {
            if Self::is_setup_line(&line) {
                setup_lines.push(line);
            } else {
                loop_lines.push(line);
            }
        }

        let mut src = String::new();
        src.push_str("void setup() {\n");
        for line in setup_lines {
            if line.trim().is_empty() {
                src.push('\n');
            } else {
                src.push_str("  ");
                src.push_str(&line);
                src.push('\n');
            }
        }
        src.push_str("}\n\n");
        src.push_str("void loop() {\n");

        for line in loop_lines {
            if line.trim().is_empty() {
                src.push('\n');
            } else {
                src.push_str("  ");
                src.push_str(&line);
                src.push('\n');
            }
        }

        src.push_str("}\n");
        Ok(src)
    }

    /// Exports the current workspace as an Arduino .ino file.
    /// Creates a directory structure and saves the generated code.
    /// Returns the path where the file was written.
    fn export_ino(&self) -> Result<std::path::PathBuf, String> {
        let file_stem = if self.workspace_name.trim().is_empty() {
            "Untitled".to_string()
        } else {
            let safe = Self::safe_filename(&self.workspace_name);
            if safe.is_empty() {
                "Untitled".to_string()
            } else {
                safe
            }
        };

        let workspace_dir = std::path::Path::new(INO_OUTPUT_DIR).join(&file_stem);
        std::fs::create_dir_all(&workspace_dir)
            .map_err(|e| format!("Could not create {}: {e}", workspace_dir.display()))?;

        let path = workspace_dir.join(format!("{}.ino", file_stem));
        let source = self.build_ino_source()?;

        std::fs::write(&path, source)
            .map_err(|e| format!("Could not write {}: {e}", path.display()))?;

        Ok(path)
    }

    /// Rebuilds the current_blocks list from the block tree structure.
    /// Called whenever the block tree changes (attach, detach, delete).
    fn refresh_current_blocks(&mut self) {
        self.current_blocks = self.build_current_blocks();
        println!("Current blocks: {}", self.current_blocks_json());
    }

    /// Deletes a block and all its children from the workspace.
    /// Automatically updates parent-child connections for remaining blocks.
    fn delete_block(&mut self, idx: usize) {
        self.detach_block(idx);

        let mut to_delete = Vec::new();
        self.collect_descendants(idx, &mut to_delete);

        to_delete.sort_unstable_by(|a, b| b.cmp(a));

        for &i in &to_delete {
            self.sections.remove(i);
        }

        for block in &mut self.sections {
            if let Some(parent) = block.attached_to {
                block.attached_to = if to_delete.contains(&parent) {
                    None
                } else {
                    Some(parent - to_delete.iter().filter(|&&deleted| deleted < parent).count())
                };
            }

            let mut updated_children = Vec::new();
            for child in &block.children {
                if !to_delete.contains(child) {
                    let shift = to_delete.iter().filter(|&&deleted| deleted < *child).count();
                    updated_children.push(*child - shift);
                }
            }
            block.children = updated_children;
        }

        self.refresh_current_blocks();
    }
}

/* ---------- WORKSPACE I/O ---------- */

const WORKSPACES_DIR: &str = "workspaces";

impl BlocksForArduino {
    /// Returns a safe filesystem filename from an arbitrary user-supplied name.
    /// Only allows alphanumeric characters, spaces, hyphens, and underscores.
    fn safe_filename(name: &str) -> String {
        let stem: String = name
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-' || *c == '_')
            .collect();
        stem.trim().to_string()
    }

    /// Constructs the filesystem path for a workspace file
    fn workspace_path(name: &str) -> std::path::PathBuf {
        std::path::Path::new(WORKSPACES_DIR).join(format!("{}.json", Self::safe_filename(name)))
    }

    /// Lists all available workspaces saved in the workspaces directory
    pub fn list_workspaces() -> Vec<String> {
        let mut names = Vec::new();
        if let Ok(entries) = std::fs::read_dir(WORKSPACES_DIR) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("json") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        names.push(stem.to_string());
                    }
                }
            }
        }
        names.sort();
        names
    }

    /// Saves the current workspace (including all code blocks but not palette blocks) to a JSON file.
    /// Creates the workspaces directory if it doesn't exist.
    pub fn save_workspace(&self, name: &str) -> Result<(), String> {
        let safe = Self::safe_filename(name);
        if safe.is_empty() {
            return Err("Workspace name must contain at least one letter or digit.".to_string());
        }
        std::fs::create_dir_all(WORKSPACES_DIR)
            .map_err(|e| format!("Could not create workspaces directory: {e}"))?;

        // Snapshot only the code blocks (not palette blocks)
        let blocks: Vec<BlockSnapshot> = self
            .sections
            .iter()
            .filter(|b| !b.is_palette)
            .map(|b| BlockSnapshot {
                unique_id: b.unique_id.clone().unwrap_or_else(|| b.id.clone()),
                pos_x: b.pos.x,
                pos_y: b.pos.y,
                input_values: b.input_values.clone(),
                instance_id: b.instance_id.clone(),
                attached_to_instance_id: b
                    .attached_to
                    .map(|idx| self.sections[idx].instance_id.clone()),
            })
            .collect();

        let snapshot = WorkspaceSnapshot {
            name: safe.clone(),
            blocks,
        };
        let json = serde_json::to_string_pretty(&snapshot)
            .map_err(|e| format!("Serialisation error: {e}"))?;

        let path = Self::workspace_path(&safe);
        std::fs::write(&path, json)
            .map_err(|e| format!("Could not write {}: {e}", path.display()))?;

        Ok(())
    }

    /// Loads a saved workspace by name, restor all code blocks and their connections.
    /// Requires that all referenced block types are already loaded as palette blocks.
    pub fn load_workspace_by_name(&mut self, name: &str) -> Result<(), String> {
        let path = Self::workspace_path(name);
        let json = std::fs::read_to_string(&path)
            .map_err(|e| format!("Could not read {}: {e}", path.display()))?;
        let snapshot: WorkspaceSnapshot = serde_json::from_str(&json)
            .map_err(|e| format!("Invalid workspace JSON: {e}"))?;

        // Remove all non-palette blocks and reset palette connection state
        self.sections.retain(|b| b.is_palette);
        for b in &mut self.sections {
            b.children.clear();
            b.attached_to = None;
        }

        let base_index = self.sections.len();

        // First pass: create code blocks from snapshots
        for snap in &snapshot.blocks {
            let palette_idx = self
                .sections
                .iter()
                .position(|b| b.is_palette && b.unique_id.as_deref().unwrap_or(&b.id) == snap.unique_id.as_str());

            let palette_idx = match palette_idx {
                Some(i) => i,
                None => {
                    return Err(format!(
                        "Unknown block type '{}' in workspace — is the block definition loaded?",
                        snap.unique_id
                    ))
                }
            };

            let mut new_block = self.sections[palette_idx].clone();
            new_block.pos = egui::pos2(snap.pos_x, snap.pos_y);
            new_block.instance_id = snap.instance_id.clone();
            new_block.input_values = snap.input_values.clone();
            new_block.is_palette = false;
            new_block.attached_to = None;
            new_block.children.clear();
            self.sections.push(new_block);
        }

        // Second pass: rebuild parent/child connections using instance IDs
        let id_to_idx: std::collections::HashMap<String, usize> = self
            .sections
            .iter()
            .enumerate()
            .filter(|(_, b)| !b.is_palette)
            .map(|(i, b)| (b.instance_id.clone(), i))
            .collect();

        for (offset, snap) in snapshot.blocks.iter().enumerate() {
            if let Some(ref parent_iid) = snap.attached_to_instance_id {
                let child_idx = base_index + offset;
                if let Some(&parent_idx) = id_to_idx.get(parent_iid) {
                    self.sections[child_idx].attached_to = Some(parent_idx);
                    if !self.sections[parent_idx].children.contains(&child_idx) {
                        self.sections[parent_idx].children.push(child_idx);
                    }
                }
            }
        }

        self.workspace_name = snapshot.name;
        self.refresh_current_blocks();
        Ok(())
    }

    /// Closes the current workspace by removing all code blocks and clearing the workspace name.
    /// Palette blocks remain intact for creating new workspaces.
    pub fn close_workspace(&mut self) {
        self.sections.retain(|b| b.is_palette);
        for b in &mut self.sections {
            b.children.clear();
            b.attached_to = None;
        }
        self.workspace_name.clear();
        self.status_message.clear();
        self.refresh_current_blocks();
    }
}

/* ---------- UI ---------- */

/// Implementation of the egui App trait for rendering and handling the UI
impl eframe::App for BlocksForArduino {
    /// Main update loop called every frame - handles rendering and user input
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mouse_down = ctx.input(|i| i.pointer.primary_down());
        let mouse_released = self.was_mouse_down && !mouse_down;
        self.was_mouse_down = mouse_down;

        let mut delete_request: Option<usize> = None;

        // --- TOP TOOLBAR PANEL ---
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Workspace controls
                if ui.button("New").clicked() {
                    self.close_workspace();
                }
                if ui.button("Open").clicked() {
                    self.available_workspaces = Self::list_workspaces();
                    self.show_open_dialog = true;
                }
                if ui.button("Save").clicked() {
                    if self.workspace_name.is_empty() {
                        self.save_as_name_input.clear();
                        self.show_save_as_dialog = true;
                    } else {
                        let name = self.workspace_name.clone();
                        match self.save_workspace(&name) {
                            Ok(()) => self.status_message = format!("Saved \"{}\"", name),
                            Err(e) => self.status_message = e,
                        }
                    }
                }
                if ui.button("Save As").clicked() {
                    self.save_as_name_input = self.workspace_name.clone();
                    self.show_save_as_dialog = true;
                }
                if ui.add_enabled(!self.workspace_name.is_empty(), egui::Button::new("Reload")).clicked() {
                    let name = self.workspace_name.clone();
                    match self.load_workspace_by_name(&name) {
                        Ok(()) => self.status_message = format!("Reloaded \"{}\"", name),
                        Err(e) => self.status_message = e,
                    }
                }
                if ui.button("Close").clicked() {
                    self.close_workspace();
                }

                ui.separator();

                let name_label = if self.workspace_name.is_empty() {
                    "Untitled".to_string()
                } else {
                    self.workspace_name.clone()
                };
                ui.label(format!("Workspace: {}", name_label));

                ui.separator();

                // --- Validation ---
                if ui.button("Check connections").clicked() {
                    // Step 1 – validate field values before checking sequences
                    match self.validate_block_inputs() {
                        Err(message) => {
                            self.status_message =
                                "Invalid block inputs detected.".to_string();
                            self.validation_popup_text = message;
                            self.show_validation_popup = true;
                        }
                        Ok(()) => {
                            // Step 2 – validate cross-block pin consistency
                            match self.validate_pin_consistency() {
                                Err(message) => {
                                    self.status_message =
                                        "Pin mismatch detected.".to_string();
                                    self.validation_popup_text = message;
                                    self.show_validation_popup = true;
                                }
                                Ok(()) => {
                                    // Step 3 – validate block-order sequences
                                    if self.current_blocks.is_empty() {
                                        self.status_message =
                                            "No code-block connections found".to_string();
                                    } else {
                                        match self.validate_current_sequences() {
                                            Ok(()) => {
                                                let current_blocks = self.current_blocks_json();
                                                self.status_message = format!(
                                                    "Connected blocks are valid: {}",
                                                    current_blocks
                                                );
                                                println!(
                                                    "Connected blocks are valid: {}",
                                                    current_blocks
                                                );
                                                self.show_sequence_popup = false;
                                                self.sequence_popup_text.clear();
                                            }
                                            Err(message) => {
                                                self.status_message =
                                                    "Invalid block order detected".to_string();
                                                self.sequence_popup_text = message;
                                                self.show_sequence_popup = true;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if ui.button("Export .ino").clicked() {
                    match self.validate_block_inputs() {
                        Err(message) => {
                            self.status_message =
                                "Invalid block inputs detected.".to_string();
                            self.validation_popup_text = message;
                            self.show_validation_popup = true;
                        }
                        Ok(()) => match self.validate_pin_consistency() {
                            Err(message) => {
                                self.status_message =
                                    "Pin mismatch detected.".to_string();
                                self.validation_popup_text = message;
                                self.show_validation_popup = true;
                            }
                            Ok(()) => match self.export_ino() {
                                Ok(path) => {
                                    self.status_message = format!(
                                        "Exported Arduino sketch to {}",
                                        path.display()
                                    );
                                }
                                Err(err) => {
                                    self.status_message = err;
                                }
                            },
                        },
                    }
                }

                if !self.status_message.is_empty() {
                    ui.label(&self.status_message);
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // --- CENTRAL CANVAS ---
            // Renders the main editor area with palette blocks on left and code canvas on right
            let screen = ui.max_rect();
            let divider_x = screen.left() + 300.0;

            // ---- headings / divider / stats ---------------------------------
            {
                let painter = ui.painter();
                let total_blocks = self.sections.len();
                let palette_blocks = self.sections.iter().filter(|b| b.is_palette).count();
                let code_blocks = total_blocks - palette_blocks;

                let heading_y = screen.top() + 10.0;
                let stats_y = screen.top() + 35.0;
                let left_x = screen.left() + 10.0;
                let right_heading_x = divider_x + 20.0;

                painter.text(
                    egui::pos2(left_x, stats_y),
                    egui::Align2::LEFT_TOP,
                    format!(
                        "Total: {} | Palette: {} | Code: {}",
                        total_blocks, palette_blocks, code_blocks
                    ),
                    egui::TextStyle::Body.resolve(ui.style()),
                    ui.style().visuals.text_color(),
                );

                painter.text(
                    egui::pos2(left_x, heading_y),
                    egui::Align2::LEFT_TOP,
                    "Block section",
                    egui::TextStyle::Heading.resolve(ui.style()),
                    ui.style().visuals.text_color(),
                );

                painter.text(
                    egui::pos2(right_heading_x, heading_y),
                    egui::Align2::LEFT_TOP,
                    "Code section",
                    egui::TextStyle::Heading.resolve(ui.style()),
                    ui.style().visuals.text_color(),
                );

                painter.line_segment(
                    [
                        egui::pos2(divider_x, screen.top()),
                        egui::pos2(divider_x, screen.bottom()),
                    ],
                    egui::Stroke::new(
                        1.0,
                        ui.style().visuals.widgets.noninteractive.bg_stroke.color,
                    ),
                );
            }

            // ---- palette scroll handling ------------------------------------
            let palette_count = self.sections.iter().filter(|b| b.is_palette).count();
            let palette_slot_h = GLOBAL_Y + PALETTE_BLOCK_GAP;
            let palette_content_h = BLOCKS_START_Y + palette_count as f32 * palette_slot_h;
            let palette_visible_rect = egui::Rect::from_min_max(
                egui::pos2(screen.left(), screen.top()),
                egui::pos2(divider_x - 1.0, screen.bottom()),
            );
            let max_scroll = (palette_content_h - palette_visible_rect.height()).max(0.0);

            let scroll_delta_y = ctx.input(|i| {
                if i.pointer
                    .hover_pos()
                    .map_or(false, |p| palette_visible_rect.contains(p))
                {
                    i.smooth_scroll_delta.y
                } else {
                    0.0
                }
            });
            self.palette_scroll_offset =
                (self.palette_scroll_offset - scroll_delta_y).clamp(0.0, max_scroll);

            // ---- blocks -----------------------------------------------------
            let size = egui::vec2(GLOBAL_X, GLOBAL_Y);
            for i in 0..self.sections.len() {
                let is_palette = self.sections[i].is_palette;

                // For palette blocks apply the scroll offset to the render position.
                let render_pos = if is_palette {
                    let nat = self.sections[i].pos;
                    egui::pos2(nat.x, nat.y - self.palette_scroll_offset)
                } else {
                    self.sections[i].pos
                };

                // Skip palette blocks that are fully scrolled out of view.
                if is_palette
                    && (render_pos.y + size.y < palette_visible_rect.top()
                        || render_pos.y > palette_visible_rect.bottom())
                {
                    continue;
                }

                let rect = egui::Rect::from_min_size(render_pos, size);
                let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());

                if response.clicked() && is_palette {
                    self.spawn_code_block(i);
                }

                if response.drag_started() && !is_palette {
                    self.detach_block(i);
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

                // Clip palette blocks so they don't overdraw outside their column.
                let painter = if is_palette {
                    ui.painter().with_clip_rect(palette_visible_rect)
                } else {
                    ui.painter().clone()
                };

                let color = self.sections[i]
                    .block_colour
                    .as_ref()
                    .map(|c| parse_hex_colour(c))
                    .unwrap_or(egui::Color32::from_rgb(80, 160, 240));

                painter.rect_filled(rect, 6.0, color);

                let label = self.sections[i]
                    .shown_element
                    .clone()
                    .unwrap_or_else(|| self.sections[i].id.clone());

                painter.text(
                    render_pos + egui::vec2(10.0, 8.0),
                    egui::Align2::LEFT_TOP,
                    label,
                    egui::TextStyle::Body.resolve(ui.style()),
                    ui.style().visuals.text_color(),
                );

                let mut y_offset = 26.0;

                if is_palette {
                    // Palette blocks: render input names as static clipped text so they
                    // scroll correctly with the block background via the palette painter.
                    for input in &self.sections[i].inputs {
                        painter.text(
                            render_pos + egui::vec2(10.0, y_offset),
                            egui::Align2::LEFT_TOP,
                            &input.name,
                            egui::TextStyle::Small.resolve(ui.style()),
                            ui.style().visuals.text_color(),
                        );
                        y_offset += 18.0;
                    }
                } else {
                    let block = &mut self.sections[i];
                    for (key, value) in block.input_values.iter_mut() {
                        let input_rect = egui::Rect::from_min_size(
                            render_pos + egui::vec2(10.0, y_offset),
                            egui::vec2(100.0, 15.0),
                        );

                        ui.scope_builder(egui::UiBuilder::new().max_rect(input_rect), |ui| {
                            ui.add(egui::TextEdit::singleline(value).hint_text(key));
                        });

                        y_offset += 22.0;
                    }
                }
            }

            // ---- palette scrollbar indicator --------------------------------
            if max_scroll > 0.0 {
                let track_top = palette_visible_rect.top() + BLOCKS_START_Y;
                let track_bottom = palette_visible_rect.bottom() - 4.0;
                let track_h = (track_bottom - track_top).max(1.0);
                let thumb_h = ((palette_visible_rect.height() / palette_content_h) * track_h)
                    .clamp(20.0, track_h);
                let thumb_top = track_top
                    + (self.palette_scroll_offset / max_scroll) * (track_h - thumb_h);
                let bar_x = divider_x - 7.0;
                let thumb_rect = egui::Rect::from_min_size(
                    egui::pos2(bar_x, thumb_top),
                    egui::vec2(4.0, thumb_h),
                );
                ui.painter().rect_filled(
                    thumb_rect,
                    2.0,
                    egui::Color32::from_gray(140),
                );
            }
        });

        if let Some(idx) = delete_request {
            self.delete_block(idx);
        }

        if self.show_sequence_popup {
            let mut open = self.show_sequence_popup;
            let mut close_requested = false;

            egui::Window::new("Sequence suggestion")
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label(&self.sequence_popup_text);
                    ui.add_space(8.0);
                    if ui.button("Close").clicked() {
                        close_requested = true;
                    }
                });

            self.show_sequence_popup = open && !close_requested;
        }

        if self.show_validation_popup {
            let mut open = self.show_validation_popup;
            let mut close_requested = false;

            egui::Window::new("Input validation error")
                .collapsible(false)
                .resizable(true)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label(&self.validation_popup_text);
                    ui.add_space(8.0);
                    if ui.button("Close").clicked() {
                        close_requested = true;
                    }
                });

            self.show_validation_popup = open && !close_requested;
        }

        // --- Open workspace dialog ---
        if self.show_open_dialog {
            let mut open = self.show_open_dialog;
            let mut close_requested = false;
            let mut selected: Option<String> = None;

            egui::Window::new("Open Workspace")
                .collapsible(false)
                .resizable(true)
                .open(&mut open)
                .show(ctx, |ui| {
                    if self.available_workspaces.is_empty() {
                        ui.label("No saved workspaces found.");
                    } else {
                        egui::ScrollArea::vertical()
                            .max_height(300.0)
                            .show(ui, |ui| {
                                for name in &self.available_workspaces {
                                    if ui.button(name).clicked() {
                                        selected = Some(name.clone());
                                    }
                                }
                            });
                    }
                    ui.add_space(8.0);
                    if ui.button("Cancel").clicked() {
                        close_requested = true;
                    }
                });

            if let Some(name) = selected {
                match self.load_workspace_by_name(&name) {
                    Ok(()) => self.status_message = format!("Opened \"{}\"", name),
                    Err(e) => self.status_message = e,
                }
                close_requested = true;
            }

            self.show_open_dialog = open && !close_requested;
        }

        // --- Save As dialog ---
        if self.show_save_as_dialog {
            let mut open = self.show_save_as_dialog;
            let mut close_requested = false;
            let mut do_save: Option<String> = None;

            egui::Window::new("Save Workspace As")
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label("Workspace name:");
                    let resp = ui.text_edit_singleline(&mut self.save_as_name_input);
                    if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let trimmed = self.save_as_name_input.trim().to_string();
                        if !trimmed.is_empty() {
                            do_save = Some(trimmed);
                        }
                    }
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let can_save = !Self::safe_filename(&self.save_as_name_input).is_empty();
                        if ui.add_enabled(can_save, egui::Button::new("Save")).clicked() {
                            do_save = Some(Self::safe_filename(&self.save_as_name_input));
                        }
                        if ui.button("Cancel").clicked() {
                            close_requested = true;
                        }
                    });
                });

            if let Some(name) = do_save {
                match self.save_workspace(&name) {
                    Ok(()) => {
                        self.workspace_name = name.clone();
                        self.status_message = format!("Saved \"{}\"", name);
                        close_requested = true;
                    }
                    Err(e) => self.status_message = e,
                }
            }

            self.show_save_as_dialog = open && !close_requested;
        }
    }
}