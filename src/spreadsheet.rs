use crate::cell::{CellValue, parse_cell_reference};
use crate::formula::Range;
use crate::visualize_cells;
use std::cmp::min;
use std::collections::HashMap;
use std::collections::HashSet;

// Constants
const MAX_ROWS: i16 = 999; // Maximum number of rows in the spreadsheet   
const MAX_COLS: i16 = 18278; // Maximum number of columns in the spreadsheet

/// Represents a highlighted relationship type for visualization.
///
/// # Variants
///
/// * `Parent` - Highlights parent cells.
/// * `Child` - Highlights child cells.
/// * `Both` - Highlights both (not typically used).
/// * `None` - No highlighting.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HighlightType {
    Parent,
    Child,
    Both,
    None,
}

/// Represents a range-based dependency.
///
/// # Fields
///
/// * `start_key` - Starting cell key.
/// * `end_key` - Ending cell key.
/// * `child_key` - Dependent cell key.

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RangeChild {
    pub start_key: i32, // Range start cell key
    pub end_key: i32,   // Range end cell key
    pub child_key: i32, // Child cell key
}

/// Status codes for command execution.
///
/// # Variants
///
/// * `CmdOk` - Success.
/// * `CmdUnrecognized` - Unknown command or error.
/// * `CmdCircularRef` - Circular reference detected.
/// * `CmdInvalidCell` - Invalid cell reference.
/// * `CmdLockedCell` - Cell is locked.
/// * `CmdNotLockedCell` - Cell is not locked.

#[derive(Debug, PartialEq)]
pub enum CommandStatus {
    CmdOk,
    CmdUnrecognized,
    CmdCircularRef,
    CmdInvalidCell,
    CmdLockedCell,
    CmdNotLockedCell,
}

/// Metadata for a cell’s formula and dependencies.
///
/// # Fields
///
/// * `formula` - Formula code.
/// * `parent1` - First parent key or constant.
/// * `parent2` - Second parent key or constant.

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

/// The core spreadsheet structure.
///
/// Manages the grid, dependencies, and UI state.
///
/// # Fields
///
/// * `grid` - Vector of cell values.
/// * `children` - Map of cell keys to child sets.
/// * `range_children` - Range-based dependencies.
/// * `cell_meta` - Map of cell keys to metadata.
/// * `rows` - Number of rows.
/// * `cols` - Number of columns.
/// * `viewport_row` - Top row of the visible area.
/// * `viewport_col` - Left column of the visible area.
/// * `output_enabled` - Toggles display output.
/// * `locked_ranges` - Locked cell ranges.
/// * `named_ranges` - Named ranges.
/// * `cell_history` - History of cell values.
/// * `last_edited` - Last edited cell coordinates.
/// * `highlight_cell` - Key of the highlighted cell.
/// * `highlight_type` - Type of highlighting.
/// * `display` - Number of rows/cols to display
pub struct Spreadsheet {
    pub grid: Vec<CellValue>, // Vector of CellValues (contiguous in memory)
    pub children: HashMap<i32, HashSet<i32>>, // Map from cell key to boxed HashSet of children
    pub range_children: Vec<RangeChild>, // Vector of range-based child relationships
    pub cell_meta: HashMap<i32, CellMeta>, // Map from cell key to metadata
    pub rows: i16,
    pub cols: i16,
    pub viewport_row: i16,
    pub viewport_col: i16,
    pub output_enabled: bool,
    pub locked_ranges: Vec<Range>,
    pub named_ranges: HashMap<String, Range>,
    pub cell_history: HashMap<i32, Vec<CellValue>>,
    pub last_edited: Option<(i16, i16)>,
    pub highlight_cell: i32,
    pub highlight_type: HighlightType,
}

