use std::clone;
use crate::spreadsheet::{Spreadsheet, CommandStatus};
use crate::cell::{Cell, CellValue, parse_cell_reference};
use crate::formula::{parse_range, Range};
use crate::formula::{eval_max, eval_min, sum_value, eval_variance};
use crate::graph::{add_children, remove_all_parents, detect_cycle};


pub fn get_key(row: i16, col: i16, cols: i16) -> i32 {
    ((row as i32 )* (cols as i32) + (col as i32)) as i32
}

pub fn get_cell_from_key (spreadsheet:&mut Spreadsheet, key:i32) -> &mut Cell {
    return &mut spreadsheet.grid[key as usize];
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
    let cell_key = get_key(row, col, cols);
    
    // Handle cell reference case
    if let Ok((target_row, target_col)) = parse_cell_reference(sheet, expr) {
        // Get the value from parent cell first to avoid borrowing issues
        let parent_value;
        let pkey = get_key(target_row, target_col, cols);
        {
            let parent_cell = sheet.get_cell(target_row, target_col);
            parent_value = parent_cell.value.clone();
        }
        
        // Variables to store old values
        let old_parent1;
        let old_parent2;
        let old_formula;
        let old_value;
        // Remove parents and set up new formula
        {
            remove_all_parents(sheet, row, col);
            let cell = sheet.get_mut_cell(row, col);
            
            // Store old values for possible restoration
            old_parent1 = cell.parent1;
            old_parent2 = cell.parent2;
            old_formula = cell.formula;
            old_value = cell.value.clone();
            // Set the formula code for sleep
            cell.parent1 = pkey;
            cell.parent2 = -1;
            cell.formula = 102;    // Custom formula code for sleep
            cell.value = parent_value.clone();
        }
        
        // Check for circular reference
        if detect_cycle(sheet, pkey, -1, 102, cell_key) {
            let cell: &mut Cell = sheet.get_mut_cell(row, col);
            cell.parent1 = old_parent1;
            cell.parent2 = old_parent2;
            cell.formula = old_formula;
            cell.value = old_value;
            add_children(sheet, old_parent1, old_parent2, old_formula, row, col);
            return CommandStatus::CmdCircularRef;
        }
        
        // Add children and update sleep time
        add_children(sheet, pkey, -1, 102, row, col);
        
        // Add to sleep time if integer
        if let CellValue::Integer(val) = parent_value {
            *sleep_time += val as f64;
        }
    } 
    // Handle numeric literal case
    else if let Ok(val) = expr.parse::<i32>() {
        // Remove all the parents
        remove_all_parents(sheet, row, col);
        let cell = sheet.get_mut_cell(row, col);
        cell.value = CellValue::Integer(val);
        cell.formula = -1;
        cell.parent1 = -1;
        cell.parent2 = -1;
        *sleep_time += val as f64;
    }
    else {
        return CommandStatus::CmdUnrecognized;
    }
    CommandStatus::CmdOk
}

