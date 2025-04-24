use crate::formula::Range;
use crate::spreadsheet::{CommandStatus, Spreadsheet};
use crate::visualize_cells;

impl Spreadsheet {
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
