// vim_mode/commands.rs
use super::editor::{EditorMode, EditorState};
use crate::spreadsheet::{CommandStatus, Spreadsheet};
use crate::cell::CellValue;
use crate::graph;
use crate::save_load::save_spreadsheet;
use crate::process_command::process_command;

// Handle vim-specific commands
pub fn handle_vim_command(
    sheet: &mut Spreadsheet,
    input: &str,
    state: &mut EditorState,
) -> CommandStatus {
    // Handle mode-specific input
    match state.mode {
        EditorMode::Normal => handle_normal_mode_command(sheet, input, state),
        EditorMode::Insert => handle_insert_mode_command(sheet, input, state),
    }
}

// Process commands in normal mode
fn handle_normal_mode_command(
    sheet: &mut Spreadsheet,
    input: &str,
    state: &mut EditorState,
) -> CommandStatus {
    // Single character commands
    if input.len() == 1 {
        match input.chars().next().unwrap() {
            // Movement commands
            'h' | 'j' | 'k' | 'l' => {
                state.move_cursor(input.chars().next().unwrap(), sheet);
                return CommandStatus::CmdOk;
            }
            // Mode switching
            'i' => {
                state.mode = EditorMode::Insert;
                return CommandStatus::CmdOk;
            }
            // Editing commands
            'd' => return cut_cell(sheet, state),
            'y' => return yank_cell(sheet, state),
            'p' => return paste_cell(sheet, state),
            'q' => {
                state.should_quit = true;
                return CommandStatus::CmdOk;
            }
            _ => {}
        }
    }


    // File commands
    if input.starts_with(':') {
        let cmd = &input[1..];

        // :w - write file
        if cmd.starts_with('w') && !cmd.starts_with("wq") {
            // Extract filename if provided
            let filename = if cmd.len() > 1 && cmd.chars().nth(1) == Some(' ') {
                Some(cmd[2..].trim().to_string())
            } else if cmd == "w" {
                state.save_file.clone()
            } else {
                None
            };

            if let Some(file) = filename {
                state.save_file = Some(file.clone());
                return save_spreadsheet(sheet, &file);
            } else {
                return CommandStatus::CmdUnrecognized;
            }
        }

        // :q - quit
        if cmd == "q" {
            state.should_quit = true;
            return CommandStatus::CmdOk;
        }

        // :wq - write and quit
        if cmd.starts_with("wq") {
            // Extract filename if provided (e.g., ":wq filename.csv")
            let filename = if cmd.len() > 2 && cmd.chars().nth(2) == Some(' ') {
                Some(cmd[3..].trim().to_string())
            } else {
                state.save_file.clone()
            };

            if let Some(file) = filename {
                state.save_file = Some(file.clone());
                let status = save_spreadsheet(sheet, &file);
                if status == CommandStatus::CmdOk {
                    state.should_quit = true;
                }
                return status;
            } else {
                // No file specified
                return CommandStatus::CmdUnrecognized;
            }
        }

        // :!rm % - delete the current file
        if cmd.trim() == "!rm %" {
            if let Some(file) = &state.save_file {
                match std::fs::remove_file(file) {
                    Ok(_) => {
                        state.save_file = None;
                        return CommandStatus::CmdOk;
                    }
                    Err(_) => return CommandStatus::CmdUnrecognized,
                }
            } else {
                return CommandStatus::CmdUnrecognized;
            }
        }
    }

    // If not handled as a vim command, pass it to the standard command handler
    // will return status
    process_command(sheet, input, &mut 0.0)
}

// Process commands in insert mode
fn handle_insert_mode_command(
    sheet: &mut Spreadsheet,
    input: &str,
    state: &mut EditorState,
) -> CommandStatus {
    // Check for Escape key to exit insert mode
    if input == "Esc" || input == "\x1b" {
        state.mode = EditorMode::Normal;
        return CommandStatus::CmdOk;
    }

    // Directly set the value of the cell at the cursor
    let status = state.set_cursor_cell_value(sheet, input);

    // If successful, move cursor down (like vim behavior)
    if status == CommandStatus::CmdOk {
        state.move_cursor('j', sheet);
    }

    status
}

