//! A simple spreadsheet application in Rust.
//!
//! Supports basic operations (cell assignments, formulas), advanced features (locking, named ranges),
//! and an optional Vim-like editing mode.

mod cell;
mod evaluator;
mod extensions;
mod formula;
mod graph;
mod process_command;
mod reevaluate_topo;
mod save_load;
mod sheet_extra_impl;
mod spreadsheet;
mod vim_mode;
mod visualize_cells;
use crate::process_command::process_command;
use std::env;
use std::io::{self, Write};
use std::process;

use std::time::Instant;

use crate::save_load::{load_spreadsheet, save_spreadsheet};
use spreadsheet::CommandStatus;
use spreadsheet::Spreadsheet;
const DEFAULT_FILENAME: &str = "rust_spreadsheet.sheet";

/// Entry point for the spreadsheet application.
///
/// Parses command-line arguments and runs in interactive or Vim mode.
///
/// # Arguments
///
/// * Expects `rows` and `cols` as arguments, optionally preceded by `--vim`
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
                    .truncate(true)
                    .open(DEFAULT_FILENAME);
            }
            Some(DEFAULT_FILENAME.to_string())
        };
        vim_mode::run_editor(&mut sheet, filename);
    } else {
        let mut last_time = start.elapsed().as_secs_f64(); // Update last_time with the command time
        let mut last_status = "ok"; // Placeholder for last status
        let mut input = String::with_capacity(128);
        let mut status;
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
            if let Some(filename_part) = trimmed.strip_prefix("open ") {
                let filename = filename_part;
                if !filename.is_empty() {
                    println!("Loading spreadsheet from '{}'...", filename);
                    match load_spreadsheet(&mut sheet, filename) {
                        CommandStatus::CmdOk => {
                            println!("Spreadsheet successfully loaded from '{}'", filename);
                            last_status = "ok";
                        }
                        _ => {
                            eprintln!("Failed to load spreadsheet from '{}'", filename);
                            last_status = "error";
                        }
                    }
                    continue;
                }
            }

            // Process the command and measure execution time
            status = process_command(&mut sheet, trimmed, &mut last_time);

            // Update last_status based on the current command status
            last_status = match status {
                CommandStatus::CmdOk => "ok",
                CommandStatus::Unrecognized => "unrecognized_cmd",
                CommandStatus::CircularRef => "circular_ref",
                CommandStatus::InvalidCell => "invalid_cell",
                CommandStatus::LockedCell => "locked_cell",
                CommandStatus::NotLockedCell => "not_locked_cell",
            };
        }
    }
}