impl Spreadsheet {
    /// Creates a new spreadsheet with the given dimensions.
    ///
    /// # Arguments
    ///
    /// * `rows` - Number of rows (1 to 999).
    /// * `cols` - Number of columns (1 to 18278).
    ///
    /// # Returns
    ///
    /// * `Some(Spreadsheet)` - If dimensions are valid.
    /// * `None` - If dimensions are invalid.
    pub fn create(rows: i16, cols: i16) -> Option<Spreadsheet> {
        if !(1..=MAX_ROWS).contains(&rows) || !(1..=MAX_COLS).contains(&cols) {
            eprintln!("Invalid spreadsheet dimensions");
            return None;
        }

        // Create empty cells - initialize with Integer(0)
        let total = rows as usize * cols as usize;
        let grid = vec![CellValue::Integer(0); total];

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
            locked_ranges: Vec::new(),
            named_ranges: HashMap::new(),
            cell_history: HashMap::new(),
            last_edited: None,
            highlight_cell: -1,
            highlight_type: HighlightType::None,
        })
    }

    /// Computes the unique key for a cell.
    pub fn get_key(&self, row: i16, col: i16) -> i32 {
        row as i32 * self.cols as i32 + col as i32
    }

    // Helper to get coordinates from cell key
    pub fn get_row_col(&self, key: i32) -> (i16, i16) {
        let row = (key / (self.cols as i32)) as i16;
        let col = (key % (self.cols as i32)) as i16;
        (row, col)
    }

    // Helper to get index from row and column
    pub fn get_index(&self, row: i16, col: i16) -> usize {
        (row as usize) * (self.cols as usize) + (col as usize)
    }

    // Get cell metadata, creating it if it doesn't exist
    pub fn get_cell_meta(&mut self, row: i16, col: i16) -> &mut CellMeta {
        let key = self.get_key(row, col);
        self.cell_meta.entry(key).or_insert_with(CellMeta::new)
    }

    pub fn get_cell_meta_ref(&self, row: i16, col: i16) -> &CellMeta {
        let key = self.get_key(row, col);
        self.cell_meta.get(&key).unwrap_or(&CellMeta {
            formula: -1,
            parent1: -1,
            parent2: -1,
        })
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

        cell_row >= start_row && cell_row <= end_row && cell_col >= start_col && cell_col <= end_col
    }

    // Add a child to a cell's dependents (modified for HashMap of boxed HashSets)
    pub fn add_child(&mut self, parent_key: &i32, child_key: &i32) {
        self.children
            .entry(*parent_key)
            .or_insert_with(|| HashSet::with_capacity(5))
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
        self.children.get(&key)
    }

    pub fn set_highlight(&mut self, row: i16, col: i16, highlight_type: HighlightType) {
        self.highlight_cell = self.get_key(row, col);
        self.highlight_type = highlight_type;
    }

    pub fn disable_highlight(&mut self) {
        self.highlight_cell = -1;
        self.highlight_type = HighlightType::None;
    }

    pub fn is_highlighted(&self, cell_key: i32) -> (bool, HighlightType) {
        if self.highlight_cell == -1 || self.highlight_type == HighlightType::None {
            return (false, HighlightType::None);
        }

        // Check if it's a parent of the highlighted cell
        let meta = self.cell_meta.get(&self.highlight_cell);
        if let Some(meta) = meta {
            if self.highlight_type == HighlightType::Parent
                || self.highlight_type == HighlightType::Both
            {
                let rem = meta.formula % 10;
                match rem {
                    0 => {
                        if meta.parent1 == cell_key || meta.parent2 == cell_key {
                            return (true, HighlightType::Parent);
                        }
                    }
                    2 => {
                        if meta.parent1 == cell_key {
                            return (true, HighlightType::Parent);
                        }
                    }
                    3 => {
                        if meta.parent2 == cell_key {
                            return (true, HighlightType::Parent);
                        }
                    }
                    _ => {
                        if self.is_cell_in_range(cell_key, meta.parent1, meta.parent2) {
                            return (true, HighlightType::Parent);
                        }
                    }
                }
            }
        }
        if self.highlight_type == HighlightType::Child || self.highlight_type == HighlightType::Both
        {
            // get cell children and also it can be in range also
            let mut is_contains = false;

            // Safely check if the highlight_cell has any children
            if let Some(children) = self.children.get(&self.highlight_cell) {
                is_contains = children.contains(&cell_key);
            }

            // Check range-based children
            is_contains |= self.range_children.iter().any(|rc| {
                rc.child_key == cell_key
                    && self.is_cell_in_range(self.highlight_cell, rc.start_key, rc.end_key)
            });

            if is_contains {
                return (true, HighlightType::Child);
            }
        }
        // If not a parent or child, return false
        (false, HighlightType::None)
    }

    pub fn print_spreadsheet_with_highlights(&self) {
        if !self.output_enabled {
            return;
        }

        let start_row = self.viewport_row;
        let start_col = self.viewport_col;
        let display_row = min(self.rows - start_row, 10); // Display only a portion of the spreadsheet
        let display_col = min(self.cols - start_col, 10);

        // ANSI color codes
        const RESET: &str = "\x1b[0m";
        const RED: &str = "\x1b[1;31m"; // Bold red for parents
        const GREEN: &str = "\x1b[1;32m"; // Bold green for children
        const CYAN: &str = "\x1b[1;36m"; // Bold cyan for main cell

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
                let row = start_row + i;
                let col = start_col + j;
                let cell_key = self.get_key(row, col);
                let cell_value = self.get_cell(row, col);

                // Check if this cell should be highlighted - only check cells in view
                let (is_highlighted, highlight_type) = self.is_highlighted(cell_key);

                // Apply appropriate color based on highlight status
                // If it's the main highlighted cell itself
                if cell_key == self.highlight_cell {
                    print!("{}", CYAN);
                } else if is_highlighted {
                    match highlight_type {
                        HighlightType::Parent => print!("{}", RED),
                        HighlightType::Child => print!("{}", GREEN),
                        HighlightType::Both => {} // This shouldn't happen due to circular ref prevention
                        HighlightType::None => {} // Main highlighted cell
                    }
                }

                // Print cell value
                match cell_value {
                    CellValue::Integer(value) => print!("{:<8} ", value),
                    CellValue::Error => print!("{:<8} ", "ERR"),
                }

                // Reset color if necessary
                print!("{}", RESET);
            }
            println!();
        }
    }

    pub fn print_spreadsheet(&self) {
        if !self.output_enabled {
            return;
        } else if self.highlight_type != HighlightType::None {
            self.print_spreadsheet_with_highlights();
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

    /// Scrolls to a specific cell.
    ///
    /// # Arguments
    ///
    /// * `cell` - Cell reference (e.g., "A1").
    ///
    /// # Returns
    ///
    /// * `CommandStatus::CmdOk` - On success.
    /// * `CommandStatus::CmdInvalidCell` - If out of bounds.
    /// * `CommandStatus::CmdUnrecognized` - If parsing fails.
    pub fn scroll_to_cell(&mut self, cell: &str) -> CommandStatus {
        match parse_cell_reference(self, cell) {
            Ok((row, col)) => {
                    self.viewport_row = row;
                    self.viewport_col = col;
                    CommandStatus::CmdOk
            }
            Err(_) => {
                CommandStatus::CmdUnrecognized
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
                if self.viewport_row + VIEWPORT_SIZE < self.rows - 9 {
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
                if self.viewport_col + VIEWPORT_SIZE < self.cols - 9 {
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
        visualize_cells::visualize_cell_relationships(self, row, col)
    }

    pub fn lock_range(&mut self, range: Range) {
        self.locked_ranges.push(range);
    }

    pub fn unlock_range(&mut self, range: Range) {
        self.locked_ranges.retain(|r: &Range| r != &range);
    }

    pub fn is_cell_locked(&self, row: i16, col: i16) -> bool {
        for range in &self.locked_ranges {
            if row >= range.start_row
                && row <= range.end_row
                && col >= range.start_col
                && col <= range.end_col
            {
                return true;
            }
        }
        false
    }

    pub fn set_last_edited(&mut self, row: i16, col: i16) {
        self.last_edited = Some((row, col));
    }

    pub fn scroll_to_last_edited(&mut self) {
        if let Some((row, col)) = self.last_edited {
            self.viewport_row = row;
            self.viewport_col = col;
        }
    }

    pub fn get_cell_name(&self, row: i16, col: i16) -> String {
        for (name, range) in &self.named_ranges {
            if range.start_row == row
                && range.start_col == col
                && range.end_row == row
                && range.end_col == col
            {
                return name.clone();
            }
        }
        let col_name = self.get_column_name(col);
        format!("{}{}", col_name, row + 1)
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
        assert_eq!(sheet.scroll_to_cell("F6"), CommandStatus::CmdUnrecognized);
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

    #[test]
    fn test_create_edge_cases() {
        let sheet = Spreadsheet::create(1, 1).unwrap();
        assert_eq!(sheet.grid.len(), 1);
    }

    #[test]
    fn test_get_column_name_large_values() {
        let sheet = Spreadsheet::create(1, MAX_COLS).unwrap();
        assert_eq!(sheet.get_column_name(702), "AAA");
        assert_eq!(sheet.get_column_name(18277), "ZZZ");
    }

    #[test]
    fn test_add_range_child() {
        let mut sheet = Spreadsheet::create(5, 5).unwrap();
        let start_key = sheet.get_key(0, 0);
        let end_key = sheet.get_key(2, 2);
        let child_key = sheet.get_key(3, 3);
        sheet.add_range_child(start_key, end_key, child_key);
        assert_eq!(sheet.range_children.len(), 1);
        assert_eq!(sheet.range_children[0].child_key, child_key);
    }

    #[test]
    fn test_remove_range_child() {
        let mut sheet = Spreadsheet::create(5, 5).unwrap();
        let child_key = sheet.get_key(3, 3);
        sheet.add_range_child(sheet.get_key(0, 0), sheet.get_key(2, 2), child_key);
        sheet.remove_range_child(child_key);
        assert!(sheet.range_children.is_empty());
    }

    #[test]
    fn test_is_highlighted_no_highlight() {
        let sheet = Spreadsheet::create(5, 5).unwrap();
        let (highlighted, htype) = sheet.is_highlighted(sheet.get_key(0, 0));
        assert!(!highlighted);
        assert_eq!(htype, HighlightType::None);
    }

    #[test]
    fn test_print_spreadsheet_with_highlights() {
        let mut sheet = Spreadsheet::create(5, 5).unwrap();
        sheet.output_enabled = true;
        sheet.set_highlight(0, 0, HighlightType::Parent);
        sheet.print_spreadsheet_with_highlights(); // Should not panic
    }

    #[test]
    fn test_lock_unlock_range() {
        let mut sheet = Spreadsheet::create(5, 5).unwrap();
        let range = Range {
            start_row: 1,
            start_col: 1,
            end_row: 2,
            end_col: 2,
        };
        sheet.lock_range(range.clone());
        assert!(sheet.is_cell_locked(1, 1));
        sheet.unlock_range(range);
        assert!(!sheet.is_cell_locked(1, 1));
    }

    #[test]
    fn test_named_ranges() {
        let mut sheet = Spreadsheet::create(5, 5).unwrap();
        let range = Range {
            start_row: 0,
            start_col: 0,
            end_row: 0,
            end_col: 0,
        };
        sheet.named_ranges.insert("test".to_string(), range);
        assert_eq!(sheet.get_cell_name(0, 0), "test");
    }

    #[test]
    fn test_scroll_viewport_top_left_bounds() {
        let mut sheet = Spreadsheet::create(50, 50).unwrap();
        sheet.viewport_row = 5;
        sheet.viewport_col = 5;
        sheet.scroll_viewport('w');
        assert_eq!(sheet.viewport_row, 0);
        sheet.scroll_viewport('a');
        assert_eq!(sheet.viewport_col, 0);
    }

    #[test]
    fn test_scroll_viewport_bottom_right_bounds() {
        let mut sheet = Spreadsheet::create(50, 50).unwrap();
        sheet.scroll_viewport('s');
        sheet.scroll_viewport('s');
        sheet.scroll_viewport('s');
        assert_eq!(sheet.viewport_row, 30); // 50 - VIEWPORT_SIZE
        sheet.scroll_viewport('d');
        sheet.scroll_viewport('d');
        sheet.scroll_viewport('d');
        assert_eq!(sheet.viewport_col, 30);
    }

    #[test]
    fn test_lock_multiple_ranges() {
        let mut sheet = Spreadsheet::create(5, 5).unwrap();
        let range1 = Range {
            start_row: 0,
            start_col: 0,
            end_row: 1,
            end_col: 1,
        };
        let range2 = Range {
            start_row: 2,
            start_col: 2,
            end_row: 3,
            end_col: 3,
        };
        sheet.lock_range(range1);
        sheet.lock_range(range2);
        assert!(sheet.is_cell_locked(0, 0));
        assert!(sheet.is_cell_locked(2, 2));
        assert!(!sheet.is_cell_locked(4, 4));
    }

    #[test]
    fn test_named_ranges_overlap() {
        let mut sheet = Spreadsheet::create(5, 5).unwrap();
        let range1 = Range {
            start_row: 0,
            start_col: 0,
            end_row: 0,
            end_col: 0,
        };
        let range2 = Range {
            start_row: 0,
            start_col: 0,
            end_row: 1,
            end_col: 1,
        };
        sheet.named_ranges.insert("start".to_string(), range1);
        sheet.named_ranges.insert("area".to_string(), range2);
        assert_eq!(sheet.get_cell_name(0, 0), "start"); // First match wins
    }

    #[test]
    fn test_cell_history_multiple_values() {
        let mut sheet = Spreadsheet::create(5, 5).unwrap();
        let key = sheet.get_key(0, 0);
        sheet.cell_history.insert(key, vec![
            CellValue::Integer(1),
            CellValue::Integer(2),
        ]);
        assert_eq!(sheet.cell_history[&key].len(), 2);
    }

    #[test]
    fn test_is_highlighted_parent() {
        let mut sheet = Spreadsheet::create(5, 5).unwrap();
        let cell_key = sheet.get_key(1, 1);
        let parent_key = sheet.get_key(0, 0);
        let meta = sheet.get_cell_meta(1, 1);
        meta.parent1 = parent_key;
        meta.formula = 2;
        sheet.set_highlight(1, 1, HighlightType::Parent);
        let (highlighted, htype) = sheet.is_highlighted(parent_key);
        assert!(highlighted);
        assert_eq!(htype, HighlightType::Parent);
    }

    #[test]
    fn test_is_highlighted_child_range() {
        let mut sheet = Spreadsheet::create(5, 5).unwrap();
        let cell_key = sheet.get_key(0, 0);
        let child_key = sheet.get_key(1, 1);
        sheet.add_range_child(cell_key, cell_key, child_key);
        sheet.set_highlight(0, 0, HighlightType::Child);
        let (highlighted, htype) = sheet.is_highlighted(child_key);
        assert!(highlighted);
        assert_eq!(htype, HighlightType::Child);
    }

    #[test]
    fn test_create_max_dimensions() {
        let sheet = Spreadsheet::create(MAX_ROWS, MAX_COLS).unwrap();
        assert_eq!(sheet.rows, MAX_ROWS);
        assert_eq!(sheet.cols, MAX_COLS);
    }

    #[test]
    fn test_scroll_to_cell_out_of_bounds() {
        let mut sheet = Spreadsheet::create(5, 5).unwrap();
        assert_eq!(
            sheet.scroll_to_cell("A1000"),
            CommandStatus::CmdUnrecognized
        );
    }
}
