use std::cmp::min;
use std::collections::{HashMap, HashSet};
use crate::cell::{CellValue, parse_cell_reference};
use crate::visualize_cells;
use crate::formula::Range;

// Constants
pub const MAX_ROWS: i16 = 999;
pub const MAX_COLS: i16 = 18278;
pub const MAX_DISPLAY: i16 = 15;

// Structure to represent a range-based child relationship
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RangeChild {
    pub start_key: i32,
    pub end_key: i32,
    pub child_key: i32,
}

#[derive(Debug, PartialEq)]
pub enum CommandStatus {
    CmdOk,
    CmdUnrecognized,
    CmdCircularRef,
    CmdInvalidCell,
    CmdLockedCell,
}

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

pub struct Spreadsheet {
    pub grid: Vec<CellValue>,
    pub children: HashMap<i32, Box<HashSet<i32>>>,
    pub range_children: Vec<RangeChild>,
    pub cell_meta: HashMap<i32, CellMeta>,
    pub rows: i16,
    pub cols: i16,
    pub viewport_row: i16,
    pub viewport_col: i16,
    pub output_enabled: bool,
    pub display_rows: i16,
    pub display_cols: i16,
    pub locked_ranges: Vec<Range>,
    pub named_ranges: HashMap<String, Range>,
    pub cell_history: HashMap<i32, Vec<CellValue>>,
    pub last_edited: Option<(i16, i16)>,
}

impl Spreadsheet {
    pub fn create(rows: i16, cols: i16) -> Option<Spreadsheet> {
        if rows < 1 || rows > MAX_ROWS || cols < 1 || cols > MAX_COLS {
            eprintln!("Invalid spreadsheet dimensions");
            return None;
        }
        let total = rows as usize * cols as usize;
        let grid = vec![CellValue::Integer(0); total];
        Some(Spreadsheet {
            grid,
            children: HashMap::new(),
            range_children: Vec::with_capacity(32),
            cell_meta: HashMap::new(),
            rows,
            cols,
            viewport_row: 0,
            viewport_col: 0,
            output_enabled: true,
            display_rows: 10,
            display_cols: 10,
            locked_ranges: Vec::new(),
            named_ranges: HashMap::new(),
            cell_history: HashMap::new(),
            last_edited: None,
        })
    }

    pub fn get_key(&self, row: i16, col: i16) -> i32 {
        (row as i32 * self.cols as i32 + col as i32) as i32
    }

    pub fn get_row_col(&self, key: i32) -> (i16, i16) {
        let row = (key / (self.cols as i32)) as i16;
        let col = (key % (self.cols as i32)) as i16;
        (row, col)
    }

    fn get_index(&self, row: i16, col: i16) -> usize {
        (row as usize) * (self.cols as usize) + (col as usize)
    }

    pub fn get_cell_meta(&mut self, row: i16, col: i16) -> &mut CellMeta {
        let key = self.get_key(row, col);
        self.cell_meta.entry(key).or_insert_with(CellMeta::new)
    }

    pub fn get_column_name(&self, mut col: i16) -> String {
        let mut temp_col = col + 1;
        let mut len = 0;
        while temp_col > 0 {
            len += 1;
            temp_col = (temp_col - 1) / 26;
        }
        col += 1;
        if col == 0 {
            return "A".to_string();
        }
        let mut buffer = vec![0; len];
        let mut i = len;
        while col > 0 {
            i -= 1;
            buffer[i] = b'A' + ((col - 1) % 26) as u8;
            col = (col - 1) / 26;
        }
        unsafe { String::from_utf8_unchecked(buffer) }
    }

