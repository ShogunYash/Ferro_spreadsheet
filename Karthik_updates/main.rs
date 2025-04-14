mod cell;
mod spreadsheet;
mod evaluator;
mod formula;
mod linked_list;
mod graph;
use std::env;
use std::io::{self, Write};
use std::process;
use std::thread::sleep;
use std::time::{Duration, Instant};

use spreadsheet::Spreadsheet;
use spreadsheet::CommandStatus;
use evaluator::handle_command;

// Add memory usage functionality
struct MemoryUsage {
    physical_mem: u64,
}

fn memory_stats() -> Option<MemoryUsage> {
    // Simple implementation that reads from /proc/self/statm on Linux
    // or returns a dummy value on other platforms
    #[cfg(target_os = "linux")]
    {
        use std::fs::File;
        use std::io::Read;
        
        let mut buffer = String::new();
        if let Ok(mut file) = File::open("/proc/self/statm") {
            if file.read_to_string(&mut buffer).is_ok() {
                let values: Vec<&str> = buffer.split_whitespace().collect();
                if values.len() >= 2 {
                    if let Ok(vm_pages) = values[0].parse::<u64>() {
                        // Convert from pages to bytes (assume 4KB page size)
                        let page_size = 4096;
                        return Some(MemoryUsage {
                            physical_mem: vm_pages * page_size,
                        });
                    }
                }
            }
        }
        None
    }
    
    // For non-Linux platforms, provide a dummy implementation
    #[cfg(not(target_os = "linux"))]
    {
        // Return a placeholder value for platforms that don't support /proc
        Some(MemoryUsage {
            physical_mem: 10 * 1024 * 1024, // 10 MB placeholder
        })
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
    
    println!("Creating spreadsheet with {} rows and {} columns...", rows, cols);
    println!("This may take a moment for large spreadsheets.");
    
    let mut sleep_time = 0.0; // Initialize sleep time
    let mut last_time = 0.0; // Initialize last time
    let start = Instant::now();
    
    // For very large spreadsheet sizes, warn the user
    // if (rows as i64) * (cols as i64) > 1_000_000 {
    //     println!("Warning: Creating a large spreadsheet with {} cells", (rows as i64) * (cols as i64));
    //     println!("This may consume significant memory.");
    //     print!("Do you want to continue? (y/n) ");
    //     io::stdout().flush().unwrap();
        
    //     let mut input = String::new();
    //     io::stdin().read_line(&mut input).unwrap();
    //     if input.trim().to_lowercase() != "y" {
    //         println!("Operation cancelled.");
    //         process::exit(0);
    //     }
    // }
    
    let mut sheet = match Spreadsheet::create(rows, cols) {
        Some(s) => s,
        None => {
            eprintln!("Failed to create spreadsheet with dimensions {}x{}", rows, cols);
            eprintln!("Please try smaller dimensions.");
            process::exit(1);
        }
    };
    let mut command_time = start.elapsed().as_secs_f64();
    last_time = command_time; // Update last_time with the command time
    
    // println!("Spreadsheet created in {:.2} seconds.", command_time);
    
    let mut last_status = "ok"; // Placeholder for last status
    let mut status = CommandStatus::CmdOk; // Placeholder for status
    let mut input = String::with_capacity(128);
    
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
        status = handle_command(&mut sheet, input.clone(), &mut sleep_time);
        command_time= start.elapsed().as_secs_f64();

        if sleep_time <= command_time {
            sleep_time=0.0;
        }
        else {
            sleep_time -= command_time;
        }
        last_time = command_time + sleep_time;
        if sleep_time > 0.0 {
            sleep(Duration::from_secs_f64(sleep_time));
        }
        sleep_time= 0.0;
    
        // Update last_status based on the current command status
        last_status = match status {
            CommandStatus::CmdOk => "ok",
            CommandStatus::CmdUnrecognized => "unrecognized_cmd",
            CommandStatus::CmdCircularRef => "circular_ref",
            CommandStatus::CmdInvalidCell => "invalid_cell",
        };
    }
}