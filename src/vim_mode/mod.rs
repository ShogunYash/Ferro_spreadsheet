// vim_mode/mod.rs
mod editor;
mod commands;

use crate::spreadsheet::{Spreadsheet, CommandStatus};
use std::io::{self, Write};

pub fn run_editor(sheet: &mut Spreadsheet, filename: Option<String>) {
    // Initialize vim mode editor state
    let mut editor_state = editor::EditorState::new();
    
    // If a filename was provided, load it and set as save file
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
        
        // Handle special command to exit vim mode
        if trimmed == ":q!" {
            break;
        }
        
        // Process the command
        let status = commands::handle_vim_command(sheet, trimmed, &mut editor_state);
        
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