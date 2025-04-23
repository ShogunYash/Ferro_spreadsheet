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
mod process_command;
use crate::process_command::process_command;
use std::env;
use std::io::{self, Write};
use std::process;

use std::time::Instant;

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
            status = process_command(&mut sheet, trimmed, &mut last_time);
            // let start = Instant::now();
            // // Pass by reference instead of cloning
            // let status = handle_command(&mut sheet, trimmed, &mut sleep_time);
            // command_time = start.elapsed().as_secs_f64();

            // if sleep_time <= command_time {
            //     sleep_time = 0.0;
            // } else {
            //     sleep_time -= command_time;
            // }
            // last_time = command_time + sleep_time;
            // if sleep_time > 0.0 {
            //     sleep(Duration::from_secs_f64(sleep_time));
            // }
            // sleep_time = 0.0;

            // Update last_status based on the current command status
            last_status = match status {
                CommandStatus::CmdOk => "ok",
                CommandStatus::CmdUnrecognized => "unrecognized_cmd",
                CommandStatus::CmdCircularRef => "circular_ref",
                CommandStatus::CmdInvalidCell => "invalid_cell",
                CommandStatus::CmdLockedCell => "locked_cell",
                CommandStatus::CmdNotLockedCell => "not_locked_cell",
            };
        }
    }
}


// #[cfg(test)]
// mod main_tests {
//     use crate::process_command::process_command;
//     use crate::spreadsheet::{CommandStatus, Spreadsheet};
//     use std::io::Cursor;
    
//     use std::fs::{self};
//     use std::path::Path;
//     use tempfile::tempdir;

    // Test the process_command function directly
    // #[test]
    // fn test_process_command() {
    //     let mut sheet = Spreadsheet::create(10, 10).unwrap();
    //     let mut last_time = 0.0;
        
    //     // Test valid commands

      
        
    //     // Test invalid commands
    //     assert_eq!(process_command(&mut sheet, "invalid_command", &mut last_time), CommandStatus::CmdUnrecognized);
        
    //     // Test circular reference
    //     process_command(&mut sheet, "A1=10", &mut last_time);
    //     process_command(&mut sheet, "B1=A1+5", &mut last_time);
    //     assert_eq!(process_command(&mut sheet, "A1=B1+2", &mut last_time), CommandStatus::CmdCircularRef);
        
    //     // Test invalid cell reference
    //     assert_eq!(process_command(&mut sheet, "Z99=42", &mut last_time), CommandStatus::CmdInvalidCell);
        
    //     // Test lock/unlock functionality
    //     process_command(&mut sheet, "lock A2", &mut last_time);
    //     assert_eq!(process_command(&mut sheet, "A2=42", &mut last_time), CommandStatus::CmdLockedCell);
    //     assert_eq!(process_command(&mut sheet, "unlock B2", &mut last_time), CommandStatus::CmdNotLockedCell);
    // }

    // Test the save functionality
//     #[test]
//     fn test_save_spreadsheet() {
//         let temp_dir = tempdir().unwrap();
//         let file_path = temp_dir.path().join("test_save.sheet");
//         let file_path_str = file_path.to_str().unwrap();
        
//         let mut sheet = Spreadsheet::create(5, 5).unwrap();
        
//         // Set up some data in the spreadsheet
//         process_command(&mut sheet, "set A1 = 10", &mut 0.0);
//         process_command(&mut sheet, "set B1 = A1 * 2", &mut 0.0);
        
//         // Save the spreadsheet using the save_load module
//         crate::save_load::save_spreadsheet(&sheet, file_path_str);
        
//         // Check if the file exists
//         assert!(Path::new(file_path_str).exists());
        
//         // Basic check that the file has content
//         let metadata = fs::metadata(file_path_str).unwrap();
//         assert!(metadata.len() > 0);
//     }
    
//     // Test the load functionality
//     #[test]
//     fn test_load_spreadsheet() {
//         let temp_dir = tempdir().unwrap();
//         let file_path = temp_dir.path().join("test_load.sheet");
//         let file_path_str = file_path.to_str().unwrap();
        
//         // Create and save a spreadsheet
//         {
//             let mut sheet = Spreadsheet::create(5, 5).unwrap();
//             process_command(&mut sheet, "set A1 = 10", &mut 0.0);
//             process_command(&mut sheet, "set B1 = A1 * 2", &mut 0.0);
//             crate::save_load::save_spreadsheet(&sheet, file_path_str);
//         }
        
