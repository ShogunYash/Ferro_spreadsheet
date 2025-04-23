// vim_mode/editor.rs
use crate::cell::CellValue;
use crate::process_command;
use crate::spreadsheet::{CommandStatus, Spreadsheet}; // <-- fix: import Spreadsheet as struct, not as trait
use std::io::{self, Write};
use crate::extensions::get_formula_string; // <-- fix: import get_formula_string from extensions

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

    // Custom rendering function for vim mode with colored cells
    pub fn render_spreadsheet(&mut self, sheet: &Spreadsheet) {

        // Clear screen
        print!("\x1B[2J\x1B[1;1H");
        // print the spreadsheet with cursor
        sheet.print_spreadsheet();

        // // Calculate visible area
        // let start_row = sheet.viewport_row;
        // let start_col = sheet.viewport_col;
        // let end_row = std::cmp::min(start_row + 10, sheet.rows);
        // let end_col = std::cmp::min(start_col + 10, sheet.cols);

        // // Print column headers only once
        // print!("     ");
        // for col in start_col..end_col {
        //     print!("{:<8} ", sheet.get_column_name(col));
        // }
        // println!();

        // // Print rows with data
        // for row in start_row..end_row {
        //     print!("{:<4} ", row + 1); // Show 1-based row numbers

        //     for col in start_col..end_col {
        //         let cell_value = sheet.get_cell(row, col);
        //         let color_code = self.get_cell_color_code(sheet, row, col);

        //         // Highlight the cell under the cursor
        //         if row == self.cursor_row && col == self.cursor_col {
        //             print!("\x1B[7m"); // Invert colors
        //             match cell_value {
        //                 CellValue::Integer(value) => print!("{:<8}", value),
        //                 CellValue::Error => print!("{:<8}", "ERR"),
        //             }
        //             print!("\x1B[0m "); // Reset colors
        //         } else {
        //             // Apply color to related cells
        //             print!("{}", color_code);
        //             match cell_value {
        //                 CellValue::Integer(value) => print!("{:<8}", value),
        //                 CellValue::Error => print!("{:<8}", "ERR"),
        //             }
        //             print!("\x1B[0m "); // Reset colors
        //         }
        //     }
        //     println!();
        // }

        // Display status bar
        let col_letter = sheet.get_column_name(self.cursor_col);
        let cell_ref = format!("{}{}", col_letter, self.cursor_row + 1);

        // Get formula for current cell (if exists)
        let cell_key = sheet.get_key(self.cursor_row, self.cursor_col);
        let formula_str = 
            if let Some(_meta) = sheet.cell_meta.get(&cell_key) {
                get_formula_string(sheet, self.cursor_row, self.cursor_col)
            } else {
                "".to_string()
            };

        println!("\nCursor at: {} - {}", cell_ref, formula_str);

        // Display mode
        println!(
            "Mode: {} | Use h|j|k|l to navigate, i to insert, Esc to exit insert mode",
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
        println!("Highlight: :HLP (parents), :HLC (children), :HLPC (family)");

        io::stdout().flush().unwrap();
    }

    // Set the value of the cell at the cursor
    pub fn set_cursor_cell_value(&self, sheet: &mut Spreadsheet, value: &str) -> CommandStatus {
        let cell_ref = self.cursor_to_cell_ref(sheet);
        let command = format!("{}={}", cell_ref, value);
        process_command::process_command(sheet, &command, &mut 0.0)
    }

    // Convert cursor position to cell reference string (e.g., "A1")
    pub fn cursor_to_cell_ref(&self, sheet: &Spreadsheet) -> String {
        let col_letter = sheet.get_column_name(self.cursor_col);
        format!("{}{}", col_letter, self.cursor_row + 1)
    }
}

