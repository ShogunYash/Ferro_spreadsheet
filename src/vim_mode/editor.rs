// vim_mode/editor.rs
use crate::cell::CellValue;
use crate::spreadsheet::{CommandStatus, Spreadsheet};
use std::io::{self, Write};

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
    pub command_buffer: String,
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
            command_buffer: String::new(),
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
            } // Fixed 'k' from 'u'
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

    // Navigate through command history
    pub fn navigate_history(&mut self, direction: &str) -> String {
        // If navigating history for the first time, save current input
        if self.history_position == self.command_history.len() && direction == "up" {
            self.current_input = self.command_buffer.clone();
        }

        match direction {
            "up" => {
                if self.history_position > 0 {
                    self.history_position -= 1;
                    self.command_history
                        .get(self.history_position)
                        .unwrap_or(&String::new())
                        .clone()
                } else {
                    self.command_history
                        .get(0)
                        .unwrap_or(&String::new())
                        .clone()
                }
            }
            "down" => {
                if self.history_position < self.command_history.len() - 1 {
                    self.history_position += 1;
                    self.command_history
                        .get(self.history_position)
                        .unwrap_or(&String::new())
                        .clone()
                } else {
                    self.history_position = self.command_history.len();
                    self.current_input.clone()
                }
            }
            _ => String::new(),
        }
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

    // Custom rendering function for vim mode
    pub fn render_spreadsheet(&self, sheet: &Spreadsheet) {
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

                // Highlight the cell under the cursor
                if row == self.cursor_row && col == self.cursor_col {
                    print!("\x1B[7m"); // Invert colors
                    match cell_value {
                        CellValue::Integer(value) => print!("{:<8}", value),
                        CellValue::Error => print!("{:<8}", "ERR"),
                    }
                    print!("\x1B[0m "); // Reset colors
                } else {
                    match cell_value {
                        CellValue::Integer(value) => print!("{:<8} ", value),
                        CellValue::Error => print!("{:<8} ", "ERR"),
                    }
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
            
                if meta.formula != 0 {
                    // Get the parent cells as references
               //get the parents , and the formula code converted to the actual operation
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
                    if meta.formula==10{
                        format!("{}+{}", parent1_ref, parent2_ref)
                    } else if meta.formula==20{
                        format!("{}-{}", parent1_ref, parent2_ref)
                    } else if meta.formula==40{
                        format!("{}*{}", parent1_ref, parent2_ref)
                    } else if meta.formula==30{
                        format!("{}/{}", parent1_ref, parent2_ref)
                    } else {
                        let (row, col) = sheet.get_row_col(cell_key);
                        format!("{:?}", sheet.get_cell(row,col))
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