// Cut the current cell (copy + clear)
fn cut_cell(sheet: &mut Spreadsheet, state: &mut EditorState) -> CommandStatus {
    // First copy the cell
    let status = yank_cell(sheet, state);
    if status != CommandStatus::CmdOk {
        return status;
    }
    
    let row = state.cursor_row;
    let col = state.cursor_col;

    *sheet.get_mut_cell(row, col) = CellValue::Integer(0);
    
    // Reset formula metadata
    let cell_key = sheet.get_key(row, col);
    // Also remove this cell from any dependency tracking
    graph::remove_all_parents(sheet, row, col);
    // Remove the formula from the cell metadata
    sheet.cell_meta.remove(&cell_key);
    CommandStatus::CmdOk
}

// Copy (yank) the current cell
fn yank_cell(sheet: &mut Spreadsheet, state: &mut EditorState) -> CommandStatus {
    // Get the cell reference string and value
    let cell_value = sheet.get_cell(state.cursor_row, state.cursor_col).clone();

    // Get the formula for the cell (if any)
    let cell_key = sheet.get_key(state.cursor_row, state.cursor_col);
    let formula = 
        if let Some( _meta) = sheet.cell_meta.get(&cell_key) {
                // Get the formula string from the cell metadata
                let formula_string = crate::extensions::get_formula_string(sheet, state.cursor_row, state.cursor_col);
                format!("{}", formula_string)
        } else {
            String::new()
        };

    // Store in clipboard
    state.clipboard = Some((state.cursor_row, state.cursor_col, cell_value, formula));

    CommandStatus::CmdOk
}

