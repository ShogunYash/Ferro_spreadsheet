use std::cmp::min;
use std::collections::HashMap;
use std::collections::HashSet;
// Spreadsheet implementation
use crate::cell::{parse_cell_reference, Cell, CellValue}; 

// Constants
const MAX_ROWS: i16 = 999;    // Example value, adjust as needed
const MAX_COLS: i16 = 18278;  // Example value, adjust as needed

#[derive(Debug, PartialEq)]
pub enum CommandStatus {
    CmdOk,
    CmdUnrecognized,
    CmdCircularRef,
    CmdInvalidCell,
}

// Modified CellMeta to remove children (they're now stored separately)
#[derive(Debug, Clone)]
pub struct CellMeta {
    pub formula: i16,
    pub parent1: i32,
    pub parent2: i32,
}

impl CellMeta {
    pub fn new() -> Self {
        CellMeta {
            formula: -1,
            parent1: -1,
            parent2: -1,
        }
    }
}

// Structure to track range dependencies
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RangeDependency {
    pub start_row: i16,
    pub start_col: i16,
    pub end_row: i16,
    pub end_col: i16,
}

// Spreadsheet structure with separate cell_meta and cell_children maps
pub struct Spreadsheet {
    pub grid: Vec<Cell>,                           // Vector of Cells (contiguous in memory)
    pub cell_meta: HashMap<i32, CellMeta>,         // Map from cell key to metadata
    pub cell_children: HashMap<i32, HashSet<i32>>, // Map from cell key to its children
    pub rows: i16,
    pub cols: i16,
    viewport_row: i16,
    viewport_col: i16,
    pub output_enabled: bool,
}

impl Spreadsheet {
    // Create a new spreadsheet with specified dimensions
    pub fn create(rows: i16, cols: i16) -> Option<Spreadsheet> {
        if rows < 1 || rows > MAX_ROWS || cols < 1 || cols > MAX_COLS {
            eprintln!("Invalid spreadsheet dimensions");
            return None;
        }
        
        // Create empty cells
        let total = rows as usize * cols as usize;
                let grid = vec![Cell::new(); total];
                
        Some(Spreadsheet {
            grid,
            cell_meta: HashMap::with_capacity(4),
            cell_children: HashMap::with_capacity(32),
            rows,
            cols,
            viewport_row: 0,
            viewport_col: 0,
            output_enabled: true,
        })
    }

    // Helper to get cell key from coordinates
    pub fn get_key(&self, row: i16, col: i16) -> i32 {
        (row as i32 * self.cols as i32 + col as i32) as i32
    }
    
    // Helper to get coordinates from cell key
    pub fn get_row_col(&self, key: i32) -> (i16, i16) {
        let row = (key / (self.cols as i32)) as i16;
        let col = (key % (self.cols as i32)) as i16;
        (row, col)
    }
    
    // Get cell metadata, creating it if it doesn't exist
    pub fn get_cell_meta(&mut self, row: i16, col: i16) -> &mut CellMeta {
        let key = self.get_key(row, col);
        self.cell_meta.entry(key).or_insert_with(CellMeta::new)
    }
    
    // Get children HashSet for a cell, creating it if it doesn't exist
    pub fn get_children(&mut self, key: i32) -> &mut HashSet<i32> {
        self.cell_children.entry(key).or_insert_with(|| HashSet::with_capacity(4))
    }
      
    // Get children for a cell (immutable)
    pub fn get_cell_children(&self, key: i32) -> Option<&HashSet<i32>> {
        self.cell_children.get(&key)
    }

    pub fn get_column_name(&self, mut col: i16) -> String {
        // Pre-calculate the length needed for the string
        let mut temp_col = col + 1; // Convert from 0-based to 1-based
        let mut len = 0;
        while temp_col > 0 {
            len += 1;
            temp_col = (temp_col - 1) / 26;
        }
             
        // Add column letters directly in reverse order
        col += 1; // Convert from 0-based to 1-based
        
        // Handle special case for col = 0
        if col == 0 {
            return "A".to_string();
        }
        
        // Create a buffer of bytes to avoid repeated string operations
        let mut buffer = vec![0; len];
        let mut i = len;
        
        while col > 0 {
            i -= 1;
            buffer[i] = b'A' + ((col - 1) % 26) as u8;
            col = (col - 1) / 26;
        }
        
        // Convert the byte buffer to a string in one operation
        unsafe {
            // This is safe because we know our bytes are valid ASCII from b'A' to b'Z'
            String::from_utf8_unchecked(buffer)
        }
    }
    // pub fn get_column_name(&self, mut col: i16) -> String {
    //     let mut name = String::new();
    //     col += 1; // Convert from 0-based to 1-based
    //     while col > 0 {
    //         name.push((b'A' + ((col - 1) % 26) as u8) as char); // Convert to character
    //         col = (col - 1) / 26;
    //     }
    //     name.chars().rev().collect() // Reverse the string to get the correct column name
    // }

