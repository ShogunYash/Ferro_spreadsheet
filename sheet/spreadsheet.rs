use std::cmp::min;
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
    CmdInvalidRange,
    CmdRangeerror,
}

// Spreadsheet structure now uses a contiguous array for grid
pub struct Spreadsheet {
    pub grid: Vec<Cell>,         // Vector of Cells (contiguous in memory)
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
        let mut grid = Vec::with_capacity(total);
        
        // Initialize each cell
        for _ in 0..total {
            grid.push(Cell::new());
        }
        
        Some(Spreadsheet {
            grid,
            rows,
            cols,
            viewport_row: 0,
            viewport_col: 0,
             output_enabled: true,
        })
    }

    pub fn get_column_name(&self, mut col: i16) -> String {
        let mut name = String::new();
        col += 1; // Convert from 0-based to 1-based
        while col > 0{
            name.push((b'A' + ((col - 1) % 26) as u8) as char); // Convert to character
            col = (col - 1) / 26;
        }
        name.chars().rev().collect() // Reverse the string to get the correct column name
    }

    pub fn column_name_to_index(&self, name: &str) -> i16 {
        let mut index: i16 = 0;
        for char in name.chars(){
            index *= 26;
            index += (char.to_ascii_uppercase() as i16) - ('A' as i16) + 1; // Convert character to index
        }
        index - 1 // Convert from 1-based to 0-based
    }

    pub fn get_cell(&self, row: i16, col: i16) -> Option<&Cell> {
        if row < 0 || row >= self.rows || col < 0 || col >= self.cols {
            return None;
        }
        
        let index = (row as usize) * (self.cols as usize) + (col as usize);
        if index >= self.grid.len() {
            return None;
        }
        
        Some(&self.grid[index])
    }
    
    pub fn get_mut_cell(&mut self, row: i16, col: i16) -> &mut Cell {
        
        let index = (row as usize) * (self.cols as usize) + (col as usize);
        
        &mut self.grid[index]
    }

    pub fn print_spreadsheet(&self){
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
                if let Some(cell) = self.get_cell(start_row + i, start_col + j) {
                    match cell.value {
                        CellValue::Integer(value) => print!("{:<8} ", value),
                        CellValue::Error => print!("{:<8} ", "ERR"),
                    }
                } else {
                    print!("{:<8} ", "???"); // Indicate an access error
                }
            }
            println!();
        }
    }

    pub fn scroll_to_cell(&mut self, cell: &str) -> CommandStatus {
        // add or give cell expr
        match parse_cell_reference(self, cell) {
            Ok((row, col)) => {
                if row>=0 && row < self.rows && col >= 0 && col < self.cols {
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
                'w'=> {
                    self.viewport_row = if self.viewport_row > 10 {
                        self.viewport_row - 10
                    } else {
                        0
                    };
                }
                's'=> {
                    if self.viewport_row + VIEWPORT_SIZE < self.rows {
                        self.viewport_row += 10;
                    } else {
                        self.viewport_row = self.rows - VIEWPORT_SIZE;
                    }
                }
                'a'=> {
                    self.viewport_col = if self.viewport_col > 10 {
                        self.viewport_col - 10
                    } else {
                        0
                    };
                }

                'd'=> {
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