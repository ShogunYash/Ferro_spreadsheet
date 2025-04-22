mod cell;
mod evaluator;
mod formula;
mod graph;
mod reevaluate_topo;
mod spreadsheet;
mod vim_mode;
mod visualize_cells;
use std::env;
use std::io::{self, Write};
use std::process;
use std::thread::sleep;
use std::time::{Duration, Instant};
// use sys_info;  // Add the system information library

use evaluator::handle_command;
use spreadsheet::CommandStatus;
use spreadsheet::Spreadsheet;
const DEFAULT_FILENAME: &str = "untitled.sheet";

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
        let filename = Some(DEFAULT_FILENAME.to_string());
        vim_mode::run_editor(&mut sheet, filename);
    } else {
        let mut command_time = start.elapsed().as_secs_f64();
        let mut last_time = command_time; // Update last_time with the command time

        let mut last_status = "ok"; // Placeholder for last status
        let mut input = String::with_capacity(128);

        // Main loop for command input
        loop {
            sheet.print_spreadsheet();
            print!("[{:.1}] ({}) > ", last_time, last_status);
            io::stdout().flush().unwrap(); // Ensure the prompt is shown

            input.clear();
            if io::stdin().read_line(&mut input).unwrap() == 0 {
                break; // End of input
            }

            let trimmed = input.trim(); // Remove any newline characters
            if trimmed == "q" {
                break;
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
            };
        }
    }
}
