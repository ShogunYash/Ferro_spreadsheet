mod commands;
mod editor;

use crate::spreadsheet::Spreadsheet;
use std::io::{self, Write};

pub fn run_editor(sheet: &mut Spreadsheet, filename: Option<String>) {
    // Initialize vim mode editor state
    let mut editor_state = editor::EditorState::new();

    // If a filename was provided, load it and set it as saved file
    if let Some(file) = filename {
        editor_state.save_file = Some(file.clone());
        let _ = commands::load_spreadsheet(sheet, &file);
    }

    // Main editor loop
    loop {
        // Render the spreadsheet with cursor
        editor_state.render_spreadsheet(sheet);

        // Show command prompt
        print!("{} > ", editor_state.mode_display());
        io::stdout().flush().unwrap();

        // Get user input
        let mut input = String::with_capacity(128);
        if io::stdin().read_line(&mut input).unwrap() == 0 {
            break; // End of input
        }

        let trimmed = input.trim();
        
        // Handle command history navigation keys
        if trimmed == "\x10" {  // Ctrl+P for previous command
            if !editor_state.command_history.is_empty() {
                let prev_cmd = editor_state.navigate_history("up");
                // Print the previous command on a new line for the user to see/copy
                println!("\n[history] {}", prev_cmd);
            }
            continue;
        } else if trimmed == "\x0e" {  // Ctrl+N for next command
            let next_cmd = editor_state.navigate_history("down");
            // Print the next command on a new line for the user to see/copy
            println!("\n[history] {}", next_cmd);
            continue;
        } else {
            // For regular commands, use the input as is
            editor_state.command_buffer = trimmed.to_string();
        }

        // Handle special command to exit vim mode
        if trimmed == ":q!" {
            break;
        }

        // Process the command - note that history is now handled inside handle_vim_command
        let _ = commands::handle_vim_command(sheet, trimmed, &mut editor_state);

        // Handle special case for Esc key in terminal
        if trimmed == "\x1b" && editor_state.mode == editor::EditorMode::Insert {
            editor_state.mode = editor::EditorMode::Normal;
        }

        // Check for quit command
        if editor_state.should_quit {
            break;
        }
    }
}