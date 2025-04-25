use crate::evaluator::handle_command;
use crate::spreadsheet::{CommandStatus, Spreadsheet};
use std::thread::sleep;
use std::time::{Duration, Instant};

/// Processes a command, measuring execution time and handling sleep.
///
/// # Arguments
///
/// * `sheet` - The mutable spreadsheet.
/// * `command` - The command string.
/// * `last_time` - Stores the total execution time (including sleep).
///
/// # Returns
///
/// The status of command execution
pub fn process_command(
    sheet: &mut Spreadsheet,
    command: &str,
    last_time: &mut f64,
) -> CommandStatus {
    // Process the command and measure execution time
    let mut sleep_time = 0.0; // Initialize sleep_time to 0.0
    // Pass by reference instead of cloning
    let start = Instant::now();
    let status = handle_command(sheet, command, &mut sleep_time);
    let command_time = start.elapsed().as_secs_f64();

    if sleep_time <= command_time {
        sleep_time = 0.0;
    } else {
        sleep_time -= command_time;
    }
    *last_time = command_time + sleep_time;
    if sleep_time > 0.0 {
        sleep(Duration::from_secs_f64(sleep_time));
    }
    status
}
