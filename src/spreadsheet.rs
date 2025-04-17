use std::cmp::min;
use std::collections::HashMap;
use std::collections::HashSet;
use crate::cell::{CellValue, parse_cell_reference}; 


use petgraph::{
    dot::{Config, Dot},
    graph::{DiGraph, NodeIndex},
    
   
};
use std::{
    fs::File,
    io::Write,
    process::Command,
};

// Constants
const MAX_ROWS: i16 = 999;    // Maximum number of rows in the spreadsheet   
const MAX_COLS: i16 = 18278;  // Maximum number of columns in the spreadsheet

// Structure to represent a range-based child relationship
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RangeChild {
    pub start_key: i32,       // Range start cell key
    pub end_key: i32,         // Range end cell key
    pub child_key: i32,       // Child cell key
}

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

impl Default for CellMeta {
    fn default() -> Self {
        CellMeta {
            formula: -1,
            parent1: -1,
            parent2: -1,
        }
    }
}

// Spreadsheet structure with HashMap of boxed HashSets for children
pub struct Spreadsheet {
    pub grid: Vec<CellValue>,                                // Vector of CellValues (contiguous in memory)
    pub children: HashMap<i32, Box<HashSet<i32>>>,           // Map from cell key to boxed HashSet of children
    pub range_children: Vec<RangeChild>,                     // Vector of range-based child relationships
    pub cell_meta: HashMap<i32, CellMeta>,                   // Map from cell key to metadata
    pub rows: i16,
    pub cols: i16,
    pub viewport_row: i16,
    pub viewport_col: i16,
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
        
        // Create an empty HashMap for children - HashSets will be created only when needed
        // let children = HashMap::with_capacity(total / 10);  // Preallocate with estimated size
                