pub fn evaluate_arithmetic(
    sheet: &mut Spreadsheet,
    row: i16,
    col: i16,
    expr: &str,
) -> CommandStatus {
    let cols = sheet.cols;
    let cell_key = get_key(row, col, cols);
    if let Ok(number) = expr.parse::<i32>() {
        // Remove all the parents
        remove_all_parents(sheet, row, col);
        let cell = sheet.get_mut_cell(row, col);
        cell.value = CellValue::Integer(number);
        cell.formula = -1;
        cell.parent1 = -1;
        cell.parent2 = -1;
        return CommandStatus::CmdOk;
    }
    //if the expr is fully alphanumeric parse the cell reference
    if expr.chars().all(|c| c.is_alphanumeric() || c == '_') {
        match parse_cell_reference(sheet, expr) {
            Ok((target_row, target_col)) => {
                // Get value from parent cell first to avoid borrowing issues
                let parent_value;
                {
                    let parent_cell = sheet.get_cell(target_row, target_col);
                    parent_value = parent_cell.value.clone();
                }
                
                // Prepare variables to store old values 
                let old_parent1;
                let old_parent2;
                let old_formula;
                let parent1;
                let old_value;
                
                // Remove parents and set up new formula
                {
                    remove_all_parents(sheet, row, col);
                    let cell = sheet.get_mut_cell(row, col);
                    
                    // Store old values
                    old_parent1 = cell.parent1;
                    old_parent2 = cell.parent2;
                    old_formula = cell.formula;
                    old_value = cell.value.clone();
                    // Set up new reference
                    parent1 = get_key(target_row, target_col, cols);
                    cell.parent1 = parent1;
                    cell.parent2 = -1;
                    cell.formula = 82;    // Custom formula code for reference
                    cell.value = parent_value;
                }
                
                // Check for circular reference
                if detect_cycle(sheet, parent1, -1, 82, cell_key) {
                    let cell = sheet.get_mut_cell(row, col);
                    cell.parent1 = old_parent1;
                    cell.parent2 = old_parent2;
                    cell.formula = old_formula;
                    cell.value = old_value;
                    add_children(sheet, old_parent1, old_parent2, old_formula, row, col);
                    return CommandStatus::CmdCircularRef;
                }
                
                // Add children after cycle check passes
                add_children(sheet, parent1, -1, 82, row, col);
            },
            Err(status) => {
                return status;
            }
        }
        return CommandStatus::CmdOk;
    }

    // Binary arithmetic expression handling
    for op_idx in 2..expr.len() {
        let c = expr.chars().nth(op_idx).unwrap();
        
        if c == '+' || c == '-' || c == '*' || c == '/' {
            // Split the expression into left and right parts
            let left = &expr[..op_idx].trim();
            let right = &expr[op_idx+1..].trim();

            if !left.is_empty() && !right.is_empty() {
                let left_status = parse_cell_reference(sheet, left);
                if left_status.is_err() {
                    return left_status.err().unwrap();
                }
                let (rowl, coll) = left_status.unwrap();
                let left_cell = sheet.get_cell(rowl, coll);
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
                let right_cell = sheet.get_cell(rowr, colr);
                
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
    // If we reach here, the expression is unrecognized
    CommandStatus::CmdUnrecognized
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

        let (old_parent1, old_parent2, old_formula, parent1, parent2, formula);
        {
            remove_all_parents(sheet, row, col); 
            let cell = sheet.get_mut_cell(row, col);
            //storing the old parents and formula in case of circular ref
            old_parent1 = cell.parent1;
            old_parent2 = cell.parent2;
            old_formula = cell.formula;
            // Set the parent keys based on the range.
            cell.parent1 = get_key(range.start_row, range.start_col, cols);
            cell.parent2 = get_key(range.end_row, range.end_col, cols);
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

            parent1 = cell.parent1;
            parent2 = cell.parent2;
            formula = cell.formula;
        }
        // Evaluate the function.
        if detect_cycle(sheet, parent1, parent2, formula, get_key(row, col, cols)) {
            // If a cycle is detected, restore the old parents and formula
            let cell = sheet.get_mut_cell(row, col);
            cell.parent1 = old_parent1;
            cell.parent2 = old_parent2;
            cell.formula = old_formula;
            add_children(sheet, old_parent1, old_parent2, old_formula, row, col);
            return CommandStatus::CmdCircularRef;
        }

        add_children(sheet, parent1, parent2, formula, row, col);
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
            
            let count  =( ((range.end_row - range.start_row + 1) as i32) * ((range.end_col - range.start_col + 1) as i32) )as i32;
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
    else{
        return evaluate_arithmetic(sheet, row, col, expr);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_command() {
        let mut sheet = Spreadsheet::create(5, 5).unwrap();
        let mut sleep_time = 0.0;
        assert_eq!(handle_command(&mut sheet, "A1=42".to_string(), &mut sleep_time), CommandStatus::CmdOk);
        assert_eq!(sheet.get_cell(0, 0).value, CellValue::Integer(42));
        assert_eq!(handle_command(&mut sheet, "disable_output".to_string(), &mut sleep_time), CommandStatus::CmdOk);
        assert_eq!(handle_command(&mut sheet, "w".to_string(), &mut sleep_time), CommandStatus::CmdOk);
        assert_eq!(handle_command(&mut sheet, "scroll_to B2".to_string(), &mut sleep_time), CommandStatus::CmdOk);
    }

    #[test]
    fn test_evaluate_arithmetic() {
        let mut sheet = Spreadsheet::create(5, 5).unwrap();
        assert_eq!(evaluate_arithmetic(&mut sheet, 0, 0, "42"), CommandStatus::CmdOk);
        assert_eq!(sheet.get_cell(0, 0).value, CellValue::Integer(42));
        assert_eq!(evaluate_arithmetic(&mut sheet, 0, 1, "A1"), CommandStatus::CmdOk);
        assert_eq!(sheet.get_cell(0, 1).value, CellValue::Integer(42));
        assert_eq!(evaluate_arithmetic(&mut sheet, 1, 0, "A1 + B1"), CommandStatus::CmdOk);
        assert_eq!(sheet.get_cell(1, 0).value, CellValue::Integer(84));
        assert_eq!(evaluate_arithmetic(&mut sheet, 1, 1, "A1 / B2"), CommandStatus::CmdOk); // B2 is 0
        assert_eq!(sheet.get_cell(1, 1).value, CellValue::Error);
    }

    #[test]
    fn test_handle_sleep() {
        let mut sheet = Spreadsheet::create(5, 5).unwrap();
        let mut sleep_time = 0.0;
        assert_eq!(handle_sleep(&mut sheet, 0, 0, "2", &mut sleep_time), CommandStatus::CmdOk);
        assert_eq!(sleep_time, 2.0);
        assert_eq!(handle_sleep(&mut sheet, 0, 1, "A1", &mut sleep_time), CommandStatus::CmdOk);
    }
}