//         // Load the spreadsheet
//         let mut sheet = Spreadsheet::create(5, 5).unwrap();
//         let status = crate::save_load::load_spreadsheet(&mut sheet, file_path_str);
        
//         // Check that the load was successful
//         assert_eq!(status, CommandStatus::CmdOk);
        
//         // Verify the data was loaded correctly
//         // We'd need to expose methods to check cell values or use get command
//         // For now, we just verify the command completed successfully
//     }
    
//     // Mock stdin/stdout for main input handling
//     #[test]
//     fn test_main_input_handling() {
//         use std::io::BufRead;
        
//         let input = b"q\nn\n";
//         let mut cursor = Cursor::new(input);
        
        
//         // Mock stdin read
//         let mut line = String::new();
        
//         cursor.read_line(&mut line).unwrap();
//         assert_eq!(line.trim(), "q");
        
       
//     }
    
//     // Test for the vim mode flag detection
//     #[test]
//     fn test_vim_mode_flag_detection() {
//         let args = vec![
//             String::from("program_name"),
//             String::from("--vim"),
//             String::from("10"),
//             String::from("10")
//         ];
        
//         // In a real test, we'd need to mock std::env::args()
//         // This is a simplified version to test the logic
//         let vim_mode_enabled = args.len() > 1 && args[1] == "--vim";
//         let rows_arg_index = if vim_mode_enabled { 2 } else { 1 };
//         let cols_arg_index = if vim_mode_enabled { 3 } else { 2 };
        
//         assert!(vim_mode_enabled);
//         assert_eq!(rows_arg_index, 2);
//         assert_eq!(cols_arg_index, 3);
        
//         let rows: i16 = args[rows_arg_index].parse().unwrap();
//         let cols: i16 = args[cols_arg_index].parse().unwrap();
        
//         assert_eq!(rows, 10);
//         assert_eq!(cols, 10);
//     }
    
//     // Test for the open command
//     #[test]
//     fn test_open_command() {
//         let temp_dir = tempdir().unwrap();
//         let file_path = temp_dir.path().join("test_open.sheet");
//         let file_path_str = file_path.to_str().unwrap();
        
//         // Create and save a spreadsheet
//         {
//             let mut sheet = Spreadsheet::create(5, 5).unwrap();
//             process_command(&mut sheet, "set A1 = 10", &mut 0.0);
//             crate::save_load::save_spreadsheet(&sheet, file_path_str);
//         }
        
//         // Test the open command by simulating its processing
//         let mut sheet = Spreadsheet::create(5, 5).unwrap();
//         let command = format!("open {}", file_path_str);
        
//         // Extract the filename portion
//         let parts: Vec<&str> = command.split_whitespace().collect();
//         assert!(parts.len() > 1);
        
//         let filename = parts[1];
//         assert_eq!(filename, file_path_str);
        
//         // Verify that the file exists
//         assert!(Path::new(filename).exists());
        
//         // Now test the actual loading
//         let status = crate::save_load::load_spreadsheet(&mut sheet, filename);
//         assert_eq!(status, CommandStatus::CmdOk);
//     }
    
//     // Test the quit with save functionality
//     #[test]
//     fn test_quit_with_save() {
//         let temp_dir = tempdir().unwrap();
//         let file_path = temp_dir.path().join("test_quit_save.sheet");
//         let file_path_str = file_path.to_str().unwrap();
        
//         // Set up spreadsheet with some data
//         let mut sheet = Spreadsheet::create(5, 5).unwrap();
//         process_command(&mut sheet, "set A1 = 10", &mut 0.0);
        
//         // Mock user input sequence for quit with save
//         let input = format!("q\ny\n{}\n", file_path_str);
//         let input_bytes = input.as_bytes();
        
//         // In a real test, we'd need to mock stdin/stdout
//         // Here we just verify the input parsing logic
//         let lines: Vec<&str> = input.lines().collect();
//         assert_eq!(lines[0], "q");
//         assert_eq!(lines[1], "y");
//         assert_eq!(lines[2], file_path_str);
        
//         // Simulate saving
//         crate::save_load::save_spreadsheet(&sheet, file_path_str);
        
//         // Verify file was created
//         assert!(Path::new(file_path_str).exists());
//     }
// }