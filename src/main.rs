mod cell;
mod evaluator;
mod formula;
mod graph;
mod reevaluate_topo;
mod spreadsheet;
mod vim_mode;
mod visualize_cells;
mod extensions;
mod save_load;
use std::env;
use std::io::{self, Write};
use std::process;
use std::thread::sleep;
use std::time::{Duration, Instant};
use evaluator::handle_command;
use spreadsheet::CommandStatus;
use spreadsheet::Spreadsheet;
use crate::save_load::{save_spreadsheet, load_spreadsheet};
const DEFAULT_FILENAME: &str = "rust_spreadsheet.sheet";

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut vim_mode_enabled = false;
    let mut rows_arg_index = 1;
    let mut cols_arg_index = 2;

    if args.len() > 1 && args[1] == "--vim" {
        vim_mode_enabled = true;
        rows_arg_index = 2;
        cols_arg_index = 3;
    }

    // else if args.len() != 3 {
    //     eprintln!("Usage: {} <rows> <columns>", args[0]);
    //     process::exit(1);
    // }

    let rows: i16 = args[rows_arg_index].parse().unwrap_or_else(|_| {
        eprintln!("Invalid number for rows");
        process::exit(1);
    });

    let cols: i16 = args[cols_arg_index].parse().unwrap_or_else(|_| {
        eprintln!("Invalid number for columns");
        process::exit(1);
    });

    let mut sleep_time = 0.0; // Initialize sleep time
    let start = Instant::now();

    let mut sheet = match Spreadsheet::create(rows, cols) {
        Some(s) => s,
        None => {
            eprintln!(
                "Failed to create spreadsheet with dimensions {}x{}",
                rows, cols
            );
            eprintln!("Please try smaller dimensions.");
            process::exit(1);
        }
    };
    if vim_mode_enabled {
        // If args[4] exists, use it; else use default filename.
        let filename = if args.len() > 4 {
            Some(args[4].to_string())
        } else {
            // Check if DEFAULT_FILENAME exists, if not, create it.
            use std::fs::OpenOptions;
            use std::path::Path;
            if !Path::new(DEFAULT_FILENAME).exists() {
                let _ = OpenOptions::new()
                    .write(true)
                    .create(true)
                    .open(DEFAULT_FILENAME);
            }
            Some(DEFAULT_FILENAME.to_string())
        };
        vim_mode::run_editor(&mut sheet, filename);

    } else {
        let mut command_time = start.elapsed().as_secs_f64();
        let mut last_time = command_time; // Update last_time with the command time

        let mut last_status = "ok"; // Placeholder for last status
        let mut input = String::with_capacity(128);

        // Main loop for command input
        loop {
            // Print the spreadsheet 
            sheet.print_spreadsheet();

            print!("[{:.1}] ({}) > ", last_time, last_status);
            io::stdout().flush().unwrap(); // Ensure the prompt is shown

            input.clear();
            if io::stdin().read_line(&mut input).unwrap() == 0 {
                break; // End of input
            }

            let trimmed = input.trim(); // Remove any newline characters
            if trimmed == "q" {
                // Add save functionality before quitting ask the user 
                // if they want to save the spreadsheet
                print!("Do you want to save the spreadsheet before quitting? (y/n): ");
                io::stdout().flush().unwrap(); // Ensure the prompt is shown
                // Take the user's response
                let mut response = String::new();
                io::stdin().read_line(&mut response).unwrap();
                let response = response.trim(); // Remove any newline characters
                if response == "y" {
                    // Ask for the filename to save
                    print!("Enter filename to save (default: {}): ", DEFAULT_FILENAME);
                    io::stdout().flush().unwrap(); // Ensure the prompt is shown
                    let mut filename = String::new();
                    io::stdin().read_line(&mut filename).unwrap();
                    let filename = filename.trim(); // Remove any newline characters
                    
                    // Use the default filename if the user didn't enter anything
                    let save_filename = if filename.is_empty() {
                        DEFAULT_FILENAME
                    } else {
                        filename
                    };
                    save_spreadsheet(&sheet, save_filename);
                }
                break;
            }

            // Add "open" command to load a spreadsheet
            if trimmed.starts_with("open ") {
                let filename = trimmed[5..].trim();
                if !filename.is_empty() {
                    println!("Loading spreadsheet from '{}'...", filename);
                    match load_spreadsheet(&mut sheet, filename) {
                        CommandStatus::CmdOk => {
                            println!("Spreadsheet successfully loaded from '{}'", filename);
                            last_status = "ok";
                        },
                        _ => {
                            eprintln!("Failed to load spreadsheet from '{}'", filename);
                            last_status = "error";
                        }
                    }
                    continue;
                }
            }

            // Process the command and measure execution time
            let start = Instant::now();
            // Pass by reference instead of cloning
            let status = handle_command(&mut sheet, trimmed, &mut sleep_time);
            command_time = start.elapsed().as_secs_f64();

            if sleep_time <= command_time {
                sleep_time = 0.0;
            } else {
                sleep_time -= command_time;
            }
            last_time = command_time + sleep_time;
            if sleep_time > 0.0 {
                sleep(Duration::from_secs_f64(sleep_time));
            }
            sleep_time = 0.0;

            // Update last_status based on the current command status
            last_status = match status {
                CommandStatus::CmdOk => "ok",
                CommandStatus::CmdUnrecognized => "unrecognized_cmd",
                CommandStatus::CmdCircularRef => "circular_ref",
                CommandStatus::CmdInvalidCell => "invalid_cell",
                CommandStatus::CmdLockedCell => "locked_cell",
            };
        }
    }
}