#[cfg(test)]
mod tests {


use super::*;
use crate::cell::CellValue;
use crate::spreadsheet::Spreadsheet;

#[test]
fn test_new_editor_state() {
    let state = EditorState::new();
    assert_eq!(state.mode, EditorMode::Normal);
    assert_eq!(state.cursor_row, 0);
    assert_eq!(state.cursor_col, 0);
    assert_eq!(state.should_quit, false);
    assert_eq!(state.clipboard, None);
    assert_eq!(state.save_file, None);
    assert!(state.command_history.is_empty());
}

#[test]
fn test_mode_display() {
    let mut state = EditorState::new();
    assert_eq!(state.mode_display(), "NORMAL");
    state.mode = EditorMode::Insert;
    assert_eq!(state.mode_display(), "INSERT");
}

#[test]
fn test_move_cursor() {
    let mut state = EditorState::new();
    let mut sheet = Spreadsheet::create(10, 10).unwrap();

    // Test right movement
    state.move_cursor('l', &mut sheet);
    assert_eq!(state.cursor_col, 1);
    assert_eq!(state.cursor_row, 0);

    // Test down movement
    state.move_cursor('j', &mut sheet);
    assert_eq!(state.cursor_col, 1);
    assert_eq!(state.cursor_row, 1);

    // Test left movement
    state.move_cursor('h', &mut sheet);
    assert_eq!(state.cursor_col, 0);
    assert_eq!(state.cursor_row, 1);

    // Test up movement
    state.move_cursor('k', &mut sheet);
    assert_eq!(state.cursor_col, 0);
    assert_eq!(state.cursor_row, 0);

    // Test edge cases - cannot move beyond boundaries
    // Left edge
    state.cursor_col = 0;
    state.move_cursor('h', &mut sheet);
    assert_eq!(state.cursor_col, 0);

    // Top edge
    state.cursor_row = 0;
    state.move_cursor('k', &mut sheet);
    assert_eq!(state.cursor_row, 0);

    // Right edge
    state.cursor_col = sheet.cols - 1;
    state.move_cursor('l', &mut sheet);
    assert_eq!(state.cursor_col, sheet.cols - 1);

    // Bottom edge
    state.cursor_row = sheet.rows - 1;
    state.move_cursor('j', &mut sheet);
    assert_eq!(state.cursor_row, sheet.rows - 1);

    // Invalid direction
    state.cursor_row = 5;
    state.cursor_col = 5;
    state.move_cursor('x', &mut sheet);
    assert_eq!(state.cursor_row, 5);
    assert_eq!(state.cursor_col, 5);
}

#[test]
fn test_add_to_history() {
    let mut state = EditorState::new();
    
    // Add first command
    state.add_to_history("A1=10");
    assert_eq!(state.command_history.len(), 1);
    assert_eq!(state.command_history[0], "A1=10");
    assert_eq!(state.history_position, 1);

    // Add second command
    state.add_to_history("B1=20");
    assert_eq!(state.command_history.len(), 2);
    assert_eq!(state.command_history[1], "B1=20");
    assert_eq!(state.history_position, 2);

    // Empty commands should not be added
    state.add_to_history("");
    assert_eq!(state.command_history.len(), 2);

    // Whitespace-only commands should not be added
    state.add_to_history("   ");
    assert_eq!(state.command_history.len(), 2);

    // Duplicate of the last command should not be added
    state.add_to_history("B1=20");
    assert_eq!(state.command_history.len(), 2);
    
    // But a different command should be added
    state.add_to_history("C1=30");
    assert_eq!(state.command_history.len(), 3);
    assert_eq!(state.command_history[2], "C1=30");
    assert_eq!(state.history_position, 3);
}

#[test]
fn test_adjust_viewport() {
    // Starting state
    let state = EditorState {
        mode: EditorMode::Normal,
        cursor_row: 5,
        cursor_col: 5,
        clipboard: None,
        should_quit: false,
        save_file: None,
        command_history: Vec::new(),
        history_position: 0,
    };

    // Test 1: Cursor is within viewport
    let mut sheet = Spreadsheet::create(20, 20).unwrap();
    sheet.viewport_row = 0;
    sheet.viewport_col = 0;
    state.adjust_viewport(&mut sheet);
    assert_eq!(sheet.viewport_row, 0);
    assert_eq!(sheet.viewport_col, 0);

    // Test 2: Cursor is below viewport
    let mut sheet = Spreadsheet::create(20, 20).unwrap();
    sheet.viewport_row = 0;
    sheet.viewport_col = 0;
    let state = EditorState { cursor_row: 15, cursor_col: 5, ..state };
    state.adjust_viewport(&mut sheet);
    assert_eq!(sheet.viewport_row, 6);
    assert_eq!(sheet.viewport_col, 0);

    // Test 3: Cursor is to the right of viewport
    let mut sheet = Spreadsheet::create(20, 20).unwrap();
    sheet.viewport_row = 0;
    sheet.viewport_col = 0;
    let state = EditorState { cursor_row: 5, cursor_col: 15, ..state };
    state.adjust_viewport(&mut sheet);
    assert_eq!(sheet.viewport_row, 0);
    assert_eq!(sheet.viewport_col, 6);

    // Test 4: Cursor is above viewport
    let mut sheet = Spreadsheet::create(20, 20).unwrap();
    sheet.viewport_row = 10;
    sheet.viewport_col = 0;
    let state = EditorState { cursor_row: 5, cursor_col: 5, ..state };
    state.adjust_viewport(&mut sheet);
    assert_eq!(sheet.viewport_row, 5);
    assert_eq!(sheet.viewport_col, 0);

    // Test 5: Cursor is to the left of viewport
    let mut sheet = Spreadsheet::create(20, 20).unwrap();
    sheet.viewport_row = 0;
    sheet.viewport_col = 10;
    let state = EditorState { cursor_row: 5, cursor_col: 5, ..state };
    state.adjust_viewport(&mut sheet);
    assert_eq!(sheet.viewport_row, 0);
    assert_eq!(sheet.viewport_col, 5);
    
    // Test 6: Edge case - cursor at (0,0) with viewport elsewhere
    let mut sheet = Spreadsheet::create(20, 20).unwrap();
    sheet.viewport_row = 5;
    sheet.viewport_col = 5;
    let state = EditorState { cursor_row: 0, cursor_col: 0, ..state };
    state.adjust_viewport(&mut sheet);
    assert_eq!(sheet.viewport_row, 0);
    assert_eq!(sheet.viewport_col, 0);
    
    // Test 7: Edge case - cursor at max position with viewport at start
    let mut sheet = Spreadsheet::create(20, 20).unwrap();
    sheet.viewport_row = 0;
    sheet.viewport_col = 0;
    let state = EditorState { cursor_row: 19, cursor_col: 19, ..state };
    state.adjust_viewport(&mut sheet);
    assert_eq!(sheet.viewport_row, 10);
    assert_eq!(sheet.viewport_col, 10);
}

#[test]
fn test_parse_cell_ref() {
    let sheet = Spreadsheet::create(20, 26).unwrap();
    
    // This is a private method, so we need to test it indirectly
    // We can do this by testing methods that use it or testing
    // the behavior it enables. Let's make sure cursor_to_cell_ref works.
    
    let mut editor = EditorState::new();
    editor.cursor_row = 0;
    editor.cursor_col = 0;
    assert_eq!(editor.cursor_to_cell_ref(&sheet), "A1");
    
    editor.cursor_row = 2;
    editor.cursor_col = 3;
    assert_eq!(editor.cursor_to_cell_ref(&sheet), "D3");
    
    editor.cursor_row = 19;
    editor.cursor_col = 25;
    assert_eq!(editor.cursor_to_cell_ref(&sheet), "Z20");
}

#[test]
fn test_cursor_to_cell_ref() {
    let mut state = EditorState::new();
    let sheet = Spreadsheet::create(10, 30).unwrap();

    // Test simple cases
    state.cursor_row = 0;
    state.cursor_col = 0;
    assert_eq!(state.cursor_to_cell_ref(&sheet), "A1");

    state.cursor_row = 2;
    state.cursor_col = 3;
    assert_eq!(state.cursor_to_cell_ref(&sheet), "D3");

    // Test multi-letter column names
    state.cursor_row = 0;
    state.cursor_col = 26;
    assert_eq!(state.cursor_to_cell_ref(&sheet), "AA1");
    
    state.cursor_row = 9;
    state.cursor_col = 29;
    assert_eq!(state.cursor_to_cell_ref(&sheet), "AD10");
}

#[test]
fn test_editor_with_large_spreadsheet() {
    // Test with a larger spreadsheet to ensure everything scales properly
    let mut state = EditorState::new();
    let mut sheet = Spreadsheet::create(100, 100).unwrap();
    
    // Navigate to a cell way outside the initial viewport
    state.cursor_row = 50;
    state.cursor_col = 50;
    state.adjust_viewport(&mut sheet);
    
    // Ensure viewport adjusted correctly
    assert!(sheet.viewport_row <= state.cursor_row);
    assert!(sheet.viewport_row + 10 > state.cursor_row);
    assert!(sheet.viewport_col <= state.cursor_col);
    assert!(sheet.viewport_col + 10 > state.cursor_col);
    
    // Set a value at this position
    let _ = state.set_cursor_cell_value(&mut sheet, "99");
    
    // Verify the cell value
    match sheet.get_cell(50, 50) {
        CellValue::Integer(value) => assert_eq!(*value, 99),
        _ => panic!("Expected Integer cell value"),
    }
    
    // Test cursor movement around edges
    state.cursor_row = 99;
    state.cursor_col = 99;
    state.move_cursor('j', &mut sheet); // Try to move down (should stay at 99)
    state.move_cursor('l', &mut sheet); // Try to move right (should stay at 99)
    assert_eq!(state.cursor_row, 99);
    assert_eq!(state.cursor_col, 99);
}

#[test]
fn test_clipboard_functionality() {
    let mut state = EditorState::new();
    
    // Set up clipboard with simple data
    state.clipboard = Some((0, 0, CellValue::Integer(42), "=A1+B1".to_string()));
    
    // Verify clipboard contents
    match &state.clipboard {
        Some((row, col, value, formula)) => {
            assert_eq!(*row, 0);
            assert_eq!(*col, 0);
            assert_eq!(*value, CellValue::Integer(42));
            assert_eq!(*formula, "=A1+B1");
        },
        None => panic!("Expected clipboard to contain data"),
    }
    
    // Clear clipboard
    state.clipboard = None;
    assert!(state.clipboard.is_none());
}

#[test]
fn test_editor_save_file() {
    let mut state = EditorState::new();
    
    // Initially no save file is set
    assert!(state.save_file.is_none());
    
    // Set a save file
    state.save_file = Some("test_spreadsheet.ss".to_string());
    
    match &state.save_file {
        Some(filename) => assert_eq!(filename, "test_spreadsheet.ss"),
        None => panic!("Expected save file to be set"),
    }
    
    // Change save file
    state.save_file = Some("new_filename.ss".to_string());
    
    match &state.save_file {
        Some(filename) => assert_eq!(filename, "new_filename.ss"),
        None => panic!("Expected save file to be set"),
    }
    
    // Clear save file
    state.save_file = None;
    assert!(state.save_file.is_none());
}

#[test]
fn test_command_history_navigation() {
    let mut state = EditorState::new();
    
    // Add some commands to history
    state.add_to_history("A1=10");
    state.add_to_history("B1=20");
    state.add_to_history("C1=30");
    
    assert_eq!(state.command_history.len(), 3);
    assert_eq!(state.history_position, 3);
    
    // Move back in history
    if state.history_position > 0 {
        state.history_position -= 1;
    }
    assert_eq!(state.history_position, 2);
    assert_eq!(state.command_history[state.history_position], "C1=30");
    
    // Move back again
    if state.history_position > 0 {
        state.history_position -= 1;
    }
    assert_eq!(state.history_position, 1);
    assert_eq!(state.command_history[state.history_position], "B1=20");
    
    // Move forward
    if state.history_position < state.command_history.len() {
        state.history_position += 1;
    }
    assert_eq!(state.history_position, 2);
    assert_eq!(state.command_history[state.history_position], "C1=30");
}

#[test]
fn test_editor_mode_switching() {
    let mut state = EditorState::new();
    
    // Default is normal mode
    assert_eq!(state.mode, EditorMode::Normal);
    
    // Switch to insert mode
    state.mode = EditorMode::Insert;
    assert_eq!(state.mode, EditorMode::Insert);
    
    // Switch back to normal mode
    state.mode = EditorMode::Normal;
    assert_eq!(state.mode, EditorMode::Normal);
}
#[test]
fn test_render_spreadsheet() {
    // Since render_spreadsheet prints to stdout, we can't easily capture and test its output
    // in a unit test. However, we can test that it runs without crashing
    // and that it correctly updates the state when needed.
    
    // Create test state and spreadsheet
    let mut state = EditorState::new();
    let sheet = Spreadsheet::create(10, 10).unwrap();
    
    // Basic test - should run without panicking
    state.render_spreadsheet(&sheet);
    
    // Test with cursor at different positions
    state.cursor_row = 5;
    state.cursor_col = 5;
    state.render_spreadsheet(&sheet);
    
    // Test with clipboard data
    state.clipboard = Some((1, 1, CellValue::Integer(42), "=A1+B1".to_string()));
    state.render_spreadsheet(&sheet);
    
    // Test with different modes
    state.mode = EditorMode::Insert;
    state.render_spreadsheet(&sheet);
    state.mode = EditorMode::Normal;
    state.render_spreadsheet(&sheet);
    
    // Test with a cell that has a formula
    let mut sheet_with_formula = Spreadsheet::create(10, 10).unwrap();
    let _ = process_command::process_command(&mut sheet_with_formula, "A1=10", &mut 0.0);
    let _ = process_command::process_command(&mut sheet_with_formula, "B1=A1*2", &mut 0.0);
    state.cursor_row = 0;
    state.cursor_col = 1; // B1
    state.render_spreadsheet(&sheet_with_formula);
}

#[test]
fn test_adjust_viewport_negative_conditions() {
    let mut state = EditorState::new();
    let mut sheet = Spreadsheet::create(20, 20).unwrap();
    
    // Test case where viewport would go negative after adjustment
    sheet.viewport_row = 5;
    sheet.viewport_col = 5;
    
    // Place cursor at position 0,0 - this should pull viewport back to 0,0
    state.cursor_row = 0;
    state.cursor_col = 0;
    state.adjust_viewport(&mut sheet);
    
    // Verify viewport is at 0,0 (not negative)
    assert_eq!(sheet.viewport_row, 0);
    assert_eq!(sheet.viewport_col, 0);
    
    // Test cursor at far edge with a large viewport position
    // that would need to pull back but not go negative
    sheet.viewport_row = 15;
    sheet.viewport_col = 15;
    
    state.cursor_row = 8; // Should pull viewport to 8
    state.cursor_col = 10; // Should pull viewport to 10
    state.adjust_viewport(&mut sheet);
    
    assert_eq!(sheet.viewport_row, 8);
    assert_eq!(sheet.viewport_col, 10);
    
    // Test edge case with cursor at max but viewport could go negative
    // First set a weird state - viewport is negative (shouldn't happen but let's test it)
    sheet.viewport_row = -5;
    sheet.viewport_col = -5;
    
    // Now adjust viewport with cursor at position that should normalize viewport
    state.cursor_row = 15;
    state.cursor_col = 15;
    state.adjust_viewport(&mut sheet);
    
    // The adjust_viewport function doesn't explicitly handle negative values,
    // so let's check the outcome (it should adjust based on cursor position)
    assert!(sheet.viewport_row <= state.cursor_row);
    assert!(sheet.viewport_row + 10 > state.cursor_row);
    assert!(sheet.viewport_col <= state.cursor_col);
    assert!(sheet.viewport_col + 10 > state.cursor_col);
}

#[test]
fn test_adjust_viewport_complex_movements() {
    let mut state = EditorState::new();
    let mut sheet = Spreadsheet::create(30, 30).unwrap();
    
    // Start at the beginning
    state.cursor_row = 0;
    state.cursor_col = 0;
    sheet.viewport_row = 0;
    sheet.viewport_col = 0;
    
    // Move cursor down beyond viewport
    state.cursor_row = 15;
    state.adjust_viewport(&mut sheet);
    assert_eq!(sheet.viewport_row, 6); // Viewport should adjust to show cursor
    
    // Move cursor right beyond viewport
    state.cursor_col = 15;
    state.adjust_viewport(&mut sheet);
    assert_eq!(sheet.viewport_col, 6); // Viewport should adjust to show cursor
    
    // Move cursor back to top-left but keep viewport where it is
    state.cursor_row = 7; // Still visible in current viewport
    state.cursor_col = 7; // Still visible in current viewport
    state.adjust_viewport(&mut sheet);
    assert_eq!(sheet.viewport_row, 6); // Should not change
    assert_eq!(sheet.viewport_col, 6); // Should not change
    
    // Move cursor to boundary edge of viewport
    state.cursor_row = 6; // Exactly at viewport edge
    state.cursor_col = 6; // Exactly at viewport edge
    state.adjust_viewport(&mut sheet);
    assert_eq!(sheet.viewport_row, 6); // Should not change
    assert_eq!(sheet.viewport_col, 6); // Should not change
    
    // Move cursor just outside viewport boundary
    state.cursor_row = 5; // Just outside viewport
    state.cursor_col = 5; // Just outside viewport
    state.adjust_viewport(&mut sheet);
    assert_eq!(sheet.viewport_row, 5); // Should adjust to show cursor
    assert_eq!(sheet.viewport_col, 5); // Should adjust to show cursor
}
}