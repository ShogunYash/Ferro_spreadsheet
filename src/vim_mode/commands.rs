// vim_mode/commands.rs
use crate::spreadsheet::{Spreadsheet, CommandStatus};
use crate::evaluator;
use super::editor::{EditorState, EditorMode};

// Handle vim-specific commands
pub fn handle_vim_command(
    sheet: &mut Spreadsheet,
    input: &str,
    state: &mut EditorState,
    sleep_time: &mut f64,
) -> CommandStatus {
    // Handle mode-specific input
    match state.mode {
        EditorMode::Normal => handle_normal_mode_command(sheet, input, state, sleep_time),
        EditorMode::Insert => handle_insert_mode_command(sheet, input, state, sleep_time),
    }
}

// Process commands in normal mode
fn handle_normal_mode_command(
    sheet: &mut Spreadsheet,
    input: &str,
    state: &mut EditorState,
    sleep_time: &mut f64,
) -> CommandStatus {
    // Single character commands
    if input.len() == 1 {
        match input.chars().next().unwrap() {
            // Movement commands
            'h' | 'j' | 'k' | 'l' => {
                state.move_cursor(input.chars().next().unwrap(), sheet);
                return CommandStatus::CmdOk;
            },
            // Mode switching
            'i' => {
                state.mode = EditorMode::Insert;
                return CommandStatus::CmdOk;
            },
            // Editing commands
            'd' => return cut_cell(sheet, state),
            'y' => return yank_cell(sheet, state),
            'p' => return paste_cell(sheet, state, sleep_time),
            'q' => {
                state.should_quit = true;
                return CommandStatus::CmdOk;
            },
            _ => {}
        }
    }
    
    // File commands
    if input.starts_with(':') {
        let cmd = &input[1..];
        
        // :w - write file
        if cmd.starts_with('w') {
            // This is a placeholder - actual file writing would need to be implemented
            // or delegated to a spreadsheet method
            return CommandStatus::CmdOk;
        }
        
        // :q - quit
        if cmd == "q" {
            state.should_quit = true;
            return CommandStatus::CmdOk;
        }
        
        // :wq - write and quit
        if cmd == "wq" {
            // Write file (placeholder)
            state.should_quit = true;
            return CommandStatus::CmdOk;
        }
    }

    // If not handled as a vim command, pass it to the standard command handler
    evaluator::handle_command(sheet, input, sleep_time)
}

// Process commands in insert mode
fn handle_insert_mode_command(
    sheet: &mut Spreadsheet,
    input: &str,
    state: &mut EditorState,
    sleep_time: &mut f64,
) -> CommandStatus {
    // Check for Escape key to exit insert mode
    if input == "Esc" || input == "\x1b" {
        state.mode = EditorMode::Normal;
        return CommandStatus::CmdOk;
    }
    
    // Otherwise, treat input as cell content
    // For insert mode, we format a cell assignment command
    let cell_ref = state.cursor_to_cell_ref();
    let command = format!("{}={}", cell_ref, input);
    
    // Use the standard command handler
    let status = evaluator::handle_command(sheet, &command, sleep_time);
    
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
    let cell_ref = state.cursor_to_cell_ref();
    let mut sleep_time = 0.0;
    
    // Use empty string to clear the cell
    let command = format!("{}=", cell_ref);
    evaluator::handle_command(sheet, &command, &mut sleep_time)
}

// Copy (yank) the current cell
fn yank_cell(sheet: &mut Spreadsheet, state: &mut EditorState) -> CommandStatus {
    // Get the cell reference string
    let cell_ref = state.cursor_to_cell_ref();
    
    // This is a placeholder - you would need to implement
    // a way to get the cell's actual content or formula
    // For now, we're just storing the cell reference itself
    state.clipboard = Some((state.cursor_row, state.cursor_col, cell_ref));
    
    CommandStatus::CmdOk
}

// Paste to the current cell
fn paste_cell(sheet: &mut Spreadsheet, state: &mut EditorState, sleep_time: &mut f64) -> CommandStatus {
    if let Some((_row, _col, content)) = &state.clipboard {
        // Get the target cell reference
        let cell_ref = state.cursor_to_cell_ref();
        
        // This is just a placeholder - in a real implementation,
        // you would paste the actual cell content or formula
        // For now, we're just creating a command that sets the cell to
        // reference the copied cell
        let command = format!("{}={}", cell_ref, content);
        evaluator::handle_command(sheet, &command, sleep_time)
    } else {
        // Nothing in clipboard
        CommandStatus::CmdUnrecognized
    }
}