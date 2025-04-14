use std::cmp::min;
use std::collections::HashMap;
use std::collections::HashSet;
// Spreadsheet implementation
use crate::cell::{CellValue, parse_cell_reference}; 

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

// Spreadsheet structure with separate cell_meta and Vec of HashSets for children
// Changed grid to Vec<CellValue> from Vec<Cell>
pub struct Spreadsheet {
    pub grid: Vec<CellValue>,                      // Vector of CellValues (contiguous in memory)
    pub cell_meta: HashMap<i32, CellMeta>,         // Map from cell key to metadata
    pub children: Vec<HashSet<i32>>,               // Vector of HashSets for children (one per cell)
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
        
        // Create empty cells - initialize with Integer(0)
        let total = rows as usize * cols as usize;
        let grid = vec![CellValue::Integer(0); total];
        
        // Create empty HashSet for each cell
        let children = vec![HashSet::with_capacity(4); total];
                
        Some(Spreadsheet {
            grid,
            cell_meta: HashMap::with_capacity(32),
            children,
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
    
    // Helper to get index from key
    fn key_to_index(&self, key: i32) -> usize {
        key as usize
    }

    // Helper to get index from row and column
    fn get_index(&self, row: i16, col: i16) -> usize {
        (row as usize) * (self.cols as usize) + (col as usize)
    }
    
    // Get cell metadata, creating it if it doesn't exist
    pub fn get_cell_meta(&mut self, row: i16, col: i16) -> &mut CellMeta {
        let key = self.get_key(row, col);
        self.cell_meta.entry(key).or_insert_with(CellMeta::new)
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

    pub fn column_name_to_index(&self, name: &str) -> i16 {
        let bytes = name.as_bytes();
        let mut index: i16 = 0;
        for &b in bytes {
                        index = index * 26 + ((b - b'A') as i16 + 1);
        }
        index - 1 // Convert from 1-based to 0-based
    }

    pub fn get_cell(&self, row: i16, col: i16) -> &CellValue {
        let index = self.get_index(row, col);
        &self.grid[index]
    }
    
    pub fn get_mut_cell(&mut self, row: i16, col: i16) -> &mut CellValue {
        let index = self.get_index(row, col);
        &mut self.grid[index]
    }
    
    // Add a child to a cell's dependents (modified for Vec of HashSets)
    pub fn add_child(&mut self, parent_key: &i32, child_key: &i32) {
        let parent_index = self.key_to_index(*parent_key);
        self.children[parent_index].insert(*child_key);
    }
    
    // Remove a child from a cell's dependents (modified for Vec of HashSets)
    pub fn remove_child(&mut self, parent_key: i32, child_key: i32) {
        let parent_index = self.key_to_index(parent_key);
        self.children[parent_index].remove(&child_key);
    }

    // Get children HashSet for a cell
    // pub fn get_children(&mut self, key: i32) -> &mut HashSet<i32> {
    //     let index = self.key_to_index(key);
    //     &mut self.children[index]
    // }
      
    // Get children for a cell (immutable)
    pub fn get_cell_children(&self, key: i32) -> Option<&HashSet<i32>> {
        let index = self.key_to_index(key);
        let children = &self.children[index];
        if children.is_empty() {
            None
        } else {
            Some(children)
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
                let cell_value = self.get_cell(start_row + i, start_col + j); 
                match cell_value {
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_valid_dimensions() {
        let sheet = Spreadsheet::create(5, 5).unwrap();
        assert_eq!(sheet.rows, 5);
        assert_eq!(sheet.cols, 5);
        assert_eq!(sheet.grid.len(), 25);
        assert_eq!(sheet.viewport_row, 0);
        assert_eq!(sheet.viewport_col, 0);
    }

    #[test]
    fn test_create_invalid_dimensions() {
        assert!(Spreadsheet::create(0, 5).is_none());
        assert!(Spreadsheet::create(5, 0).is_none());
        assert!(Spreadsheet::create(MAX_ROWS + 1, 5).is_none());
        assert!(Spreadsheet::create(5, MAX_COLS + 1).is_none());
    }

    #[test]
    fn test_get_column_name() {
        let sheet = Spreadsheet::create(1, 1).unwrap();
        assert_eq!(sheet.get_column_name(0), "A");
        assert_eq!(sheet.get_column_name(25), "Z");
        assert_eq!(sheet.get_column_name(26), "AA");
        assert_eq!(sheet.get_column_name(51), "AZ");
    }

    #[test]
    fn test_column_name_to_index() {
        let sheet = Spreadsheet::create(1, 1).unwrap();
        assert_eq!(sheet.column_name_to_index("A"), 0);
        assert_eq!(sheet.column_name_to_index("Z"), 25);
        assert_eq!(sheet.column_name_to_index("AA"), 26);
        assert_eq!(sheet.column_name_to_index("AZ"), 51);
    }

    #[test]
    fn test_get_cell_and_get_mut_cell() {
        let mut sheet = Spreadsheet::create(2, 2).unwrap();
        let cell_value = sheet.get_mut_cell(0, 0);
        *cell_value = CellValue::Integer(42);
        assert_eq!(*sheet.get_cell(0, 0), CellValue::Integer(42));
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(0));
    }

    #[test]
    fn test_scroll_to_cell_valid() {
        let mut sheet = Spreadsheet::create(20, 20).unwrap();
        let status = sheet.scroll_to_cell("B2");
        assert_eq!(status, CommandStatus::CmdOk);
        assert_eq!(sheet.viewport_row, 1);
        assert_eq!(sheet.viewport_col, 1);
    }

    #[test]
    fn test_scroll_to_cell_invalid() {
        let mut sheet = Spreadsheet::create(5, 5).unwrap();
        assert_eq!(sheet.scroll_to_cell("F6"), CommandStatus::CmdInvalidCell);
        assert_eq!(sheet.scroll_to_cell("1A"), CommandStatus::CmdUnrecognized);
    }

    #[test]
    fn test_scroll_viewport() {
        let mut sheet = Spreadsheet::create(50, 50).unwrap();
        sheet.scroll_viewport('s');
        assert_eq!(sheet.viewport_row, 10);
        sheet.scroll_viewport('d');
        assert_eq!(sheet.viewport_col, 10);
        sheet.scroll_viewport('w');
        assert_eq!(sheet.viewport_row, 0);
        sheet.scroll_viewport('a');
        assert_eq!(sheet.viewport_col, 0);
        // Test boundaries
        sheet.viewport_row = 45;
        sheet.scroll_viewport('s');
        assert_eq!(sheet.viewport_row, 40);
    }

    #[test]
    fn test_print_spreadsheet_disabled() {
        let mut sheet = Spreadsheet::create(5, 5).unwrap();
        sheet.output_enabled = false;
        sheet.print_spreadsheet(); // Should not panic
    }
}