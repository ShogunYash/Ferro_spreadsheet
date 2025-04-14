use crate::spreadsheet::{Spreadsheet, CommandStatus}; // Importing Spreadsheet and CommandStatus from spreadsheet module
use std::collections::{HashMap, HashSet};

// Cell value representation
#[derive(Debug, Clone, PartialEq)]
pub enum CellValue {
    Integer(i32),
    Error,
}

// Cell structure redesigned to use hashmaps instead of linked lists
pub struct Cell {
    pub value: CellValue,
}

// Global hashmaps for storing relationships between cells
// These could be part of the Spreadsheet struct, but separating for clarity
pub struct CellRelationships {
    pub parents: HashMap<i32, Vec<i32>>,      // cell_key -> parent_keys
    pub children: HashMap<i32, HashSet<i32>>, // cell_key -> child_keys
    pub formulas: HashMap<i32, i16>,          // cell_key -> formula_code
}

impl CellRelationships {
    pub fn new() -> Self {
        CellRelationships {
            parents: HashMap::new(),
            children: HashMap::new(),
            formulas: HashMap::new(),
        }
    }
    
    pub fn add_parent(&mut self, cell_key: i32, parent_key: i32) {
        self.parents.entry(cell_key)
            .or_insert_with(Vec::new)
            .push(parent_key);
    }
    
    pub fn add_child(&mut self, parent_key: i32, child_key: i32) {
        self.children.entry(parent_key)
            .or_insert_with(HashSet::new)
            .insert(child_key);
    }
    
    pub fn set_formula(&mut self, cell_key: i32, formula: i16) {
        self.formulas.insert(cell_key, formula);
    }
    
    pub fn get_formula(&self, cell_key: i32) -> i16 {
        *self.formulas.get(&cell_key).unwrap_or(&-1)
    }
    
    pub fn get_parents(&self, cell_key: i32) -> Vec<i32> {
        self.parents.get(&cell_key).cloned().unwrap_or_default()
    }
    
    pub fn get_children(&self, cell_key: i32) -> HashSet<i32> {
        self.children.get(&cell_key).cloned().unwrap_or_default()
    }
    
    pub fn remove_all_parents(&mut self, cell_key: i32) {
        // Get current parents
        if let Some(parents) = self.parents.get(&cell_key).cloned() {
            // Remove this cell as a child from all its parents
            for parent_key in parents {
                if let Some(children) = self.children.get_mut(&parent_key) {
                    children.remove(&cell_key);
                }
            }
        }
        // Clear this cell's parents
        self.parents.remove(&cell_key);
    }
}

impl Cell {
    pub fn new() -> Self {
        Cell {
            value: CellValue::Integer(0),
        }
    }
}

pub fn parse_cell_reference(sheet: &Spreadsheet, cell_ref: &str) -> Result<(i16, i16), CommandStatus> {
    // Extract column letters
    let mut i = 0;
    let mut col_name = String::new();
    
    for c in cell_ref.chars() {
        if c.is_ascii_uppercase() {
            if i >= 3 {
                return Err(CommandStatus::CmdUnrecognized);
            }
            col_name.push(c);
            i += 1;
        } else {
            break;
        }
    }
    
    // Make sure we have at least one letter and digits follow
    if col_name.is_empty() || i >= cell_ref.len() {
        return Err(CommandStatus::CmdUnrecognized);
    }
    
    // Extract row number
    let row_str = &cell_ref[i..];
    
    if row_str.is_empty() {
        return Err(CommandStatus::CmdUnrecognized);
    }
    
    // Validate that all remaining characters are digits
    if !row_str.chars().all(|c| c.is_ascii_digit()) {
        return Err(CommandStatus::CmdUnrecognized);
    }
    
    // Parse row number (convert to 0-based)
    let row = match row_str.parse::<i16>() {
        Ok(r) => r - 1,
        Err(_) => return Err(CommandStatus::CmdUnrecognized)
    };
    
    // Convert column name to column index
    let col = sheet.column_name_to_index(&col_name);
  
    Ok((row, col))
}
