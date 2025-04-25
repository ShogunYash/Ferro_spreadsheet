use crate::formula::Range;
use crate::spreadsheet::{CommandStatus, Spreadsheet};
use crate::visualize_cells;

impl Spreadsheet {
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
    /// A `CommandStatus` indicating the success or failure of the visualization operation.
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
    /// Unlocks a specified range of cells, allowing modifications.
    ///
    /// Removes the given range from the collection of locked ranges, if it exists.
    ///
    /// # Arguments
    ///
    /// * `range` - The `Range` struct defining the cell range to unlock.
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formula::Range;
    use crate::spreadsheet::CommandStatus;

    fn create_test_spreadsheet(rows: i16, cols: i16) -> Spreadsheet {
        Spreadsheet::create(rows, cols).unwrap()
    }

    #[test]
    fn test_visualize_cell_relationships_valid() {
        let sheet = create_test_spreadsheet(5, 5);
        assert_eq!(sheet.visualize_cell_relationships(0, 0), CommandStatus::CmdOk);
    }

    #[test]
    fn test_visualize_cell_relationships_invalid_row() {
        let sheet = create_test_spreadsheet(5, 5);
        assert_eq!(sheet.visualize_cell_relationships(5, 0), CommandStatus::InvalidCell);
    }

    #[test]
    fn test_visualize_cell_relationships_invalid_col() {
        let sheet = create_test_spreadsheet(5, 5);
        assert_eq!(sheet.visualize_cell_relationships(0, 5), CommandStatus::InvalidCell);
    }

    #[test]
    fn test_lock_range() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let range = Range { start_row: 1, start_col: 1, end_row: 2, end_col: 2 };
        sheet.lock_range(range.clone());
        assert_eq!(sheet.locked_ranges.len(), 1);
        assert_eq!(sheet.locked_ranges[0], range);
    }

    #[test]
    fn test_unlock_range() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let range = Range { start_row: 1, start_col: 1, end_row: 2, end_col: 2 };
        sheet.lock_range(range.clone());
        sheet.unlock_range(range.clone());
        assert!(sheet.locked_ranges.is_empty());
    }

    #[test]
    fn test_unlock_range_not_present() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let range = Range { start_row: 1, start_col: 1, end_row: 2, end_col: 2 };
        sheet.unlock_range(range);
        assert!(sheet.locked_ranges.is_empty());
    }

    #[test]
    fn test_is_cell_locked_inside() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let range = Range { start_row: 1, start_col: 1, end_row: 3, end_col: 3 };
        sheet.lock_range(range);
        assert!(sheet.is_cell_locked(2, 2));
    }

    #[test]
    fn test_is_cell_locked_outside() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let range = Range { start_row: 1, start_col: 1, end_row: 3, end_col: 3 };
        sheet.lock_range(range);
        assert!(!sheet.is_cell_locked(0, 0));
        assert!(!sheet.is_cell_locked(4, 4));
    }

    #[test]
    fn test_is_cell_locked_boundary() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let range = Range { start_row: 1, start_col: 1, end_row: 3, end_col: 3 };
        sheet.lock_range(range);
        assert!(sheet.is_cell_locked(1, 1));
        assert!(sheet.is_cell_locked(3, 3));
    }

    #[test]
    fn test_is_cell_locked_multiple_ranges() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let range1 = Range { start_row: 0, start_col: 0, end_row: 1, end_col: 1 };
        let range2 = Range { start_row: 3, start_col: 3, end_row: 4, end_col: 4 };
        sheet.lock_range(range1);
        sheet.lock_range(range2);
        assert!(sheet.is_cell_locked(0, 0));
        assert!(sheet.is_cell_locked(4, 4));
        assert!(!sheet.is_cell_locked(2, 2));
    }

    #[test]
    fn test_set_last_edited() {
        let mut sheet = create_test_spreadsheet(5, 5);
        sheet.set_last_edited(2, 3);
        assert_eq!(sheet.last_edited, Some((2, 3)));
    }

    #[test]
    fn test_scroll_to_last_edited_set() {
        let mut sheet = create_test_spreadsheet(5, 5);
        sheet.set_last_edited(2, 3);
        sheet.scroll_to_last_edited();
        assert_eq!(sheet.viewport_row, 2);
        assert_eq!(sheet.viewport_col, 3);
    }

    #[test]
    fn test_scroll_to_last_edited_not_set() {
        let mut sheet = create_test_spreadsheet(5, 5);
        sheet.viewport_row = 1;
        sheet.viewport_col = 1;
        sheet.scroll_to_last_edited();
        assert_eq!(sheet.viewport_row, 1);
        assert_eq!(sheet.viewport_col, 1);
    }

    #[test]
    fn test_get_cell_name_named_range() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let range = Range { start_row: 0, start_col: 0, end_row: 0, end_col: 0 };
        sheet.named_ranges.insert("test".to_string(), range);
        assert_eq!(sheet.get_cell_name(0, 0), "test");
    }

    #[test]
    fn test_get_cell_name_default() {
        let sheet = create_test_spreadsheet(5, 5);
        assert_eq!(sheet.get_cell_name(0, 0), "A1");
        assert_eq!(sheet.get_cell_name(1, 2), "C2");
    }

    #[test]
    fn test_get_cell_name_multiple_named_ranges() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let range1 = Range { start_row: 0, start_col: 0, end_row: 0, end_col: 0 };
        let range2 = Range { start_row: 1, start_col: 1, end_row: 1, end_col: 1 };
        sheet.named_ranges.insert("top_left".to_string(), range1);
        sheet.named_ranges.insert("center".to_string(), range2);
        assert_eq!(sheet.get_cell_name(0, 0), "top_left");
        assert_eq!(sheet.get_cell_name(1, 1), "center");
        assert_eq!(sheet.get_cell_name(2, 2), "C3");
    }

    #[test]
    fn test_get_cell_name_no_matching_named_range() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let range1 = Range { start_row: 1, start_col: 1, end_row: 1, end_col: 1 };
        let range2 = Range { start_row: 2, start_col: 2, end_row: 2, end_col: 2 };
        sheet.named_ranges.insert("B2".to_string(), range1);
        sheet.named_ranges.insert("C3".to_string(), range2);
        assert_eq!(sheet.get_cell_name(0, 0), "A1");
    }

    #[test]
    fn test_get_cell_name_high_column() {
        let sheet = create_test_spreadsheet(5, 30);
        assert_eq!(sheet.get_cell_name(0, 25), "Z1");
        assert_eq!(sheet.get_cell_name(0, 26), "AA1");
    }

    #[test]
    fn test_get_cell_name_row_numbers() {
        let sheet = create_test_spreadsheet(10, 5);
        assert_eq!(sheet.get_cell_name(0, 0), "A1");
        assert_eq!(sheet.get_cell_name(9, 0), "A10");
    }
}
