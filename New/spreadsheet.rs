use std::cmp::min;
use std::collections::{HashMap, HashSet, VecDeque};
// Spreadsheet implementation
use crate::cell::{Cell, CellValue, parse_cell_reference, CellRelationships}; 
use crate::evaluator::{get_key, get_cell_value, evaluate_formula, recalculate_cell};

// Constants
const MAX_ROWS: i16 = 999;    // Example value, adjust as needed
const MAX_COLS: i16 = 18278;  // Example value, adjust as needed

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum CommandStatus {
    CmdOk,
    CmdUnrecognized,
    CmdCircularRef,
    CmdInvalidCell,
}

// Spreadsheet structure now includes CellRelationships

pub struct Spreadsheet {
    pub grid: Vec<Cell>,         // Vector of Cells (contiguous in memory)
    pub rows: i16,
    pub cols: i16,
    viewport_row: i16,
    viewport_col: i16,
    pub output_enabled: bool,
    pub relationships: CellRelationships, // Added relationships field
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
            relationships: CellRelationships::new(),
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

    pub fn get_cell(&self, row: i16, col: i16) -> &Cell {
        let index = (row as usize) * (self.cols as usize) + (col as usize);    
        &self.grid[index]
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
    
    // New function for topological re-evaluation
    pub fn re_evaluate_topological(&mut self, cell_key: i32, expr: &str, sleep_time: &mut f64) -> CommandStatus {
        // Store old state for reverting in case of circular reference
        let old_parents = self.relationships.get_parents(cell_key);
        let old_formula = self.relationships.get_formula(cell_key);
        let old_value = {
            let row = (cell_key / self.cols as i32) as i16;
            let col = (cell_key % self.cols as i32) as i16;
            self.get_cell(row, col).value.clone()
        };
        
        // Remove old parents
        self.relationships.remove_all_parents(cell_key);
        
        // Calculate row and col from cell_key
        let row = (cell_key / self.cols as i32) as i16;
        let col = (cell_key % self.cols as i32) as i16;
        
        // Evaluate new formula
        let status = evaluate_formula(self, row, col, expr, sleep_time);
        
        if status == CommandStatus::CmdOk {
            // Check for circular references
            if self.detect_cycle(cell_key) {
                // Revert changes if a cycle is detected
                self.relationships.remove_all_parents(cell_key);
                
                // Restore old parents
                for parent_key in old_parents {
                    self.relationships.add_parent(cell_key, parent_key);
                    self.relationships.add_child(parent_key, cell_key);
                }
                
                // Restore old formula and value
                if old_formula != -1 {
                    self.relationships.set_formula(cell_key, old_formula);
                }
                
                let cell = {
                    let row = (cell_key / self.cols as i32) as i16;
                    let col = (cell_key % self.cols as i32) as i16;
                    self.get_mut_cell(row, col)
                };
                cell.value = old_value;
                
                return CommandStatus::CmdCircularRef;
            }
            
            // If no cycles, propagate changes to all dependent cells
            self.update_dependent_cells(cell_key, sleep_time);
        }
        
        status
    }
    
    // Function to detect cycles in the dependency graph
    fn detect_cycle(&self, start_key: i32) -> bool {
        // Map to track visited nodes
        let mut visited = HashMap::new();
        let mut path = HashSet::new();
        
        fn dfs(
            sheet: &Spreadsheet, 
            current: i32,
            visited: &mut HashMap<i32, bool>,
            path: &mut HashSet<i32>,
        ) -> bool {
            // Mark current node as being visited in current path
            visited.insert(current, true);
            path.insert(current);
            
            // Visit all children
            let children = sheet.relationships.get_children(current);
            for &child in &children {
                // If child is not visited, recursively check
                if !visited.contains_key(&child) {
                    if dfs(sheet, child, visited, path) {
                        return true;
                    }
                } 
                // If child is in current path, we have a cycle
                else if path.contains(&child) {
                    return true;
                }
            }
            
            // Remove current node from current path
            path.remove(&current);
            false
        }
        
        dfs(self, start_key, &mut visited, &mut path)
    }
    
    // Function to update all cells dependent on a changed cell
    fn update_dependent_cells(&mut self, start_key: i32, sleep_time: &mut f64) {
        // Use a queue for breadth-first traversal of dependencies
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        
        // Add all immediate children to the queue
        for &child in &self.relationships.get_children(start_key) {
            queue.push_back(child);
            visited.insert(child);
        }
        
        // Process queue until empty
        while let Some(key) = queue.pop_front() {
            // Calculate row and col for this cell
            let row = (key / self.cols as i32) as i16;
            let col = (key % self.cols as i32) as i16;
            
            // Recalculate this cell's value
            recalculate_cell(self, row, col, sleep_time);
            
            // Add all unvisited children to the queue
            for &child in &self.relationships.get_children(key) {
                if !visited.contains(&child) {
                    queue.push_back(child);
                    visited.insert(child);
                }
            }
        }
    }
}
