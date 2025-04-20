// vim_mode/mod.rs
mod editor;
mod commands;

use crate::spreadsheet::{Spreadsheet, CommandStatus};
use std::io::{self, Write};
use std::process;
use std::time::{Duration, Instant};

pub fn run_editor(sheet: &mut Spreadsheet) {
    // Initialize vim mode editor state
    let mut editor_state = editor::EditorState::new();
    let mut sleep_time = 0.0;
    let mut last_time = 0.0;
    let mut last_status = "ok";

    // Main editor loop
    loop {
        // Render the spreadsheet with cursor
        editor_state.render_spreadsheet(sheet);
        
        // Show status line with mode indication
        print!("[{:.1}s] ({}) {} > ", 
               last_time, 
               last_status, 
               editor_state.mode_display());
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
        
        // Process the command with timing
        let start = Instant::now();
        let status = commands::handle_vim_command(sheet, trimmed, &mut editor_state, &mut sleep_time);
        let command_time = start.elapsed().as_secs_f64();
        
        // Handle sleep time as in the original code
        if sleep_time <= command_time {
            sleep_time = 0.0;
        } else {
            sleep_time -= command_time;
        }
        last_time = command_time + sleep_time;
        if sleep_time > 0.0 {
            std::thread::sleep(Duration::from_secs_f64(sleep_time));
        }
        sleep_time = 0.0;
        
        // Update status and check for quit command
        last_status = match status {
            CommandStatus::CmdOk => "ok",
            CommandStatus::CmdUnrecognized => "unrecognized_cmd",
            CommandStatus::CmdCircularRef => "circular_ref",
            CommandStatus::CmdInvalidCell => "invalid_cell",
        };
        
        if editor_state.should_quit {
            break;
        }
    }
}