use std::clone;

use crate::spreadsheet::{Spreadsheet, CommandStatus};
use crate::cell::{Cell, CellValue, parse_cell_reference};
use crate::formula::parse_range;
use crate::formula::Range;
use crate::formula::{eval_max, eval_min, sum_value, eval_variance};

pub fn get_key(row: i16, col: i16, cols: i16) -> i32 {
    ((row as i32 )* (cols as i32) + (col as i32)) as i32
}

pub fn handle_sleep(
    sheet: &mut Spreadsheet,
    row: i16,
    col: i16,
    expr: &str,
    sleep_time: &mut f64,
) -> CommandStatus {
    // Get the cols value early to avoid borrowing issues
    let cols = sheet.cols;
    let pkey = get_key(row, col, cols) ; // Cast to i16 to match cell.parent fields
    
    // Handle cell reference case
    if let Ok((target_row, target_col)) = parse_cell_reference(sheet, expr) {
        // Check if the referenced cell exists and get its value
        if let Some(ref_cell) = sheet.get_cell(target_row, target_col) {
            // Store the value before mutable borrow
            let value = ref_cell.value.clone();
            
            // Now get our target cell for mutation
            let cell = sheet.get_mut_cell(row, col);
            cell.formula = 102;    // Custom formula code for sleep
            cell.parent1 = pkey;   // Store the current cell key
            cell.parent2 = -1;     // No second parent for sleep
            cell.value = value.clone();    // Set the value we stored earlier
            
            // Add to sleep time if integer
            if let CellValue::Integer(val) = value {
                *sleep_time += val as f64;
            } else {
                cell.value = CellValue::Error;
            }
        } else {
            // Referenced cell doesn't exist
            let cell = sheet.get_mut_cell(row, col);
            cell.value = CellValue::Error;
            return CommandStatus::CmdInvalidCell;
        }
    } 
    // Handle numeric literal case
    else if let Ok(val) = expr.parse::<i32>() {
        let cell = sheet.get_mut_cell(row, col);
        cell.value = CellValue::Integer(val);
        *sleep_time += val as f64;
        cell.formula = -1;
        cell.parent1 = -1;
        cell.parent2 = -1;
    }
    else {
        return CommandStatus::CmdUnrecognized;
    }

    CommandStatus::CmdOk
}
pub fn evaluate_formula(
    sheet: &mut Spreadsheet,
    row: i16,
    col: i16,
    expr: &str,
    sleep_time: &mut f64,
) -> CommandStatus {
    let expr_len = expr.len();
    if expr_len == 0 {
        return CommandStatus::CmdUnrecognized;
    }
    let cols = sheet.cols;

    let is_avg = expr.starts_with("AVG(");
    let is_min = expr.starts_with("MIN(");
    let is_max = expr.starts_with("MAX(");
    let is_stdev = expr.starts_with("STDEV(");
    let is_sum = expr.starts_with("SUM(");

    // Range-based functions: SUM, AVG, MIN, MAX, STDEV
    if is_avg || is_min || is_max || is_stdev || is_sum {
        let prefix_len = if is_stdev { 6 } else { 4 };

        if !expr.ends_with(')') {
            return CommandStatus::CmdUnrecognized;
        }

        // Extract the range string without allocating extra memory.
        let range_str = &expr[prefix_len..expr_len - 1];
        let range = match parse_range(sheet,range_str) {
            Ok(r) => r,
            Err(status) => return status,
        };

        let cell = sheet.get_mut_cell(row, col);
        // Set the formula code based on the function type.
        cell.formula = if is_sum {
            5
        } else if is_avg {
            6
        } else if is_min {
            7
        } else if is_max {
            8
        } else {
            9 // is_stdev case
        };

        cell.parent1 = get_key(range.start_row, range.start_col, cols);
        cell.parent2 = get_key(range.end_row, range.end_col, cols);

        // Evaluate the function.

        if is_stdev {
            return eval_variance(sheet,row,col, &range);
        } else if is_max {
            return eval_max(sheet, row,col, &range);
        } else if is_min {
            return eval_min(sheet, row,col, &range);
        
        } else if is_avg {
            let status = sum_value(sheet, row,col, &range);
            if status != CommandStatus::CmdOk {
                return status;
            }
            
            let count  =( (range.end_row - range.start_row + 1) * (range.end_col - range.start_col + 1) )as i32;
            let cell = sheet.get_mut_cell(row, col);
            if let CellValue::Integer(sum) = cell.value {
                cell.value = CellValue::Integer(sum / count);
            } else {
                cell.value = CellValue::Error;
            }
            return CommandStatus::CmdOk;
        } else {
            return sum_value(sheet,row,col, &range);
        }
    } else if expr.starts_with("SLEEP(") {
        if !expr.ends_with(')') {
            return CommandStatus::CmdUnrecognized;
        }

        let sleep_str = &expr[6..expr_len - 1];
        return handle_sleep(sheet, row, col, sleep_str, sleep_time);
    }
    if let Ok(number) = expr.parse::<i32>() {
        let cell = sheet.get_mut_cell(row, col);
        cell.value = CellValue::Integer(number);
        cell.formula = -1;
        return CommandStatus::CmdOk;
    }
    //if the expr is fully alphanumeric parse the cell reference
    if expr.chars().all(|c| c.is_alphanumeric() || c == '_') {
        let cell_ref = expr;
        match parse_cell_reference(sheet, cell_ref) {
            Ok((target_row, target_col)) => {
                // Check if the referenced cell exists and get its value
                if let Some(ref_cell) = sheet.get_cell(target_row, target_col) {
                    // Store the value before mutable borrow
                    let value = ref_cell.value.clone();
                        
                    
                    // Now get our target cell for mutation
                    let cell = sheet.get_mut_cell(row, col);
                    cell.formula = 82;    // Custom formula code for reference
                    cell.parent1 = get_key(target_row, target_col, cols);   // Store the current cell key
                    cell.parent2 = -1;     // No second parent for reference
                    cell.value = value.clone();    // Set the value we stored earlier
                } else {
                    // Referenced cell doesn't exist
                    let cell = sheet.get_mut_cell(row, col);
                    cell.value = CellValue::Error;
                    return CommandStatus::CmdInvalidCell;
                }
            },
            Err(status) => {
                return status;
            }
        }
        return CommandStatus::CmdOk;
    }

    // Binary arithmetic expression handling
    // Find operators +, -, *, / with proper precedence
    // Need to account for nested expressions and operator precedence
    for op_idx in 2..expr.len() {
        // Skip first character to not confuse leading minus with subtraction
        
            let c = expr.chars().nth(op_idx).unwrap();
            
            if c == '+' || c == '-' {
                // Only treat as operator if not inside parentheses
                let left = &expr[..op_idx].trim();
                let right = &expr[op_idx+1..].trim();

                if !left.is_empty() && !right.is_empty() {
                    // Evaluate left and right expressions recursively
                    let left_status = parse_cell_reference(sheet, left);
                    if left_status.is_err() {
                        return left_status.err().unwrap();
                    }
                    // Save the left result and clear the cell for right evaluation
                    let (rowl, coll) = left_status.unwrap();
                    let left_cell = sheet.get_cell(rowl, coll).unwrap();
                    let left_value = if let CellValue::Integer(val) = left_cell.value {
                        val
                    } else {
                        let cell = sheet.get_mut_cell(row, col);
                        cell.value = CellValue::Error;
                        return CommandStatus::CmdOk;
                    };
                    
                    let right_status = parse_cell_reference(sheet, right);
                    if right_status.is_err() {
                        return right_status.err().unwrap();
                    }
                    let (rowr, colr) = right_status.unwrap();
                    // Get the right result and perform the operation
                    let right_cell = sheet.get_cell(rowr, colr).unwrap();
                    
                    let right_value = if let CellValue::Integer(val) = right_cell.value {
                        val
                    } else {
                        let cell = sheet.get_mut_cell(row, col);
                        cell.value = CellValue::Error;
                        return CommandStatus::CmdOk;
                    };
                    
                    // Perform the operation
                    let cell = sheet.get_mut_cell(row, col);
                    if c == '+' {
                        cell.value = CellValue::Integer(left_value + right_value);
                        cell.formula = 10;
                    } else if c == '-'{ 
                        cell.value = CellValue::Integer(left_value - right_value);
                        cell.formula = 20;
                    }
                    else if c == '*' {
                        cell.value = CellValue::Integer(left_value * right_value);
                        cell.formula = 40; // Code for multiplication
                    } else if c == '/' { 
                        if right_value == 0 {
                            cell.value = CellValue::Error; // Division by zero
                            return CommandStatus::CmdOk;
                        }
                        cell.value = CellValue::Integer(left_value / right_value);
                        cell.formula = 30; // Code for division
                    }
                    
                    
                     // Code for binary operation
                    return CommandStatus::CmdOk;
                }
            
        }
    }
    
    CommandStatus::CmdUnrecognized
}