    pub fn column_name_to_index(&self, name: &str) -> i16 {
        let bytes = name.as_bytes();
        let mut index: i16 = 0;
        for &b in bytes {
            index = index * 26 + ((b - b'A') as i16 + 1);
        }
        index - 1
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

    pub fn add_range_child(&mut self, start_key: i32, end_key: i32, child_key: i32) {
        self.range_children.push(RangeChild {
            start_key,
            end_key,
            child_key,
        });
    }

    pub fn remove_range_child(&mut self, child_key: i32) {
        self.range_children.retain(|rc| rc.child_key != child_key);
    }

    pub fn is_cell_in_range(&self, cell_key: i32, start_key: i32, end_key: i32) -> bool {
        let (cell_row, cell_col) = self.get_row_col(cell_key);
        let (start_row, start_col) = self.get_row_col(start_key);
        let (end_row, end_col) = self.get_row_col(end_key);
        cell_row >= start_row && cell_row <= end_row && cell_col >= start_col && cell_col <= end_col
    }

    pub fn add_child(&mut self, parent_key: &i32, child_key: &i32) {
        self.children
            .entry(*parent_key)
            .or_insert_with(|| Box::new(HashSet::with_capacity(5)))
            .insert(*child_key);
    }

    pub fn remove_child(&mut self, parent_key: i32, child_key: i32) {
        if let Some(children) = self.children.get_mut(&parent_key) {
            children.remove(&child_key);
            if children.is_empty() {
                self.children.remove(&parent_key);
            }
        }
    }

    pub fn get_cell_children(&self, key: i32) -> Option<&HashSet<i32>> {
        self.children.get(&key).map(|boxed_set| boxed_set.as_ref())
    }

    pub fn print_spreadsheet(&self) {
        if !self.output_enabled {
            return;
        }
        let start_row = self.viewport_row;
        let start_col = self.viewport_col;
        let display_row = min(self.rows - start_row, self.display_rows);
        let display_col = min(self.cols - start_col, self.display_cols);
        print!("     ");
        for i in 0..display_col {
            print!("{:<8} ", self.get_column_name(start_col + i));
        }
        println!();
        for i in 0..display_row {
            print!("{:<4} ", start_row + i + 1);
            for j in 0..display_col {
                let cell_value = self.get_cell(start_row + i, start_col + j);
                let value_str = match cell_value {
                    CellValue::Integer(value) => value.to_string(),
                    CellValue::Error => "ERR".to_string(),
                };
                print!("{:<8.8} ", value_str); 
            }
            println!();
        }
    }

    pub fn print_spreadsheet_with_highlights(
        &self,
        target_key: i32,
        highlight_parents: &HashSet<i32>,
        highlight_children: &HashSet<i32>,
    ) {
        if !self.output_enabled {
            return;
        }
        let start_row = self.viewport_row;
        let start_col = self.viewport_col;
        let display_row = min(self.rows - start_row, self.display_rows);
        let display_col = min(self.cols - start_col, self.display_cols);
        
        // Print column headers
        print!("     ");
        for i in 0..display_col {
            print!("{:<8} ", self.get_column_name(start_col + i));
        }
        println!();
        
        // Print rows with highlights
        for i in 0..display_row {
            print!("{:<4} ", start_row + i + 1);
            for j in 0..display_col {
                let row = start_row + i;
                let col = start_col + j;
                let key = self.get_key(row, col);
                let cell_value = self.get_cell(row, col);
                let value_str = match cell_value {
                    CellValue::Integer(value) => value.to_string(),
                    CellValue::Error => "ERR".to_string(),
                };
                
                let display_str = if value_str.len() > 8 {
                    &value_str[..8]
                } else {
                    value_str.as_str()
                };
                let padding = 8 - display_str.len();
                let pre_padding = if j > 0 { 1 } else { 0 };
                
                // Apply highlights
                let cell_str = if key == target_key {
                    format!("\x1b[4m{}\x1b[0m", display_str) // Underline
                } else if highlight_parents.contains(&key) {
                    format!("\x1b[31m{}\x1b[0m", display_str) // Red for parents
                } else if highlight_children.contains(&key) {
                    format!("\x1b[32m{}\x1b[0m", display_str) // Green for children
                } else {
                    display_str.to_string()
                };
                
                // Format with pre-padding, cell content, padding, and trailing space
                let formatted = format!("{}{}{} ", " ".repeat(pre_padding), cell_str, " ".repeat(padding));
                print!("{}", formatted);
            }
            println!();
        }
    }

    pub fn get_parents(&self, key: i32) -> HashSet<i32> {
        let mut parents = HashSet::new();
        if let Some(meta) = self.cell_meta.get(&key) {
            let rem = meta.formula % 10;
            if rem >= 5 && rem <= 9 {
                let (start_row, start_col) = self.get_row_col(meta.parent1);
                let (end_row, end_col) = self.get_row_col(meta.parent2);
                for r in start_row..=end_row {
                    for c in start_col..=end_col {
                        let pkey = self.get_key(r, c);
                        parents.insert(pkey);
                    }
                }
            } else if rem == 0 {
                if meta.parent1 >= 0 {
                    parents.insert(meta.parent1);
                }
                if meta.parent2 >= 0 {
                    parents.insert(meta.parent2);
                }
            } else if rem == 2 {
                if meta.parent1 >= 0 {
                    parents.insert(meta.parent1);
                }
            } else if rem == 3 {
                if meta.parent2 >= 0 {
                    parents.insert(meta.parent2);
                }
            } else if meta.formula == 82 || meta.formula == 102 {
                if meta.parent1 >= 0 {
                    parents.insert(meta.parent1);
                }
            }
        }
        parents
    }

    pub fn get_children(&self, key: i32) -> HashSet<i32> {
        let mut children = HashSet::new();
        if let Some(direct_children) = self.get_cell_children(key) {
            children.extend(direct_children);
        }
        for range_child in &self.range_children {
            if self.is_cell_in_range(key, range_child.start_key, range_child.end_key) {
                children.insert(range_child.child_key);
            }
        }
        children
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
            Err(_) => CommandStatus::CmdUnrecognized,
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
            _ => {}
        }
    }

    pub fn visualize_cell_relationships(&self, row: i16, col: i16) -> CommandStatus {
        visualize_cells::visualize_cell_relationships(self, row, col)
    }

    pub fn lock_range(&mut self, range: Range) {
        self.locked_ranges.push(range);
    }

    pub fn is_cell_locked(&self, row: i16, col: i16) -> bool {
        for range in &self.locked_ranges {
            if row >= range.start_row && row <= range.end_row && col >= range.start_col && col <= range.end_col {
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
        sheet.print_spreadsheet();
    }

    #[test]
    fn test_print_spreadsheet_with_values() {
        let mut sheet = Spreadsheet::create(5, 5).unwrap();
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(42);
        *sheet.get_mut_cell(1, 1) = CellValue::Error;
        sheet.output_enabled = true;
        sheet.print_spreadsheet();
    }

    #[test]
    fn test_last_edited() {
        let mut sheet = Spreadsheet::create(5, 5).unwrap();
        assert_eq!(sheet.last_edited, None);
        sheet.set_last_edited(2, 3);
        assert_eq!(sheet.last_edited, Some((2, 3)));
        sheet.scroll_to_last_edited();
        assert_eq!(sheet.viewport_row, 2);
        assert_eq!(sheet.viewport_col, 3);
    }
}