    pub fn column_name_to_index(&self, name: &str) -> i16 {
        let bytes = name.as_bytes();
        let mut index: i16 = 0;
        for &b in bytes {
                        index = index * 26 + ((b - b'A') as i16 + 1);
        }
        index - 1 // Convert from 1-based to 0-based
    }
    // pub fn column_name_to_index(&self, name: &str) -> i16 {
    //     let mut index: i16 = 0;
    //     for char in name.chars() {
    //         index *= 26;
    //         index += (char.to_ascii_uppercase() as i16) - ('A' as i16) + 1; // Convert character to index
    //     }
    //     index - 1 // Convert from 1-based to 0-based
    // }

    pub fn get_cell(&self, row: i16, col: i16) -> &Cell {
        let index = (row as usize) * (self.cols as usize) + (col as usize);    
        &self.grid[index]
    }
    
    pub fn get_mut_cell(&mut self, row: i16, col: i16) -> &mut Cell {
        let index = (row as usize) * (self.cols as usize) + (col as usize);
        &mut self.grid[index]
    }
    
    // Add a child to a cell's dependents (modified for separate children HashMap)
    pub fn add_child(&mut self, parent_row: i16, parent_col: i16, child_row: i16, child_col: i16) {
        let parent_key = self.get_key(parent_row, parent_col);
        let child_key = self.get_key(child_row, child_col);
        
        let children = self.get_children(parent_key);
        children.insert(child_key);
    }
    
    // Remove a child from a cell's dependents (modified for separate children HashMap)
    pub fn remove_child(&mut self, parent_key: i32, child_key: i32) {
        if let Some(children) = self.cell_children.get_mut(&parent_key) {
            children.remove(&child_key);
    
            // If no children left, remove the entry to save memory
            if children.is_empty() {
                self.cell_children.remove(&parent_key);
            }
        }
    }

    pub fn print_spreadsheet(&self) {
        if !self.output_enabled {
            return;
        }
        
        let start_row = self.viewport_row;
        let start_col = self.viewport_col;
        let display_row = min(self.rows - start_row, 10); // Display only a portion of the spreadsheet
        let display_col = min(self.cols - start_col, 10);
        
        // Print column headers
        print!("     ");
        for i in 0..display_col {
            print!("{:<8} ", self.get_column_name(start_col + i));
        }
        println!();
        
        // Print rows with data
        for i in 0..display_row {
            print!("{:<4} ", start_row + i + 1); // Show 1-based row numbers
            for j in 0..display_col {
                let cell = self.get_cell(start_row + i, start_col + j); 
                    match cell.value {
                        CellValue::Integer(value) => print!("{:<8} ", value),
                        CellValue::Error => print!("{:<8} ", "ERR"),
                    }
            }
            println!();
        }
    }

    pub fn scroll_to_cell(&mut self, cell: &str) -> CommandStatus {
        match parse_cell_reference(self, cell) {
            Ok((row, col)) => {
                if row >= 0 && row < self.rows && col >= 0 && col < self.cols {
                    self.viewport_row = row;
                    self.viewport_col = col;
                    return CommandStatus::CmdOk;
                } else {
                    return CommandStatus::CmdInvalidCell;
                }
            }
            Err(_) => {
                return CommandStatus::CmdUnrecognized;
            }
        }
    }

    pub fn scroll_viewport(&mut self, direction: char) {
        const VIEWPORT_SIZE: i16 = 10;
        match direction {
            'w' => {
                self.viewport_row = if self.viewport_row > 10 {
                    self.viewport_row - 10
                } else {
                    0
                };
            }
            's' => {
                if self.viewport_row + VIEWPORT_SIZE < self.rows {
                    self.viewport_row += 10;
                } else {
                    self.viewport_row = self.rows - VIEWPORT_SIZE;
                }
            }
            'a' => {
                self.viewport_col = if self.viewport_col > 10 {
                    self.viewport_col - 10
                } else {
                    0
                };
            }

            'd' => {
                if self.viewport_col + VIEWPORT_SIZE < self.cols {
                    self.viewport_col += 10;
                } else {
                    self.viewport_col = self.cols - VIEWPORT_SIZE;
                }
            }
            _ => {} // Invalid direction, do nothing
        }
    }
      
}