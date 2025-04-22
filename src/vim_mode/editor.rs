// vim_mode/editor.rs
use crate::cell::CellValue;
use crate::spreadsheet::{CommandStatus, Spreadsheet};
use std::io::{self, Write};
use std::collections::HashSet;

// Define editor modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditorMode {
    Normal,
    Insert,
}

// Editor state structure
pub struct EditorState {
    pub mode: EditorMode,
    pub cursor_row: i16,
    pub cursor_col: i16,
    pub clipboard: Option<(i16, i16, CellValue, String)>, // (row, col, value, formula)
    pub should_quit: bool,
    pub save_file: Option<String>,
    // Command history
    pub command_history: Vec<String>,
    pub history_position: usize,
    pub current_input: String,

    // Highlighted cells
    pub highlighted_cells: HashSet<i16>,
    pub highlight_color: u8, // Current highlighting color
}
impl EditorState {
    pub fn new() -> Self {
        EditorState {
            mode: EditorMode::Normal,
            cursor_row: 0,
            cursor_col: 0,
            clipboard: None,
            should_quit: false,
            save_file: None,
            command_history: Vec::new(),
            history_position: 0,
            current_input: String::new(),
            highlighted_cells: HashSet::new(),
            highlight_color: 1, // Default highlight color (red)
        }
    }

    pub fn mode_display(&self) -> &str {
        match self.mode {
            EditorMode::Normal => "NORMAL",
            EditorMode::Insert => "INSERT",
        }
    }

    // Move cursor in the specified direction
    pub fn move_cursor(&mut self, direction: char, sheet: &mut Spreadsheet) {
        match direction {
            'h' => {
                if self.cursor_col > 0 {
                    self.cursor_col -= 1
                }
            }
            'j' => {
                if self.cursor_row < sheet.rows - 1 {
                    self.cursor_row += 1
                }
            }
            'k' => {
                if self.cursor_row > 0 {
                    self.cursor_row -= 1
                }
            }
            'l' => {
                if self.cursor_col < sheet.cols - 1 {
                    self.cursor_col += 1
                }
            }
            _ => {}
        }

        // Ensure viewport contains cursor
        self.adjust_viewport(sheet);
    }

    pub fn add_to_history(&mut self, command: &str) {
        // Don't add empty commands or duplicates of the most recent command
        if command.trim().is_empty()
            || (self
                .command_history
                .last()
                .map_or(false, |last| last == command))
        {
            return;
        }

        self.command_history.push(command.to_string());
        self.history_position = self.command_history.len();
    }

    // Adjust spreadsheet viewport to contain cursor
    pub fn adjust_viewport(&self, sheet: &mut Spreadsheet) {
        const VIEWPORT_SIZE: i16 = 10;

        // Adjust viewport row if cursor is outside
        if self.cursor_row < sheet.viewport_row {
            sheet.viewport_row = self.cursor_row;
        } else if self.cursor_row >= sheet.viewport_row + VIEWPORT_SIZE {
            sheet.viewport_row = self.cursor_row - VIEWPORT_SIZE + 1;
            if sheet.viewport_row < 0 {
                sheet.viewport_row = 0;
            }
        }

        // Adjust viewport column if cursor is outside
        if self.cursor_col < sheet.viewport_col {
            sheet.viewport_col = self.cursor_col;
        } else if self.cursor_col >= sheet.viewport_col + VIEWPORT_SIZE {
            sheet.viewport_col = self.cursor_col - VIEWPORT_SIZE + 1;
            if sheet.viewport_col < 0 {
                sheet.viewport_col = 0;
            }
        }
    }

    // Helper function to parse cell reference
    fn parse_cell_ref(&self, sheet: &Spreadsheet, cell_ref: &str) -> Option<(i16, i16)> {
        // Parse cell reference like "A1" into row and column
        let mut chars = cell_ref.chars();
        
        // Get column letter(s)
        let mut col_str = String::new();
        while let Some(c) = chars.next() {
            if c.is_alphabetic() {
                col_str.push(c.to_ascii_uppercase());
            } else {
                break;
            }
        }
        
        // Get row number
        let mut row_str = String::new();
        for c in cell_ref.chars().skip(col_str.len()) {
            if c.is_numeric() {
                row_str.push(c);
            } else {
                return None; // Invalid character in row
            }
        }
        
        // Convert row string to number (1-indexed to 0-indexed)
        if let Ok(row_num) = row_str.parse::<i16>() {
            if row_num > 0 && row_num <= sheet.rows {
                // Calculate column index
                let mut col_idx = 0;
                for c in col_str.chars() {
                    col_idx = col_idx * 26 + (c as i16 - 'A' as i16 + 1);
                }
                col_idx -= 1; // Convert to 0-indexed
                
                if col_idx >= 0 && col_idx < sheet.cols {
                    return Some((row_num - 1, col_idx));
                }
            }
        }
        
        None
    }

