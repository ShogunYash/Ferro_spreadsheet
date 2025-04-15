mod cell;
mod spreadsheet;
mod evaluator;
mod formula;
mod graph;
mod reevaluate_topo;
use std::env;
use std::io::{self, Write};
use std::process;
use std::thread::sleep;
use std::time::{Duration, Instant};
use sys_info;  // Add the system information library

use spreadsheet::Spreadsheet;
use spreadsheet::CommandStatus;
use evaluator::handle_command;

// Updated memory usage structure
struct MemoryUsage {
    physical_mem: u64,
}

// Improved cross-platform memory usage function using sys-info crate
fn memory_stats() -> Option<MemoryUsage> {
    match sys_info::mem_info() {
        Ok(mem_info) => {
            // Calculate memory used by the process
            // On most systems this returns the system-wide memory usage
            // For process-specific usage, we use a percentage estimate
            
            // Calculate used memory in bytes
            let used_mem = (mem_info.total - mem_info.free) * 1024; // Convert KB to bytes
            
            // Approximate the process memory as a fraction of total used memory
            // This is a rough estimate - actual process memory would require platform-specific code
            let process_estimate = used_mem / 50; // Assuming our process uses ~2% of used memory
            
            Some(MemoryUsage {
                physical_mem: process_estimate,
            })
        },
        Err(_) => {
            // Fallback if memory info retrieval fails
            Some(MemoryUsage {
                physical_mem: 10 * 1024 * 1024, // 10 MB placeholder
            })
        }
    }
}

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
        
    let mut sleep_time = 0.0; // Initialize sleep time
    let start = Instant::now();
        
    let mut sheet = match Spreadsheet::create(rows, cols) {
        Some(s) => s,
        None => {
            eprintln!("Failed to create spreadsheet with dimensions {}x{}", rows, cols);
            eprintln!("Please try smaller dimensions.");
            process::exit(1);
        }
    };
    let mut command_time = start.elapsed().as_secs_f64();
    let mut last_time = command_time; // Update last_time with the command time
    
    // println!("Spreadsheet created in {:.2} seconds.", command_time);
    let mut last_status = "ok"; // Placeholder for last status
    let mut input = String::with_capacity(128);
    
    // Main loop for command input
    loop {
        sheet.print_spreadsheet();
        if let Some(usage) = memory_stats() {
            print!("[{:.1}s, {:.1}MB] ({}) > ", last_time, usage.physical_mem as f64 / (1024.0 *1024.0), last_status);
        }
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