        Some(Spreadsheet {
            grid,
            children: HashMap::new(),
            range_children: Vec::with_capacity(32), // Preallocate with initial size
            cell_meta: HashMap::new(),
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

    // Helper to get index from row and column
    fn get_index(&self, row: i16, col: i16) -> usize {
        (row as usize) * (self.cols as usize) + (col as usize)
    }
    
    // Get cell metadata, creating it if it doesn't exist
    pub fn get_cell_meta(&mut self, row: i16, col: i16) -> &mut CellMeta {
        let key = self.get_key(row, col);
        self.cell_meta.entry(key).or_insert_with(CellMeta::new)
    }


    pub fn get_cell_meta_mut(&mut self, row: i16, col: i16) -> &mut CellMeta {
        let key = self.get_key(row, col);
        self.cell_meta.entry(key).or_insert(CellMeta::default())
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
    
    pub fn get_key_cell(&self, cell_key: i32) -> &CellValue {
        &self.grid[cell_key as usize]
    }

    pub fn get_mut_cell(&mut self, row: i16, col: i16) -> &mut CellValue {
        let index = self.get_index(row, col);
        &mut self.grid[index]
    }
    
    // Add a range-based child relationship
    pub fn add_range_child(&mut self, start_key: i32, end_key: i32, child_key: i32) {
        self.range_children.push(RangeChild {
            start_key,
            end_key,
            child_key,
        });
    }
    
    // Remove range-based child relationships for a given child
    pub fn remove_range_child(&mut self, child_key: i32) {
        self.range_children.retain(|rc| rc.child_key != child_key);
    }
    
    // Check if a cell is within a range
    pub fn is_cell_in_range(&self, cell_key: i32, start_key: i32, end_key: i32) -> bool {
        let (cell_row, cell_col) = self.get_row_col(cell_key);
        let (start_row, start_col) = self.get_row_col(start_key);
        let (end_row, end_col) = self.get_row_col(end_key);
        
        cell_row >= start_row && cell_row <= end_row && 
        cell_col >= start_col && cell_col <= end_col
    }
    
    // Get all range-based children for a cell
    pub fn get_range_children(&self, cell_key: i32) -> Vec<i32> {
        let mut result = Vec::new();
        for range in &self.range_children {
            if self.is_cell_in_range(cell_key, range.start_key, range.end_key) {
                result.push(range.child_key);
            }
        }
        result
    }

    // Add a child to a cell's dependents (modified for HashMap of boxed HashSets)
    pub fn add_child(&mut self, parent_key: &i32, child_key: &i32) {
        self.children
            .entry(*parent_key)
            .or_insert_with(|| Box::new(HashSet::with_capacity(5)))
            .insert(*child_key);
    }
    
    // Remove a child from a cell's dependents (modified for HashMap of boxed HashSets)
    pub fn remove_child(&mut self, parent_key: i32, child_key: i32) {
        if let Some(children) = self.children.get_mut(&parent_key) {
            children.remove(&child_key);
            
            // If the hashset is now empty, remove it from the HashMap to save memory
            if children.is_empty() {
                self.children.remove(&parent_key);
            }
        }
    }
      
    // Get children for a cell (immutable) (modified for HashMap of boxed HashSets)
    pub fn get_cell_children(&self, key: i32) -> Option<&HashSet<i32>> {
        self.children.get(&key).map(|boxed_set| boxed_set.as_ref())
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
    
    pub fn visualize_cell_relationships(&self, row: i16, col: i16) -> CommandStatus {
        // Check if the cell is valid
        if row < 0 || row >= self.rows || col < 0 || col >= self.cols {
            return CommandStatus::CmdInvalidCell;
        }
    
        // Get the cell key
        let cell_key = self.get_key(row, col);
        
        // Create a directed graph for visualization
        let mut graph = DiGraph::<String, &str>::new();
        let mut node_indices = HashMap::new();
    
        // Function to get formatted cell name
        let get_cell_label = |key: i32| -> String {
            let (r, c) = self.get_row_col(key);
            let col_name = self.get_column_name(c);
            format!("{}{} ({})", col_name, r + 1, match self.grid[key as usize] {
                CellValue::Integer(val) => val.to_string(),
                CellValue::Error => "ERROR".to_string(),
            })
        };
    
        // Add the target cell to the graph
        let target_label = get_cell_label(cell_key);
        let target_node = graph.add_node(target_label.clone());
        node_indices.insert(cell_key, target_node);
    
        // Helper function to process relationships
        fn process_relationships(
            spreadsheet: &Spreadsheet,
            start_key: i32, 
            is_parent_direction: bool,
            processed: &mut HashSet<i32>,
            depth_limit: usize,
            graph: &mut DiGraph<String, &str>,
            node_indices: &mut HashMap<i32, NodeIndex>,
            get_cell_label: &dyn Fn(i32) -> String,
        ) {
            if !processed.insert(start_key) {
                return; // Already processed this cell
            }
            
    
            // Add appropriate relationships based on direction
            if is_parent_direction {
                // Add parents - traverse up the dependency tree
                if let Some(meta) = spreadsheet.cell_meta.get(&start_key) {
                   
    
                    for parent_key in [meta.parent1, meta.parent2].iter()
                                    .filter(|&&k| k >= 0) {
                        
                        // Create parent node if it doesn't exist
                        let parent_idx = if let Some(&idx) = node_indices.get(parent_key) {
                            idx
                        } else {
                            let parent_label = get_cell_label(*parent_key);
                            let idx = graph.add_node(parent_label);
                            node_indices.insert(*parent_key, idx);
                            idx
                        };
                        
                        // Add edge from parent to child
                        let child_idx = node_indices[&start_key];
                        graph.add_edge(parent_idx, child_idx, "depends on");
                        
                        // Recurse for this parent (up to the depth limit)
                        if processed.len() < depth_limit {
                            process_relationships(
                                spreadsheet,
                                *parent_key, 
                                true, 
                                processed, 
                                depth_limit,
                                graph,
                                node_indices,
                                get_cell_label
                            );
                        }
                    }
                }
            } else {
                // Add children - traverse down the dependency tree
                if let Some(children) = spreadsheet.get_cell_children(start_key) {
                
                    for &child_key in children {
                        // Create child node if it doesn't exist
                        let child_idx = if let Some(&idx) = node_indices.get(&child_key) {
                            idx
                        } else {
                            let child_label = get_cell_label(child_key);
                            let idx = graph.add_node(child_label);
                            node_indices.insert(child_key, idx);
                            idx
                        };
                        
                        // Add edge from parent to child
                        let parent_idx = node_indices[&start_key];
                        graph.add_edge(parent_idx, child_idx, "used by");
                        
                        // Recurse for this child (up to the depth limit)
                        if processed.len() < depth_limit {
                            process_relationships(
                                spreadsheet,
                                child_key, 
                                false, 
                                processed, 
                                depth_limit,
                                graph,
                                node_indices,
                                get_cell_label
                            );
                        }
                    }
                }
            }
            
        }
    
        // Process parents (upward traversal)
        let mut processed = HashSet::new();
         // Mark target cell as processed
        process_relationships(
            self,
            cell_key, 
            true, 
            &mut processed, 
            20,
            &mut graph,
            &mut node_indices,
            &get_cell_label
        );
        
        // Process children (downward traversal)
        let mut processed = HashSet::new();
         // Mark target cell as processed
        process_relationships(
            self,
            cell_key, 
            false, 
            &mut processed, 
            20,
            &mut graph,
            &mut node_indices,
            &get_cell_label
        );
    
        // Generate DOT format
        let dot = Dot::with_config(&graph, &[Config::EdgeNoLabel]);
        
        // Save to temp file
        let temp_file = format!("cell_{}_{}_relationships.dot", row, col);
        let mut file = match File::create(&temp_file) {
            Ok(file) => file,
            Err(e) => {
                eprintln!("Failed to create dot file: {}", e);
                return CommandStatus::CmdOk;
            }
        };
        
        if let Err(e) = writeln!(file, "{:?}", dot) {
            eprintln!("Failed to write to dot file: {}", e);
            return CommandStatus::CmdOk;
        }
    
        println!("Cell relationships saved to {}", temp_file);
        
        // Attempt to render with Graphviz if available
        let output_file = format!("cell_{}_{}_relationships.png", row, col);
        match Command::new("dot")
            .args(["-Tpng", &temp_file, "-o", &output_file])
            .output() 
        {
            Ok(_) => {
                println!("Cell relationship diagram generated as {}", output_file);
                // Try to open the image with the default viewer
                #[cfg(target_os = "windows")]
                let _ = Command::new("cmd").args(["/C", &output_file]).spawn();
                
                #[cfg(target_os = "macos")]
                let _ = Command::new("open").arg(&output_file).spawn();
                
                #[cfg(target_os = "linux")]
                let _ = Command::new("xdg-open").arg(&output_file).spawn();
            }
            Err(_) => {
                println!("Graphviz not found. You can manually convert the .dot file to an image.");
                println!("For instance: dot -Tpng {} -o {}", temp_file, output_file);
            }
        }
    
        // Print textual representation of the relationships
        println!("\nCell {}{}:", self.get_column_name(col), row + 1);
        
        // Show parents
        if let Some(meta) = self.cell_meta.get(&cell_key) {
            println!("  Parents:");
            let mut has_parents = false;
            
            for parent_key in [meta.parent1, meta.parent2].iter().filter(|&&k| k >= 0) {
                has_parents = true;
                let (r, c) = self.get_row_col(*parent_key);
                println!("    - {}{}: {}", 
                    self.get_column_name(c), 
                    r + 1,
                    match self.grid[*parent_key as usize] {
                        CellValue::Integer(val) => val.to_string(),
                        CellValue::Error => "ERROR".to_string(),
                    }
                );
            }
            
            if !has_parents {
                println!("    (none)");
            }
        }
        
        // Show children
        println!("  Children:");
        if let Some(children) = self.get_cell_children(cell_key) {
            if !children.is_empty() {
                for &child_key in children {
                    let (r, c) = self.get_row_col(child_key);
                    println!("    - {}{}: {}", 
                        self.get_column_name(c), 
                        r + 1,
                        match self.grid[child_key as usize] {
                            CellValue::Integer(val) => val.to_string(),
                            CellValue::Error => "ERROR".to_string(),
                        }
                    );
                }
            } else {
                println!("    (none)");
            }
        } else {
            println!("    (none)");
        }
        
        CommandStatus::CmdOk
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::CellValue;

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
    fn test_get_key_and_row_col() {
        let sheet = Spreadsheet::create(5, 5).unwrap();
        let key = sheet.get_key(2, 3);
        let (row, col) = sheet.get_row_col(key);
        assert_eq!(row, 2);
        assert_eq!(col, 3);
    }

    #[test]
    fn test_get_cell_meta() {
        let mut sheet = Spreadsheet::create(5, 5).unwrap();
        let meta = sheet.get_cell_meta(1, 1);
        assert_eq!(meta.formula, -1);
        assert_eq!(meta.parent1, -1);
        assert_eq!(meta.parent2, -1);
        meta.formula = 10;
        assert_eq!(sheet.get_cell_meta(1, 1).formula, 10);
    }

    #[test]
    fn test_add_remove_child() {
        let mut sheet = Spreadsheet::create(5, 5).unwrap();
        let parent = sheet.get_key(0, 0);
        let child = sheet.get_key(1, 1);
        sheet.add_child(&parent, &child);
        assert!(sheet.get_cell_children(parent).unwrap().contains(&child));
        sheet.remove_child(parent, child);
        assert!(sheet.get_cell_children(parent).is_none());
    }

    #[test]
    fn test_add_remove_range_child() {
        let mut sheet = Spreadsheet::create(5, 5).unwrap();
        let parent1 = sheet.get_key(0, 0);
        let parent2 = sheet.get_key(1, 1);
        let child = sheet.get_key(2, 2);
        sheet.add_range_child(parent1, parent2, child);
        assert!(sheet.get_range_children(parent1).contains(&child));
        sheet.remove_range_child(child);
        assert!(!sheet.get_range_children(parent1).contains(&child));
    }

    #[test]
    fn test_is_cell_in_range() {
        let sheet = Spreadsheet::create(5, 5).unwrap();
        let cell_key = sheet.get_key(1, 1);
        let start_key = sheet.get_key(0, 0);
        let end_key = sheet.get_key(2, 2);
        assert!(sheet.is_cell_in_range(cell_key, start_key, end_key));
        assert!(!sheet.is_cell_in_range(cell_key, end_key, start_key));
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

    #[test]
    fn test_print_spreadsheet_with_values() {
        let mut sheet = Spreadsheet::create(5, 5).unwrap();
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(42);
        *sheet.get_mut_cell(1, 1) = CellValue::Error;
        sheet.output_enabled = true;
        sheet.print_spreadsheet(); // Should not panic
    }
}