    // Get ANSI color code for a given cell
    fn get_cell_color_code(&self, sheet: &Spreadsheet, row: i16, col: i16) -> String {
        let cell_key = sheet.get_key(row, col);
        
        // Check if this cell is highlighted
        if self.highlighted_cells.contains(&(cell_key as i16)) {
            match self.highlight_color {
                1 => "\x1B[31m".to_string(), // Red
                2 => "\x1B[32m".to_string(), // Green
                3 => "\x1B[33m".to_string(), // Yellow
                4 => "\x1B[34m".to_string(), // Blue
                5 => "\x1B[35m".to_string(), // Magenta
                6 => "\x1B[36m".to_string(), // Cyan
                _ => "\x1B[37m".to_string(), // White (default)
            }
        } else {
            "\x1B[37m".to_string() // White (default)
        }
    }

    // Custom rendering function for vim mode with colored cells
    pub fn render_spreadsheet(&mut self, sheet: &Spreadsheet) {
        // Clear screen
        print!("\x1B[2J\x1B[1;1H");

        // Calculate visible area
        let start_row = sheet.viewport_row;
        let start_col = sheet.viewport_col;
        let end_row = std::cmp::min(start_row + 10, sheet.rows);
        let end_col = std::cmp::min(start_col + 10, sheet.cols);

        // Print column headers only once
        print!("     ");
        for col in start_col..end_col {
            print!("{:<8} ", sheet.get_column_name(col));
        }
        println!();

        // Print rows with data
        for row in start_row..end_row {
            print!("{:<4} ", row + 1); // Show 1-based row numbers

            for col in start_col..end_col {
                let cell_value = sheet.get_cell(row, col);
                let color_code = self.get_cell_color_code(sheet, row, col);

                // Highlight the cell under the cursor
                if row == self.cursor_row && col == self.cursor_col {
                    print!("\x1B[7m"); // Invert colors
                    match cell_value {
                        CellValue::Integer(value) => print!("{:<8}", value),
                        CellValue::Error => print!("{:<8}", "ERR"),
                    }
                    print!("\x1B[0m "); // Reset colors
                } else {
                    // Apply color to related cells
                    print!("{}", color_code);
                    match cell_value {
                        CellValue::Integer(value) => print!("{:<8}", value),
                        CellValue::Error => print!("{:<8}", "ERR"),
                    }
                    print!("\x1B[0m "); // Reset colors
                }
            }
            println!();
        }

        // Display status bar
        let col_letter = sheet.get_column_name(self.cursor_col);
        let cell_ref = format!("{}{}", col_letter, self.cursor_row + 1);

        // Get formula for current cell (if exists)
        let cell_key = sheet.get_key(self.cursor_row, self.cursor_col);
        let formula_str = if let Some(meta) = sheet.cell_meta.get(&cell_key) {
            if meta.formula != -1 {
                // Get the parent cells as references
                let parent1_ref = if meta.parent1 != -1 {
                    let (p1_row, p1_col) = sheet.get_row_col(meta.parent1);
                    format!("{}{}", sheet.get_column_name(p1_col), p1_row + 1)
                } else {
                    String::from("")
                };
                
                let parent2_ref = if meta.parent2 != -1 {
                    let (p2_row, p2_col) = sheet.get_row_col(meta.parent2);
                    format!("{}{}", sheet.get_column_name(p2_col), p2_row + 1)
                } else {
                    String::from("")
                };
                
                // Convert formula code to string
                if meta.formula == 10 {
                    format!("{}+{}", parent1_ref, parent2_ref)
                } else if meta.formula == 20 {
                    format!("{}-{}", parent1_ref, parent2_ref)
                } else if meta.formula == 40 {
                    format!("{}*{}", parent1_ref, parent2_ref)
                } else if meta.formula == 30 {
                    format!("{}/{}", parent1_ref, parent2_ref)
                } else {
                    let (row, col) = sheet.get_row_col(cell_key);
                    format!("{:?}", sheet.get_cell(row, col))
                }
            } else {
                "".to_string()
            }
        } else {
            "".to_string()
        };

        println!("\nCursor at: {} - {}", cell_ref, formula_str);

        // Display mode
        println!(
            "Mode: {} | Use hjkl to navigate, i to insert, Esc to exit insert mode",
            self.mode_display()
        );

        // If clipboard has content, show it
        if let Some((_, _, value, formula)) = &self.clipboard {
            println!("Clipboard: {:?}", value);
            if !formula.is_empty() {
                println!("Formula: {:?}", formula);
            }
        }

        // Show highlighting commands
        println!("Highlight: :hp CELL (parents), :hc CELL (children), :hf CELL (family)");

        io::stdout().flush().unwrap();
    }

    // Set the value of the cell at the cursor
    pub fn set_cursor_cell_value(&self, sheet: &mut Spreadsheet, value: &str) -> CommandStatus {
        let cell_ref = self.cursor_to_cell_ref(sheet);
        let command = format!("{}={}", cell_ref, value);
        let mut sleep_time = 0.0;
        crate::evaluator::handle_command(sheet, &command, &mut sleep_time)
    }

    // Convert cursor position to cell reference string (e.g., "A1")
    pub fn cursor_to_cell_ref(&self, sheet: &Spreadsheet) -> String {
        let col_letter = sheet.get_column_name(self.cursor_col);
        format!("{}{}", col_letter, self.cursor_row + 1)
    }
}