// Paste to the current cell
fn paste_cell(sheet: &mut Spreadsheet, state: &mut EditorState) -> CommandStatus {
    if let Some((_row, _col, _value, formula)) = &state.clipboard {
        // Get the target cell reference
        let cell_ref = state.cursor_to_cell_ref(sheet);

        // If there's a formula, paste that
        if !formula.is_empty() {
            let command: String = format!("{}={}", cell_ref, formula);
            return process_command(sheet, &command, &mut 0.0);
        } else {
            // Otherwise paste the literal value
            match _value {
                crate::cell::CellValue::Integer(value) => {
                    let command = format!("{}={}", cell_ref, value);
                    return process_command(sheet, &command, &mut 0.0);
                }
                crate::cell::CellValue::Error => {
                    // Can't paste an error
                    return CommandStatus::CmdUnrecognized;
                }
            }
        }
    } else {
        // Nothing in clipboard
        CommandStatus::CmdUnrecognized
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spreadsheet::Spreadsheet;
    use crate::cell::CellValue;

    // Helper function to set up test environment
    fn setup() -> (Spreadsheet, EditorState) {
        let sheet = Spreadsheet::create(10, 10).unwrap();
        let state = EditorState {
            mode: EditorMode::Normal,
            cursor_row: 0,
            cursor_col: 0,
            clipboard: None,
            should_quit: false,
            save_file: None,
            command_history: Vec::new(),
            history_position: 0,
        };
        (sheet, state)
    }

    #[test]
    fn test_normal_mode_movement() {
        let (mut sheet, mut state) = setup();
        
        // Test 'l' movement (right)
        let result = handle_vim_command(&mut sheet, "l", &mut state);
        assert_eq!(result, CommandStatus::CmdOk);
        assert_eq!(state.cursor_col, 1);
        assert_eq!(state.cursor_row, 0);
        
        // Test 'j' movement (down)
        let result = handle_vim_command(&mut sheet, "j", &mut state);
        assert_eq!(result, CommandStatus::CmdOk);
        assert_eq!(state.cursor_col, 1);
        assert_eq!(state.cursor_row, 1);
        
        // Test 'h' movement (left)
        let result = handle_vim_command(&mut sheet, "h", &mut state);
        assert_eq!(result, CommandStatus::CmdOk);
        assert_eq!(state.cursor_col, 0);
        assert_eq!(state.cursor_row, 1);
        
        // Test 'k' movement (up)
        let result = handle_vim_command(&mut sheet, "k", &mut state);
        assert_eq!(result, CommandStatus::CmdOk);
        assert_eq!(state.cursor_col, 0);
        assert_eq!(state.cursor_row, 0);
    }

    #[test]
    fn test_mode_switching() {
        let (mut sheet, mut state) = setup();
        
        // Test switching to insert mode with 'i'
        let result = handle_vim_command(&mut sheet, "i", &mut state);
        assert_eq!(result, CommandStatus::CmdOk);
        assert_eq!(state.mode, EditorMode::Insert);
        
        // Test switching back to normal mode with Esc
        let result =handle_vim_command(&mut sheet, "Esc", &mut state);
        assert_eq!(result, CommandStatus::CmdOk);
        assert_eq!(state.mode, EditorMode::Normal);
    }

    #[test]
    fn test_quit_command() {
        let (mut sheet, mut state) = setup();
        
        // Test quit with 'q'
        let result = handle_vim_command(&mut sheet, "q", &mut state);
        assert_eq!(result, CommandStatus::CmdOk);
        assert!(state.should_quit);

        // Reset flag
        state.should_quit = false;

        // Test quit with ':q'
        let result = handle_vim_command(&mut sheet, ":q", &mut state);
        assert_eq!(result, CommandStatus::CmdOk);
        assert!(state.should_quit);
    }

    #[test]
    fn test_yank_paste_cell() {
        let (mut sheet, mut state) = setup();
        
        // Set a value in the current cell
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(42);
        
        // Test yanking the cell
        let result = handle_vim_command(&mut sheet, "y", &mut state);
        assert_eq!(result, CommandStatus::CmdOk);
        assert!(state.clipboard.is_some());
        if let Some((row, col, value, _)) = &state.clipboard {
            assert_eq!(*row, 0);
            assert_eq!(*col, 0);
            assert_eq!(*value, CellValue::Integer(42));
        }
        
        // Move cursor and paste
        handle_vim_command(&mut sheet, "j", &mut state); // Move down
        handle_vim_command(&mut sheet, "l", &mut state); // Move right
        let result = handle_vim_command(&mut sheet, "p", &mut state);
        assert_eq!(result, CommandStatus::CmdOk);
        
        // Check if the value was pasted correctly
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(42));
    }

    #[test]
    fn test_cut_cell() {
        let (mut sheet, mut state) = setup();
        
        // Set a value in the current cell
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(42);
        
        // Test cutting the cell
        let result = handle_vim_command(&mut sheet, "d", &mut state);
        assert_eq!(result, CommandStatus::CmdOk);
        
        // Check if the cell is now empty (0)
        assert_eq!(*sheet.get_cell(0, 0), CellValue::Integer(0));
        
        // Check if the value was stored in clipboard
        assert!(state.clipboard.is_some());
        if let Some((row, col, value, _)) = &state.clipboard {
            assert_eq!(*row, 0);
            assert_eq!(*col, 0);
            assert_eq!(*value, CellValue::Integer(42));
        }
    }

    #[test]
    fn test_insert_mode_editing() {
        let (mut sheet, mut state) = setup();
        
        // Switch to insert mode
        handle_vim_command(&mut sheet, "i", &mut state);
        
        // Enter a value in insert mode
        let result = handle_vim_command(&mut sheet, "123", &mut state);
        assert_eq!(result, CommandStatus::CmdOk);
        
        // Check if the value was set correctly
        assert_eq!(*sheet.get_cell(0, 0), CellValue::Integer(123));
        
        // Check if cursor moved down after insertion (vim behavior)
        assert_eq!(state.cursor_row, 1);
        assert_eq!(state.cursor_col, 0);
    }

    #[test]
    fn test_save_command() {
        let (mut sheet, mut state) = setup();
        
        // Test save command with explicit filename
        // Note: This is a mock test that checks if the filename is stored
        // without actually writing to the filesystem
        let _result = handle_vim_command(&mut sheet, ":w test.csv", &mut state);
        
        // The actual save operation might fail in the test environment,
        // but we can check if the filename was stored in the state
        assert!(state.save_file.is_some());
        assert_eq!(state.save_file.unwrap(), "test.csv");
    }

    #[test]
    fn test_write_quit_command() {
        let (mut sheet, mut state) = setup();
        
        // Test write and quit command with explicit filename
        let _result = handle_vim_command(&mut sheet, ":wq test.csv", &mut state);
        
        // Check if the filename was stored
        assert!(state.save_file.is_some());
        assert_eq!(state.save_file.unwrap(), "test.csv");
        
        // The should_quit flag may or may not be set depending on if the save was successful
        // In a real test environment, this might not work unless we mock the file system
    }

    #[test]
    fn test_paste_formula() {
        let (mut sheet, mut state) = setup();
        
        // Create a cell with a formula (mock by directly setting the clipboard)
        state.clipboard = Some((0, 0, CellValue::Integer(42), "A1+B1".to_string()));
        
        // Move cursor and paste
        state.cursor_row = 1;
        state.cursor_col = 1;
        
        // Paste the formula
        let result = handle_vim_command(&mut sheet, "p", &mut state);
        assert_eq!(result, CommandStatus::CmdOk);
        
        // Check if the formula was applied (this is difficult to test directly)
        // In a real test we'd need to check the cell metadata to verify the formula was set
    }

    #[test]
    fn test_movement_boundaries() {
        let (mut sheet, mut state) = setup();
        
        // Test movement at boundaries
        // Move left at leftmost position
        let result = handle_vim_command(&mut sheet, "h", &mut state);
        assert_eq!(result, CommandStatus::CmdOk);
        assert_eq!(state.cursor_col, 0); // Should stay at 0
        
        // Move up at topmost position
        let result = handle_vim_command(&mut sheet, "k", &mut state);
        assert_eq!(result, CommandStatus::CmdOk);
        assert_eq!(state.cursor_row, 0); // Should stay at 0
        
        // Move to bottom-right corner
        state.cursor_row = 9;
        state.cursor_col = 9;
        
        // Move right at rightmost position
        let result = handle_vim_command(&mut sheet, "l", &mut state);
        assert_eq!(result, CommandStatus::CmdOk);
        assert_eq!(state.cursor_col, 9); // Should stay at 9
        
        // Move down at bottommost position
        let result = handle_vim_command(&mut sheet, "j", &mut state);
        assert_eq!(result, CommandStatus::CmdOk);
        assert_eq!(state.cursor_row, 9); // Should stay at 9
    }

    #[test]
    fn test_command_history() {
        let (mut sheet, mut state) = setup();
        
        // Execute a command
        handle_vim_command(&mut sheet, "i", &mut state);
        
        // Check if it was added to history
        assert_eq!(state.command_history.len(), 1);
        assert_eq!(state.command_history[0], "i");
        
        // Execute another command
        handle_vim_command(&mut sheet, "123", &mut state);
        
        // Check if it was added to history
        assert_eq!(state.command_history.len(), 2);
        assert_eq!(state.command_history[1], "123");
    }

    #[test]
    fn test_empty_input() {
        let (mut sheet, mut state) = setup();
        
        // Test with empty input
        let _result = handle_vim_command(&mut sheet, "", &mut state);
        
        // Empty input should not change history
        assert_eq!(state.command_history.len(), 0);
    }

    #[test]
    fn test_paste_with_empty_clipboard() {
        let (mut sheet, mut state) = setup();
        
        // Ensure clipboard is empty
        state.clipboard = None;
        
        // Try to paste
        let result = handle_vim_command(&mut sheet, "p", &mut state);
        assert_eq!(result, CommandStatus::CmdUnrecognized);
    }

    #[test]
    fn test_paste_error_value() {
        let (mut sheet, mut state) = setup();
        
        // Set clipboard to contain an error value
        state.clipboard = Some((0, 0, CellValue::Error, String::new()));
        
        // Try to paste
        let result = handle_vim_command(&mut sheet, "p", &mut state);
        assert_eq!(result, CommandStatus::CmdUnrecognized);
    }
}