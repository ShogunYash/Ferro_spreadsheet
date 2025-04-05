mod cell;
mod spreadsheet;

use std::env;
use std::io::{self, Write};
use std::process;
use std::time::Instant;

use spreadsheet::Spreadsheet;
use spreadsheet::CommandStatus;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <rows> <columns>", args[0]);
        process::exit(1);
    }
    
    let rows: i16 = args[1].parse().unwrap_or_else(|_| {
        eprintln!("Invalid number for rows");
        process::exit(1);
    });

    let cols: i16 = args[2].parse().unwrap_or_else(|_| {
        eprintln!("Invalid number for columns");
        process::exit(1);
    });
    
    println!("Creating spreadsheet with {} rows and {} columns...", rows, cols);
    println!("This may take a moment for large spreadsheets.");
    
    let mut last_time = 0.0; // Placeholder for last time
    let start = Instant::now();
    
    // For very large spreadsheet sizes, warn the user
    if (rows as i64) * (cols as i64) > 1_000_000 {
        println!("Warning: Creating a large spreadsheet with {} cells", (rows as i64) * (cols as i64));
        println!("This may consume significant memory.");
        print!("Do you want to continue? (y/n) ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        if input.trim().to_lowercase() != "y" {
            println!("Operation cancelled.");
            process::exit(0);
        }
    }
    
    let mut sheet = match Spreadsheet::create(rows, cols) {
        Some(s) => s,
        None => {
            eprintln!("Failed to create spreadsheet with dimensions {}x{}", rows, cols);
            eprintln!("Please try smaller dimensions.");
            process::exit(1);
        }
    };
    
    let command_time = start.elapsed().as_secs_f64();
    println!("Spreadsheet created in {:.2} seconds.", command_time);
    
    let mut last_status = "ok"; // Placeholder for last status
    let mut status = CommandStatus::CmdOk; // Placeholder for status
    let mut input = String::with_capacity(128);
    
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
    
        // Update last_status based on the current command status.
        last_status = match status {
            CommandStatus::CmdOk => "ok",
            CommandStatus::CmdUnrecognized => "unrecognized cmd",
            CommandStatus::CmdCircularRef => "circular ref",
        };
    }
}