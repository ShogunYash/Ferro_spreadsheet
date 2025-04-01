use std::collections::{HashSet, VecDeque};
use std::time::Duration;
use std::thread;

use crate::cell::{Cell, CellValue};
use crate::formula;

pub struct Spreadsheet {
    cells: Vec<Vec<Cell>>,
    rows: usize,
    cols: usize,
    view_row: usize, // Top-left cell's row for viewing
    view_col: usize, // Top-left cell's column for viewing
}

impl Spreadsheet {
    pub fn new(rows: usize, cols: usize) -> Self {
        let mut cells = Vec::with_capacity(rows);
        for _ in 0..rows {
            let mut row = Vec::with_capacity(cols);
            for _ in 0..cols {
                row.push(Cell::new());
            }
            cells.push(row);
        }
        
        Spreadsheet {
            cells,
            rows,
            cols,
            view_row: 0,
            view_col: 0,
        }
    }
    
    pub fn get_rows(&self) -> usize {
        self.rows
    }
    
    pub fn get_cols(&self) -> usize {
        self.cols
    }
    
    pub fn get_view_row(&self) -> usize {
        self.view_row
    }
    
    pub fn get_view_col(&self) -> usize {
        self.view_col
    }
    
    pub fn get_cell(&self, row: usize, col: usize) -> Option<&Cell> {
        if row < self.rows && col < self.cols {
            Some(&self.cells[row][col])
        } else {
            None
        }
    }
    
    // Converts a cell reference like "A1" to (row, col) indices (0-based)
    pub fn parse_cell_reference(&self, cell_ref: &str) -> Result<(usize, usize), String> {
        let mut col_str = String::new();
        let mut row_str = String::new();
        
        for c in cell_ref.chars() {
            if c.is_alphabetic() {
                col_str.push(c.to_ascii_uppercase());
            } else if c.is_numeric() {
                row_str.push(c);
            } else {
                return Err(format!("Invalid cell reference: {}", cell_ref));
            }
        }
        
        if col_str.is_empty() || row_str.is_empty() {
            return Err(format!("Invalid cell reference: {}", cell_ref));
        }
        
        // Parse row (1-based in input, 0-based internally)
        let row = match row_str.parse::<usize>() {
            Ok(r) => {
                if r < 1 || r > self.rows {
                    return Err(format!("Row out of range: {}", r));
                }
                r - 1 // Convert to 0-based
            }
            Err(_) => return Err(format!("Invalid row: {}", row_str)),
        };
        
        // Parse column (A-ZZZ to 0-based index)
        let mut col = 0;
        for c in col_str.chars() {
            if !c.is_ascii_uppercase() {
                return Err(format!("Invalid column character: {}", c));
            }
            col = col * 26 + (c as usize - 'A' as usize + 1);
        }
        col -= 1; // Convert to 0-based
        
        if col >= self.cols {
            return Err(format!("Column out of range: {}", col_str));
        }
        
        Ok((row, col))
    }
    
    // Converts (row, col) indices to a cell reference like "A1"
    pub fn format_cell_reference(&self, row: usize, col: usize) -> String {
        let mut col_str = String::new();
        let mut col_val = col + 1; // Convert to 1-based for calculation
        
        while col_val > 0 {
            let remainder = (col_val - 1) % 26;
            col_str.insert(0, (b'A' + remainder as u8) as char);
            col_val = (col_val - remainder) / 26;
            if col_val == 1 {
                break;
            }
        }
        
        format!("{}{}", col_str, row + 1) // Convert row to 1-based
    }
    
    pub fn process_command(&mut self, command: &str) -> Result<(), String> {
        // Check if it's a formula assignment: Cell=Expression
        if let Some((cell_ref, expression)) = command.split_once('=') {
            let cell_ref = cell_ref.trim();
            let expression = expression.trim();
            
            // Parse the cell reference
            let (row, col) = self.parse_cell_reference(cell_ref)?;
            
            // Update cell value and evaluate
            self.set_cell_formula(row, col, expression)?;
            
            Ok(())
        } else {
            Err("unrecognized cmd".to_string())
        }
    }
    
