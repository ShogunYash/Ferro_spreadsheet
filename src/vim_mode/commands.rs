// vim_mode/commands.rs
use super::editor::{EditorMode, EditorState};
use crate::evaluator;
use crate::spreadsheet::{CommandStatus, Spreadsheet};
use crate::cell::CellValue;
use crate::graph;
use crate::extensions_2::save_spreadsheet;

// Handle vim-specific commands
pub fn handle_vim_command(
    sheet: &mut Spreadsheet,
    input: &str,
    state: &mut EditorState,
) -> CommandStatus {
    if !input.trim().is_empty() {
        state.add_to_history(input);
    }
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
    let mut sleep_time = 0.0;
    let status = evaluator::handle_command(sheet, input, &mut sleep_time);
    status
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
    if let Some(meta) = sheet.cell_meta.get_mut(&cell_key) {
        meta.formula = -1;  // No formula
        meta.parent1 = -1;  // No parents
        meta.parent2 = -1;
    }
    
    // Also remove this cell from any dependency tracking
    graph::remove_all_parents(sheet, row, col);
    CommandStatus::CmdOk
}

// Copy (yank) the current cell
fn yank_cell(sheet: &mut Spreadsheet, state: &mut EditorState) -> CommandStatus {
    // Get the cell reference string and value
    let cell_value = sheet.get_cell(state.cursor_row, state.cursor_col).clone();

    // Get the formula for the cell (if any)
    let cell_key = sheet.get_key(state.cursor_row, state.cursor_col);
    let formula = if let Some(meta) = sheet.cell_meta.get(&cell_key) {
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
                format!("{}", meta.formula)
            }
            
             
        } else {
            String::new()
        }
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
            let command = format!("{}={}", cell_ref, formula);
            return evaluator::handle_command(sheet, &command, &mut 0.0);
        } else {
            // Otherwise paste the literal value
            match _value {
                crate::cell::CellValue::Integer(value) => {
                    let command = format!("{}={}", cell_ref, value);
                    return evaluator::handle_command(sheet, &command, &mut 0.0);
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