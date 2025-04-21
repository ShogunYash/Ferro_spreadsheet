// vim_mode/commands.rs
use super::editor::{EditorMode, EditorState};
use crate::evaluator;
use crate::spreadsheet::{CommandStatus, Spreadsheet};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

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
    let mut sleep_time = 0.0;
    evaluator::handle_command(sheet, input, &mut sleep_time);
    CommandStatus::CmdOk
}

// Save spreadsheet to a file
fn save_spreadsheet(sheet: &Spreadsheet, filename: &str) -> CommandStatus {
    let path = Path::new(filename);

    // Determine file type (CSV or TSV) from extension
    let is_tsv = path.extension().map(|ext| ext == "tsv").unwrap_or(false);
    let delimiter = if is_tsv { '\t' } else { ',' };

    // Open file for writing
    let file = match OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
    {
        Ok(file) => file,
        Err(_) => return CommandStatus::CmdUnrecognized,
    };

    // Create a buffered writer
    let mut writer = std::io::BufWriter::new(file);

    // Write the spreadsheet data (write all cells, including zeros)
    for row in 0..sheet.rows {
        let mut line = String::new();

        for col in 0..sheet.cols {
            if col > 0 {
                line.push(delimiter);
            }
            match sheet.get_cell(row, col) {
                crate::cell::CellValue::Integer(value) => line.push_str(&value.to_string()),
                crate::cell::CellValue::Error => line.push_str("ERR"),
            }
        }

        // Write the line
        if let Err(_) = writeln!(writer, "{}", line) {
            return CommandStatus::CmdUnrecognized;
        }
    }

    CommandStatus::CmdOk
}

// Load a spreadsheet from a file
pub fn load_spreadsheet(sheet: &mut Spreadsheet, filename: &str) -> CommandStatus {
    let path = Path::new(filename);

    // Determine file type (CSV or TSV) from extension
    let is_tsv = path.extension().map(|ext| ext == "tsv").unwrap_or(false);
    let delimiter = if is_tsv { '\t' } else { ',' };

    // Open file for reading
    let file = match File::open(path) {
        Ok(file) => file,
        Err(_) => return CommandStatus::CmdUnrecognized,
    };

    // Create a buffered reader
    let reader = BufReader::new(file);

    // Read and parse the file
    for (row_idx, line_result) in reader.lines().enumerate() {
        if row_idx >= sheet.rows as usize {
            break; // Don't exceed sheet dimensions
        }

        let line = match line_result {
            Ok(line) => line,
            Err(_) => continue,
        };

        // Split line by delimiter
        for (col_idx, value) in line.split(delimiter).enumerate() {
            if col_idx >= sheet.cols as usize {
                break; // Don't exceed sheet dimensions
            }

            // Parse and set cell value
            if !value.is_empty() {
                let cell_ref = format!("{}{}", sheet.get_column_name(col_idx as i16), row_idx + 1);
                let command = format!("{}={}", cell_ref, value);

                // Process the command
                let mut sleep_time = 0.0;
                evaluator::handle_command(sheet, &command, &mut sleep_time);
            }
        }
    }

    CommandStatus::CmdOk
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

    // Then clear it by setting it to an empty value
    let cell_ref = state.cursor_to_cell_ref(sheet);

    // Use empty string to clear the cell
    let command = format!("{}=", cell_ref);
    evaluator::handle_command(sheet, &command, &mut 0.0);
    CommandStatus::CmdOk
}

// Copy (yank) the current cell
fn yank_cell(sheet: &mut Spreadsheet, state: &mut EditorState) -> CommandStatus {
    // Get the cell reference string and value
    let cell_ref = state.cursor_to_cell_ref(sheet);
    let cell_value = sheet.get_cell(state.cursor_row, state.cursor_col).clone();

    // Get the formula for the cell (if any)
    let cell_key = sheet.get_key(state.cursor_row, state.cursor_col);
    let formula = if let Some(meta) = sheet.cell_meta.get(&cell_key) {
        if meta.formula >= 0 {
            // In a real implementation, you would get the actual formula
            // This is a placeholder
            "placeholder_formula".to_string()
        } else {
            "".to_string()
        }
    } else {
        "".to_string()
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
