mod commands;
mod editor;

use crate::spreadsheet::Spreadsheet;
use rustyline::{Config, Editor};

use crate::extensions::load_spreadsheet;

pub fn run_editor(sheet: &mut Spreadsheet, filename: Option<String>) {
    // Initialize vim mode editor state
    let mut editor_state = editor::EditorState::new();

    // If a filename was provided, load it and set it as saved file
    if let Some(file) = filename {
        editor_state.save_file = Some(file.clone());
        let _ = load_spreadsheet(sheet, &file);
    }

    // Configure and initialize rustyline
    let config = Config::builder()
        .history_ignore_dups(true)
        .history_ignore_space(true)
        .build();
    
    let mut rl = Editor::<()>::with_config(config).unwrap();
    
    // Load history from file if available
    let _ = rl.load_history("command_history.txt");

    // Main editor loop
    loop {
        // Render the spreadsheet with cursor
        editor_state.render_spreadsheet(sheet);

        // Create prompt based on the current mode
        let prompt = format!("{} > ", editor_state.mode_display());
        
        // Get user input with command history support
        let readline = rl.readline(&prompt);
        
        match readline {
            Ok(input) => {
                // Handle special command to exit vim mode
                if input == ":q!" {
                    break;
                }

                // Process the command if it's not empty
                if !input.trim().is_empty() {
                    // Add the command to history
                    rl.add_history_entry(&input);
                    editor_state.add_to_history(&input);
                    
                    // Process the command
                    let status= commands::handle_vim_command(sheet, &input, &mut editor_state);

                }

                // Handle special case for Esc key (will need to be entered as a literal escape or as a string "Esc")
                if input == ":esc" && editor_state.mode == editor::EditorMode::Insert {
                    editor_state.mode = editor::EditorMode::Normal;
                }

                // Check for quit command
                if editor_state.should_quit {
                    break;
                }
            },
            Err(_) => {
                // Handle errors or ctrl+c/ctrl+d
                println!("\nExiting editor...");
                break;
            }
        }
    }
    
    // Save history before exiting
    let _ = rl.save_history("command_history.txt");
}