pub fn set_cell_value(sheet: &mut Spreadsheet, row: i16, col: i16, expr: &str, sleep_time: &mut f64) -> CommandStatus {
        // let cell = sheet.get_mut_cell(row, col);
        let status = evaluate_formula(sheet, row, col, expr, sleep_time);
        
        status
}
pub fn handle_command(
    sheet: &mut Spreadsheet,
    input: String,
    sleep_time: &mut f64,
) -> CommandStatus {
    let trimmed = input.trim();
    
    // Handle special commands
    if trimmed == "disable_output" {
        sheet.output_enabled = false;
        return CommandStatus::CmdOk;
    } else if trimmed == "enable_output" {
        sheet.output_enabled = true;
        return CommandStatus::CmdOk;
    } else if trimmed.len() == 1 && "wasd".contains(trimmed.chars().next().unwrap()) {
        let direction = trimmed.chars().next().unwrap();
        sheet.scroll_viewport(direction);
        return CommandStatus::CmdOk;
    } else if trimmed.starts_with("scroll_to ") {
        let cell_ref = &trimmed[10..]; // Skip "scroll_to "
        return sheet.scroll_to_cell(cell_ref);
    }
    
    // Handle cell assignments (CELL=EXPRESSION)
    if let Some(eq_pos) = trimmed.find('=') {
        let (cell_ref, expr) = trimmed.split_at(eq_pos);
        let cell_ref = cell_ref.trim();
        let expr = &expr[1..].trim(); // Skip the '=' character
        
        // Parse the cell reference
        match parse_cell_reference(sheet, cell_ref) {
            Ok((row, col)) => {
                // Check if cell is within bounds
                if row < 0 || row >= sheet.rows || col < 0 || col >= sheet.cols {
                    return CommandStatus::CmdInvalidCell;
                }
                
                // Set the cell value
                
                return set_cell_value(sheet, row, col, expr, sleep_time);
            },
            Err(status) => {
                return status;
            }
        }
    }
    
    CommandStatus::CmdUnrecognized
}