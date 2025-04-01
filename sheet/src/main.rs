mod cell;
mod formula;
mod spreadsheet;
mod ui;

use std::env;
use std::io::{self, BufRead, Write};
use std::process;
use std::time::Instant;

use spreadsheet::Spreadsheet;

fn main() {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    
    if args.len() != 3 {
        eprintln!("Usage: {} <rows> <columns>", args[0]);
        process::exit(1);
    }
    
    // Parse rows and columns
    let rows = match args[1].parse::<usize>() {
        Ok(r) => {
            if r < 1 || r > 999 {
                eprintln!("Error: Rows must be between 1 and 999");
                process::exit(1);
            }
            r
        }
        Err(_) => {
            eprintln!("Error: Invalid number of rows");
            process::exit(1);
        }
    };
    
    let cols = match args[2].parse::<usize>() {
        Ok(c) => {
            if c < 1 || c > 18278 {  // 26*26*26+26*26+26
                eprintln!("Error: Columns must be between 1 and 18,278");
                process::exit(1);
            }
            c
        }
        Err(_) => {
            eprintln!("Error: Invalid number of columns");
            process::exit(1);
        }
    };
    
    // Create a new spreadsheet
    let mut sheet = Spreadsheet::new(rows, cols);
    let mut output_enabled = true;
    
    // Display the initial spreadsheet
    if output_enabled {
        ui::display_sheet(&sheet);
    }
    
    // Process user commands
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    
    // Main loop
    let mut last_command_time = 0.0;
    let mut last_status = "(ok)".to_string();
    
    loop {
        // Display prompt
        print!("[{:.1}] {} > ", last_command_time, last_status);
        stdout.flush().unwrap();
        
        // Read user input
        let mut input = String::new();
        if stdin.lock().read_line(&mut input).is_err() {
            continue;
        }
        
        let input = input.trim();
        
        // Exit condition
        if input == "q" {
            break;
        }
        
        // Track command execution time
        let start_time = Instant::now();
        
        // Process command
        match input {
            "w" => {
                sheet.scroll_up();
                last_status = "(ok)".to_string();
            },
            "a" => {
                sheet.scroll_left();
                last_status = "(ok)".to_string();
            },
            "s" => {
                sheet.scroll_down();
                last_status = "(ok)".to_string();
            },
            "d" => {
                sheet.scroll_right();
                last_status = "(ok)".to_string();
            },
            "disable_output" => {
                output_enabled = false;
                last_status = "(ok)".to_string();
            },
            "enable_output" => {
                output_enabled = true;
                last_status = "(ok)".to_string();
            },
            _ if input.starts_with("scroll_to ") => {
                let cell_ref = input.trim_start_matches("scroll_to ").trim();
                match sheet.scroll_to(cell_ref) {
                    Ok(_) => last_status = "(ok)".to_string(),
                    Err(e) => last_status = format!("({})", e),
                }
            },
            _ => {
                // Try to parse and evaluate a formula
                match sheet.process_command(input) {
                    Ok(_) => last_status = "(ok)".to_string(),
                    Err(e) => last_status = format!("({})", e),
                }
            }
        }
        
        // Calculate execution time
        let elapsed = start_time.elapsed();
        last_command_time = elapsed.as_secs_f64();
        
        // Display updated spreadsheet
        if output_enabled {
            ui::display_sheet(&sheet);
        }
    }
}