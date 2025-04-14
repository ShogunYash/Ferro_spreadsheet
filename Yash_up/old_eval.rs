use std::clone;
use crate::spreadsheet::{Spreadsheet, CommandStatus, CellMeta};
use crate::cell::{CellValue, parse_cell_reference};
use crate::formula::{parse_range, Range};
use crate::formula::{eval_max, eval_min, sum_value, eval_variance};
use crate::graph::{add_children, remove_all_parents, detect_cycle};

pub fn handle_sleep(
    sheet: &mut Spreadsheet,
    row: i16,
    col: i16,
    expr: &str,
    sleep_time: &mut f64,
) -> CommandStatus {
    let cell_key = sheet.get_key(row, col);
    
    // Handle cell reference case
    if let Ok((target_row, target_col)) = parse_cell_reference(sheet, expr) {
        // Get the value from parent cell first to avoid borrowing issues
        let parent_value;
        let pkey = sheet.get_key(target_row, target_col);
        {
            let parent_cell = sheet.get_cell(target_row, target_col);
            parent_value = parent_cell.value.clone();
        }
        
        // Store old metadata for possible restoration
        let old_meta = if let Some(meta) = sheet.cell_meta.get(&cell_key) {
            Some(meta.clone())
        } else {
            None
        };
        
        let old_value;
        {
            // Remove parents and set up new formula
            remove_all_parents(sheet, row, col);
            
            let cell = sheet.get_mut_cell(row, col);
            old_value = cell.value.clone();
            cell.value = parent_value.clone();
            
            // Set up the new cell metadata
            let meta = sheet.get_cell_meta(row, col);
            meta.parent1 = pkey;
            meta.parent2 = -1;
            meta.formula = 102;    // Custom formula code for sleep
        }
        
        // Check for circular reference
        if detect_cycle(sheet, pkey, -1, 102, cell_key) {
            // Restore old state if cycle detected
            let cell = sheet.get_mut_cell(row, col);
            cell.value = old_value;
            
            if let Some(old) = old_meta {
                sheet.cell_meta.insert(cell_key, old.clone());
                add_children(sheet, old.parent1, old.parent2, old.formula, row, col);
            } else {
                sheet.cell_meta.remove(&cell_key);
            }
            
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
        
        // Update cell value
        let cell = sheet.get_mut_cell(row, col);
        cell.value = CellValue::Integer(val);
        
        // Update metadata
        if let Some(meta) = sheet.cell_meta.get_mut(&cell_key) {
            meta.formula = -1;
            meta.parent1 = -1;
            meta.parent2 = -1;
        }
        
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
    let cell_key = sheet.get_key(row, col);
    
    if let Ok(number) = expr.parse::<i32>() {
        // Remove all the parents
        remove_all_parents(sheet, row, col);
        
        // Update cell value
        let cell = sheet.get_mut_cell(row, col);
        cell.value = CellValue::Integer(number);
        
        // Update metadata
        let meta = sheet.get_cell_meta(row, col);
        meta.formula = -1;
        meta.parent1 = -1;
        meta.parent2 = -1;
        
        return CommandStatus::CmdOk;
    }
    
    // If the expr is fully alphanumeric parse the cell reference
    if expr.chars().all(|c| c.is_alphanumeric() || c == '_') {
        match parse_cell_reference(sheet, expr) {
            Ok((target_row, target_col)) => {
                // Get value from parent cell first to avoid borrowing issues
                let parent_value;
                let parent1 = sheet.get_key(target_row, target_col);
                {
                    let parent_cell = sheet.get_cell(target_row, target_col);
                    parent_value = parent_cell.value.clone();
                }
                
                // Store old metadata for possible restoration
                let old_meta = if let Some(meta) = sheet.cell_meta.get(&cell_key) {
                    Some(meta.clone())
                } else {
                    None
                };
                
                let old_value;
                {
                    // Remove parents and set up new formula
                    remove_all_parents(sheet, row, col);
                    
                    let cell = sheet.get_mut_cell(row, col);
                    old_value = cell.value.clone();
                    cell.value = parent_value;
                    
                    // Set up the new cell metadata
                    let meta = sheet.get_cell_meta(row, col);
                    meta.parent1 = parent1;
                    meta.parent2 = -1;
                    meta.formula = 82;    // Custom formula code for reference
                }
                
                // Check for circular reference
                if detect_cycle(sheet, parent1, -1, 82, cell_key) {
                    // Restore old state if cycle detected
                    let cell = sheet.get_mut_cell(row, col);
                    cell.value = old_value;
                    
                    if let Some(old) = old_meta {
                        sheet.cell_meta.insert(cell_key, old.clone());
                        add_children(sheet, old.parent1, old.parent2, old.formula, row, col);
                    } else {
                        sheet.cell_meta.remove(&cell_key);
                    }
                    
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
                
                // Get left cell value
                let left_value = {
                    let left_cell = sheet.get_cell(rowl, coll);
                    match left_cell.value {
                        CellValue::Integer(val) => val,
                        _ => {
                            let cell = sheet.get_mut_cell(row, col);
                            cell.value = CellValue::Error;
                            return CommandStatus::CmdOk;
                        }
                    }
                };
                
                let right_status = parse_cell_reference(sheet, right);
                if right_status.is_err() {
                    return right_status.err().unwrap();
                }
                let (rowr, colr) = right_status.unwrap();
                
                // Get right cell value
                let right_value = {
                    let right_cell = sheet.get_cell(rowr, colr);
                    match right_cell.value {
                        CellValue::Integer(val) => val,
                        _ => {
                            let cell = sheet.get_mut_cell(row, col);
                            cell.value = CellValue::Error;
                            return CommandStatus::CmdOk;
                        }
                    }
                };
                
                // Setup the keys for parents
                let left_key = sheet.get_key(rowl, coll);
                let right_key = sheet.get_key(rowr, colr);
                
                // Store old metadata for possible restoration
                let old_meta = if let Some(meta) = sheet.cell_meta.get(&cell_key) {
                    Some(meta.clone())
                } else {
                    None
                };
                
                // Calculate the formula type
                let formula_type = if c == '+' {
                    10
                } else if c == '-'{ 
                    20
                } else if c == '*' {
                    40 // Code for multiplication
                } else {
                    30 // Code for division
                };
                
                let old_value;
                {
                    // Remove parents before updating
                    remove_all_parents(sheet, row, col);
                    
                    // Update cell value
                    let cell = sheet.get_mut_cell(row, col);
                    old_value = cell.value.clone();
                    
                    if c == '+' {
                        cell.value = CellValue::Integer(left_value + right_value);
                    } else if c == '-'{ 
                        cell.value = CellValue::Integer(left_value - right_value);
                    } else if c == '*' {
                        cell.value = CellValue::Integer(left_value * right_value);
                    } else if c == '/' { 
                        if right_value == 0 {
                            cell.value = CellValue::Error; // Division by zero
                            return CommandStatus::CmdOk;
                        }
                        cell.value = CellValue::Integer(left_value / right_value);
                    }
                    
                    // Update metadata
                    let meta = sheet.get_cell_meta(row, col);
                    meta.formula = formula_type;
                    meta.parent1 = left_key;
                    meta.parent2 = right_key;
                }
                
                // Check for circular reference
                if detect_cycle(sheet, left_key, right_key, formula_type, cell_key) {
                    // Restore old state if cycle detected
                    let cell = sheet.get_mut_cell(row, col);
                    cell.value = old_value;
                    
                    if let Some(old) = old_meta {
                        sheet.cell_meta.insert(cell_key, old.clone());
                        add_children(sheet, old.parent1, old.parent2, old.formula, row, col);
                    } else {
                        sheet.cell_meta.remove(&cell_key);
                    }
                    
                    return CommandStatus::CmdCircularRef;
                }
                
                // Add children after cycle check passes
                add_children(sheet, left_key, right_key, formula_type, row, col);
                
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

        // Extract the range string without allocating extra memory
        let range_str = &expr[prefix_len..expr_len - 1];

        // Parse range and validate early to avoid unnecessary work
        let range = match parse_range(sheet, range_str) {
            Ok(r) => r,
            Err(status) => return status,
        };

        let cell_key = sheet.get_key(row, col);
        let parent1 = sheet.get_key(range.start_row, range.start_col);
        let parent2 = sheet.get_key(range.end_row, range.end_col);

        // Store old metadata for possible restoration
        let old_meta = if let Some(meta) = sheet.cell_meta.get(&cell_key) {
            Some(meta.clone())
        } else {
            None
        };

        {
            remove_all_parents(sheet, row, col); 
            
                        let cell = sheet.get_mut_cell(row, col);
            cell.value = CellValue::Error; // Temporary value until we calculate the result

            // Update metadata
            let meta = sheet.get_cell_meta(row, col);
            meta.parent1 = parent1;
            meta.parent2 = parent2;
            meta.formula = if is_sum {
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
        }

        // Get the formula value before calling detect_cycle to avoid borrowing issues
        let formula_value = sheet.get_cell_meta(row, col).formula;
        
        // Evaluate the function.
        if detect_cycle(sheet, parent1, parent2, formula_value, cell_key) {
            // If a cycle is detected, restore the old parents and formula
            let cell = sheet.get_mut_cell(row, col);
            cell.value = CellValue::Error;

            if let Some(old) = old_meta {
                sheet.cell_meta.insert(cell_key, old.clone());
                add_children(sheet, old.parent1, old.parent2, old.formula, row, col);
            } else {
                sheet.cell_meta.remove(&cell_key);
            }

            return CommandStatus::CmdCircularRef;
        }

        let formula = sheet.get_cell_meta(row, col).formula;
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
    let status: CommandStatus = evaluate_formula(sheet, row, col, expr, sleep_time);
    status
}

pub fn handle_command(
    sheet: &mut Spreadsheet,
    input: &str,  // Changed from String to &str
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