    fn set_cell_formula(&mut self, row: usize, col: usize, formula: &str) -> Result<(), String> {
        // First, remove this cell from its dependencies' dependents lists
        let deps = self.cells[row][col].dependencies.clone();
        for (dep_row, dep_col) in deps {
            if dep_row < self.rows && dep_col < self.cols {
                self.cells[dep_row][dep_col].clear_dependent(row, col);
            }
        }
        
        // Clear existing dependencies
        self.cells[row][col].clear_dependencies();
        
        // Try to parse as a simple integer
        if let Ok(value) = formula.parse::<i32>() {
            self.cells[row][col].set_value(CellValue::Integer(value));
        } else {
            // It's a formula
            self.cells[row][col].set_value(CellValue::Formula(formula.to_string()));
            
            // Store the original formula for reference in dependencies
            let formula_str = formula.to_string();
            
            // Try to evaluate it immediately
            match self.evaluate_formula(row, col, &formula_str) {
                Ok(value) => {
                    self.cells[row][col].set_display_value(value);
                }
                Err(e) => {
                    self.cells[row][col].set_display_value(CellValue::Error(e));
                }
            }
        }
        
        // Recalculate all cells that depend on this one
        self.recalculate_dependents(row, col)
    }
    
    fn recalculate_dependents(&mut self, row: usize, col: usize) -> Result<(), String> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        
        // Initial cells to update
        for &(dep_row, dep_col) in &self.cells[row][col].dependents.clone() {
            queue.push_back((dep_row, dep_col));
            visited.insert((dep_row, dep_col));
        }
        
        while let Some((curr_row, curr_col)) = queue.pop_front() {
            // Get the formula of this cell
            if let Some(formula) = self.cells[curr_row][curr_col].value.get_formula() {
                // Evaluate the formula
                match self.evaluate_formula(curr_row, curr_col, &formula) {
                    Ok(value) => {
                        self.cells[curr_row][curr_col].set_display_value(value);
                    }
                    Err(e) => {
                        self.cells[curr_row][curr_col].set_display_value(CellValue::Error(e));
                    }
                }
                
                // Add its dependents to the queue
                for &(dep_row, dep_col) in &self.cells[curr_row][curr_col].dependents {
                    if !visited.contains(&(dep_row, dep_col)) {
                        queue.push_back((dep_row, dep_col));
                        visited.insert((dep_row, dep_col));
                    }
                }
            }
        }
        
        Ok(())
    }
    
    fn evaluate_formula(&mut self, row: usize, col: usize, formula: &str) -> Result<CellValue, String> {
        // This is a placeholder - we'll need to implement the formula parser/evaluator
        formula::evaluate(self, row, col, formula)
    }
    
    // Scrolling functions
    pub fn scroll_up(&mut self) {
        if self.view_row >= 10 {
            self.view_row -= 10;
        } else {
            self.view_row = 0;
        }
    }
    
    pub fn scroll_down(&mut self) {
        if self.view_row + 10 < self.rows {
            self.view_row += 10;
        }
    }
    
    pub fn scroll_left(&mut self) {
        if self.view_col >= 10 {
            self.view_col -= 10;
        } else {
            self.view_col = 0;
        }
    }
    
    pub fn scroll_right(&mut self) {
        if self.view_col + 10 < self.cols {
            self.view_col += 10;
        }
    }
    
    pub fn scroll_to(&mut self, cell_ref: &str) -> Result<(), String> {
        let (row, col) = self.parse_cell_reference(cell_ref)?;
        self.view_row = row;
        self.view_col = col;
        Ok(())
    }
    
    // Helper for formula evaluation
    pub fn get_cell_value(&self, row: usize, col: usize) -> Result<CellValue, String> {
        if row >= self.rows || col >= self.cols {
            return Err(format!("Cell reference out of bounds: {}", self.format_cell_reference(row, col)));
        }
        
        Ok(self.cells[row][col].display_value.clone())
    }
    
    // Register a dependency from one cell to another
    pub fn register_dependency(&mut self, from_row: usize, from_col: usize, to_row: usize, to_col: usize) {
        if from_row < self.rows && from_col < self.cols && to_row < self.rows && to_col < self.cols {
            // Add the dependency (from depends on to)
            self.cells[from_row][from_col].add_dependency(to_row, to_col);
            
            // Add the dependent (to is depended on by from)
            self.cells[to_row][to_col].add_dependent(from_row, from_col);
        }
    }
    
    // For SLEEP function
    pub fn sleep(&self, seconds: i32) -> i32 {
        let duration = if seconds > 0 {
            Duration::from_secs(seconds as u64)
        } else {
            Duration::from_secs(0)
        };
        
        thread::sleep(duration);
        seconds
    }
}