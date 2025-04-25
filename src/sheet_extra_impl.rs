use crate::cell::CellValue;
use crate::formula::Range;
use crate::spreadsheet::{CellMeta, CommandStatus, HighlightType, Spreadsheet};
use crate::visualize_cells;
use std::cmp::min;

impl Spreadsheet {
    pub fn get_cell_meta_ref(&self, row: i16, col: i16) -> &CellMeta {
        let key = self.get_key(row, col);
        self.cell_meta.get(&key).unwrap_or(&CellMeta {
            formula: -1,
            parent1: -1,
            parent2: -1,
        })
    }

    /// Visualizes the relationships of a cell at the specified row and column.
    ///
    /// This function checks if the cell at the given coordinates is valid and then
    /// delegates to the `visualize_cells` module to generate a visualization of the
    /// cell's relationships, such as dependencies or references.
    ///
    /// # Arguments
    ///
    /// * `row` - The row index of the cell (0-based).
    /// * `col` - The column index of the cell (0-based).
    ///
    /// # Returns
    ///
    /// A `CommandStatus` indicating the success or failure of the visualization operation
    pub fn visualize_cell_relationships(&self, row: i16, col: i16) -> CommandStatus {
        // Check if the cell is valid
        visualize_cells::visualize_cell_relationships(self, row, col)
    }

    /// Locks a specified range of cells to prevent modifications.
    ///
    /// Adds the given range to the collection of locked ranges in the spreadsheet.
    /// Locked ranges cannot be edited until they are explicitly unlocked.
    ///
    /// # Arguments
    ///
    /// * `range` - The `Range` struct defining the cell range to lock.
    pub fn lock_range(&mut self, range: Range) {
        self.locked_ranges.push(range);
    }

    pub fn unlock_range(&mut self, range: Range) {
        self.locked_ranges.retain(|r: &Range| r != &range);
    }

    /// Checks if a cell at the specified row and column is locked.
    ///
    /// A cell is considered locked if it falls within any of the locked ranges
    /// stored in the spreadsheet.
    ///
    /// # Arguments
    ///
    /// * `row` - The row index of the cell (0-based).
    /// * `col` - The column index of the cell (0-based).
    ///
    /// # Returns
    ///
    /// A `bool` indicating whether the cell is locked (`true`) or not (`false`).
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

    /// Sets the last edited cell to the specified row and column.
    ///
    /// Updates the `last_edited` field of the spreadsheet to store the coordinates
    /// of the most recently edited cell.
    ///
    /// # Arguments
    ///
    /// * `row` - The row index of the last edited cell (0-based).
    /// * `col` - The column index of the last edited cell (0-based).
    pub fn set_last_edited(&mut self, row: i16, col: i16) {
        self.last_edited = Some((row, col));
    }

    /// Scrolls the viewport to the last edited cell.
    ///
    /// If a cell was previously marked as the last edited cell, this function updates
    /// the spreadsheet's viewport to center on that cell's coordinates.
    pub fn scroll_to_last_edited(&mut self) {
        if let Some((row, col)) = self.last_edited {
            self.viewport_row = row;
            self.viewport_col = col;
        }
    }

    /// Retrieves the name of a cell at the specified row and column.
    ///
    /// If the cell is part of a named range (a single-cell range with an associated name),
    /// the function returns that name. Otherwise, it generates a standard cell name in
    /// the format `<column_letter><row_number>` (e.g., `A1`).
    ///
    /// # Arguments
    ///
    /// * `row` - The row index of the cell (0-based).
    /// * `col` - The column index of the cell (0-based).
    ///
    /// # Returns
    ///
    /// A `String` representing the name of the cell.
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::CellValue;
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
        sheet
            .cell_history
            .insert(key, vec![CellValue::Integer(1), CellValue::Integer(2)]);
        assert_eq!(sheet.cell_history[&key].len(), 2);
    }

    #[test]
    fn test_is_highlighted_parent() {
        let mut sheet = Spreadsheet::create(5, 5).unwrap();
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
}
