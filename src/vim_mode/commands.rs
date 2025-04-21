// vim_mode/commands.rs
use super::editor::{EditorMode, EditorState};
use crate::evaluator;
use crate::spreadsheet::{CommandStatus, Spreadsheet};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write, BufWriter};
use std::path::Path;
use crate::cell::{parse_cell_reference, CellValue};
use crate::graph;


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
pub fn save_spreadsheet(sheet: &Spreadsheet, filename: &str) -> CommandStatus {
    let path = Path::new(filename);
    
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
    let mut writer = BufWriter::new(file);

    // Write header with dimensions
    if let Err(_) = writeln!(writer, "DIMS,{},{}", sheet.rows, sheet.cols) {
        return CommandStatus::CmdUnrecognized;
    }

    // Write cell data with formulas
    for row in 0..sheet.rows {
        for col in 0..sheet.cols {
            let key = sheet.get_key(row, col);
            let cell_value = sheet.get_cell(row, col);
            
            // Only write cells with non-zero values or formulas
            let is_nonzero = match cell_value {
                CellValue::Integer(0) => false,
                _ => true,
            };
            
            // Check if cell has formula metadata
            let has_metadata = sheet.cell_meta.contains_key(&key);
            
            if is_nonzero || has_metadata {
                let cell_ref = format!("{}{}", sheet.get_column_name(col), row + 1);
                
                // Write the cell value
                match cell_value {
                    CellValue::Integer(val) => {
                        write!(writer, "CELL,{},{}", cell_ref, val).unwrap();
                    },
                    CellValue::Error => {
                        write!(writer, "CELL,{},ERR", cell_ref).unwrap();
                    },
                }
                
                // If the cell has formula metadata, write it too
                if let Some(meta) = sheet.cell_meta.get(&key) {
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
                        
                        write!(writer, ",FORMULA,{},{},{}", 
                            meta.formula, parent1_ref, parent2_ref).unwrap();
                    }
                }
                
                // End the line
                writeln!(writer, "").unwrap();
            }
        }
    }

    CommandStatus::CmdOk
}

// Load a spreadsheet from a file
pub fn load_spreadsheet(sheet: &mut Spreadsheet, filename: &str) -> CommandStatus {
    let path = Path::new(filename);

    // Open file for reading
    let file = match File::open(path) {
        Ok(file) => file,
        Err(_) => return CommandStatus::CmdUnrecognized,
    };

    // Create a buffered reader
    let reader = BufReader::new(file);
    
    // Clear the existing spreadsheet
    for row in 0..sheet.rows {
        for col in 0..sheet.cols {
            let key = sheet.get_key(row, col);
            // Clear cell value
            let index = sheet.get_index(row, col);
            sheet.grid[index] = CellValue::Integer(0);
            
            // Clear metadata and dependencies
            if sheet.cell_meta.contains_key(&key) {
                // Remove all parent-child relationships
                graph::remove_all_parents(sheet, row, col);
                sheet.cell_meta.remove(&key);
            }
        }
    }

    // Read and parse the file
    for line_result in reader.lines() {
        let line = match line_result {
            Ok(line) => line,
            Err(_) => continue,
        };
        
        let parts: Vec<&str> = line.split(',').collect();
        if parts.is_empty() {
            continue;
        }
        
        // Process line based on type
        match parts[0] {
            "DIMS" => {
                // Dimensions line: DIMS,rows,cols
                if parts.len() >= 3 {
                    // We don't resize the sheet here, just validate dimensions
                    let file_rows: i16 = parts[1].parse().unwrap_or(0);
                    let file_cols: i16 = parts[2].parse().unwrap_or(0);
                    
                    if file_rows > sheet.rows || file_cols > sheet.cols {
                        eprintln!("Warning: File contains a larger spreadsheet than current dimensions");
                    }
                }
            },
            "CELL" => {
                // Cell data line: CELL,ref,value[,FORMULA,formula_code,parent1,parent2]
                if parts.len() >= 3 {
                    let cell_ref = parts[1];
                    let value_str = parts[2];
                    
                    // Parse cell reference
                    if let Ok((row, col)) = parse_cell_reference(sheet, cell_ref) {
                        // Set cell value
                        let cell_value = if value_str == "ERR" {
                            CellValue::Error
                        } else {
                            match value_str.parse::<i32>() {
                                Ok(val) => CellValue::Integer(val),
                                Err(_) => continue,
                            }
                        };
                        
                        let index = sheet.get_index(row, col);
                        sheet.grid[index] = cell_value;
                        
                        // If there's formula data, process it
                        if parts.len() >= 6 && parts[3] == "FORMULA" {
                            let formula: i16 = parts[4].parse().unwrap_or(-1);
                            let parent1_ref = parts[5];
                            let parent2_ref = if parts.len() > 6 { parts[6] } else { "" };
                            
                            if formula != -1 {
                                // Get parent cell keys
                                let parent1_key = if !parent1_ref.is_empty() {
                                    if let Ok((p1_row, p1_col)) = parse_cell_reference(sheet, parent1_ref) {
                                        sheet.get_key(p1_row, p1_col)
                                    } else {
                                        -1
                                    }
                                } else {
                                    -1
                                };
                                
                                let parent2_key = if !parent2_ref.is_empty() {
                                    if let Ok((p2_row, p2_col)) = parse_cell_reference(sheet, parent2_ref) {
                                        sheet.get_key(p2_row, p2_col)
                                    } else {
                                        -1
                                    }
                                } else {
                                    -1
                                };
                                
                                // Set cell metadata
                                let key = sheet.get_key(row, col);
                                let meta = sheet.cell_meta.entry(key).or_insert_with(|| crate::spreadsheet::CellMeta::new());
                                meta.formula = formula;
                                meta.parent1 = parent1_key;
                                meta.parent2 = parent2_key;
                                
                                // Add dependencies
                                graph::add_children(sheet, parent1_key, parent2_key, formula, row, col);
                            }
                        }
                    }
                }
            },
            _ => continue,
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
