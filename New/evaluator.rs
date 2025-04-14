use std::clone;
use std::collections::VecDeque;
use std::collections::HashSet;
use crate::spreadsheet::{Spreadsheet, CommandStatus};
use crate::cell::{Cell, CellValue, parse_cell_reference};
use crate::formula::{parse_range, Range};
use crate::formula::{eval_max, eval_min, sum_value, eval_variance};

pub fn get_key(row: i16, col: i16, cols: i16) -> i32 {
    ((row as i32) * (cols as i32) + (col as i32)) as i32
}

pub fn get_cell_from_key(spreadsheet: &mut Spreadsheet, key: i32) -> &mut Cell {
    let row = (key / spreadsheet.cols as i32) as i16;
    let col = (key % spreadsheet.cols as i32) as i16;
    spreadsheet.get_mut_cell(row, col)
}

// Fix get_cell_value function
pub fn get_cell_value(spreadsheet: &Spreadsheet, key: i32) -> CellValue {
    let row = (key / spreadsheet.cols as i32) as i16;
    let col = (key % spreadsheet.cols as i32) as i16;
    spreadsheet.get_cell(row, col).value.clone()
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
        
        // Remove old parents and setup new formula
        sheet.relationships.remove_all_parents(cell_key);
        
        // Update cell value
        {
            let cell = sheet.get_mut_cell(row, col);
            cell.value = parent_value.clone();
        }
        
        // Update relationships
        sheet.relationships.add_parent(cell_key, pkey);
        sheet.relationships.add_child(pkey, cell_key);
        sheet.relationships.set_formula(cell_key, 102);  // Code for sleep
        
        // Add to sleep time if integer
        if let CellValue::Integer(val) = parent_value {
            *sleep_time += val as f64;
        }
    } 
    // Handle numeric literal case
    else if let Ok(val) = expr.parse::<i32>() {
        // Remove old parents
        sheet.relationships.remove_all_parents(cell_key);
        
        // Update cell value
        let cell = sheet.get_mut_cell(row, col);
        cell.value = CellValue::Integer(val);
        
        // No formula or parents for literals
        sheet.relationships.set_formula(cell_key, -1);
        
        // Add to sleep time
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
    
    // Handle numeric literals
    if let Ok(number) = expr.parse::<i32>() {
        // Remove old parents
        sheet.relationships.remove_all_parents(cell_key);
        
        // Update cell value
        let cell = sheet.get_mut_cell(row, col);
        cell.value = CellValue::Integer(number);
        
        // No formula or parents for literals
        sheet.relationships.set_formula(cell_key, -1);
        
        return CommandStatus::CmdOk;
    }
    
    // Handle cell references
    if expr.chars().all(|c| c.is_alphanumeric() || c == '_') {
        match parse_cell_reference(sheet, expr) {
            Ok((target_row, target_col)) => {
                // Get value from parent cell first to avoid borrowing issues
                let parent_value;
                let parent_key = get_key(target_row, target_col, cols);
                {
                    let parent_cell = sheet.get_cell(target_row, target_col);
                    parent_value = parent_cell.value.clone();
                }
                
                // Remove old parents
                sheet.relationships.remove_all_parents(cell_key);
                
                // Update cell value
                {
                    let cell = sheet.get_mut_cell(row, col);
                    cell.value = parent_value;
                }
                
                // Update relationships
                sheet.relationships.add_parent(cell_key, parent_key);
                sheet.relationships.add_child(parent_key, cell_key);
                sheet.relationships.set_formula(cell_key, 82); // Code for reference
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
                // Parse left cell reference
                let left_status = parse_cell_reference(sheet, left);
                if left_status.is_err() {
                    return left_status.err().unwrap();
                }
                let (rowl, coll) = left_status.unwrap();
                let left_key = get_key(rowl, coll, cols);
                let left_value = if let CellValue::Integer(val) = sheet.get_cell(rowl, coll).value {
                    val
                } else {
                    let cell = sheet.get_mut_cell(row, col);
                    cell.value = CellValue::Error;
                    return CommandStatus::CmdOk;
                };
                
                // Parse right cell reference
                let right_status = parse_cell_reference(sheet, right);
                if right_status.is_err() {
                    return right_status.err().unwrap();
                }
                let (rowr, colr) = right_status.unwrap();
                let right_key = get_key(rowr, colr, cols);
                let right_value = if let CellValue::Integer(val) = sheet.get_cell(rowr, colr).value {
                    val
                } else {
                    let cell = sheet.get_mut_cell(row, col);
                    cell.value = CellValue::Error;
                    return CommandStatus::CmdOk;
                };
                
                // Determine formula code
                let formula_code = match c {
                    '+' => 10,
                    '-' => 20,
                    '*' => 40,
                    '/' => 30,
                    _ => unreachable!()
                };
                
                // Remove old parents
                sheet.relationships.remove_all_parents(cell_key);
                
                // Update cell value
                {
                    let cell = sheet.get_mut_cell(row, col);
                    cell.value = match c {
                        '+' => CellValue::Integer(left_value + right_value),
                        '-' => CellValue::Integer(left_value - right_value),
                        '*' => CellValue::Integer(left_value * right_value),
                        '/' => {
                            if right_value == 0 {
                                CellValue::Error // Division by zero
                            } else {
                                CellValue::Integer(left_value / right_value)
                            }
                        },
                        _ => unreachable!()
                    };
                }
                
                // Update relationships
                sheet.relationships.add_parent(cell_key, left_key);
                sheet.relationships.add_parent(cell_key, right_key);
                sheet.relationships.add_child(left_key, cell_key);
                sheet.relationships.add_child(right_key, cell_key);
                sheet.relationships.set_formula(cell_key, formula_code);
                
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
    let cell_key = get_key(row, col, cols);

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

        // Extract the range string
        let range_str = &expr[prefix_len..expr_len - 1];
        let range = match parse_range(sheet, range_str) {
            Ok(r) => r,
            Err(status) => return status,
        };

        // Remove old parents
        sheet.relationships.remove_all_parents(cell_key);
        
        // Set the formula code based on the function type
        let formula_code = if is_sum { 5 }
                          else if is_avg { 6 }
                          else if is_min { 7 }
                          else if is_max { 8 }
                          else { 9 }; // STDEV
                          
        sheet.relationships.set_formula(cell_key, formula_code);
        
        // Add all cells in the range as parents
        for r in range.start_row..=range.end_row {
            for c in range.start_col..=range.end_col {
                let parent_key = get_key(r, c, cols);
                sheet.relationships.add_parent(cell_key, parent_key);
                sheet.relationships.add_child(parent_key, cell_key);
            }
        }
        
        // Evaluate the function
        if is_stdev {
            return eval_variance(sheet, row, col, &range);
        } else if is_max {
            return eval_max(sheet, row, col, &range);
        } else if is_min {
            return eval_min(sheet, row, col, &range);
        } else if is_avg {
            let status = sum_value(sheet, row, col, &range);
            if status != CommandStatus::CmdOk {
                return status;
            }
            
            let count = ((range.end_row - range.start_row + 1) as i32) * ((range.end_col - range.start_col + 1) as i32);
            let cell = sheet.get_mut_cell(row, col);
            if let CellValue::Integer(sum) = cell.value {
                cell.value = CellValue::Integer(sum / count);
            } else {
                cell.value = CellValue::Error;
            }
            return CommandStatus::CmdOk;
        } else {
            return sum_value(sheet, row, col, &range);
        }
    } else if expr.starts_with("SLEEP(") {
        if !expr.ends_with(')') {
            return CommandStatus::CmdUnrecognized;
        }

        let sleep_str = &expr[6..expr_len - 1];
        return handle_sleep(sheet, row, col, sleep_str, sleep_time);
    }
    else {
        return evaluate_arithmetic(sheet, row, col, expr);
    }
}

// Fix recalculate_cell function - this is causing errors because it's mentioned
// in spreadsheet.rs but not implemented in evaluator.rs
pub fn recalculate_cell(sheet: &mut Spreadsheet, row: i16, col: i16, sleep_time: &mut f64) -> CommandStatus {
    let cols = sheet.cols;
    let cell_key = get_key(row, col, cols);
    
    // Get formula code for this cell
    let formula_code = sheet.relationships.get_formula(cell_key);
    
    if formula_code == -1 {
        // Literal value, nothing to recalculate
        return CommandStatus::CmdOk;
    }
    
    // For binary operations and references, get the formula and re-evaluate
    match formula_code {
        // Simple cell reference
        82 => {
            let parents = sheet.relationships.get_parents(cell_key);
            if let Some(&parent_key) = parents.first() {
                let parent_row = (parent_key / cols as i32) as i16;
                let parent_col = (parent_key % cols as i32) as i16;
                let parent_value = sheet.get_cell(parent_row, parent_col).value.clone();
                let mut cell = sheet.get_mut_cell(row, col);
                cell.value = parent_value;
            }
        },
        
        // Binary operations
        10 | 20 | 30 | 40 => {
            let parents = sheet.relationships.get_parents(cell_key);
            if parents.len() >= 2 {
                // Get values from both parents
                let parent1_key = parents[0];
                let parent2_key = parents[1];
                
                let parent1_row = (parent1_key / cols as i32) as i16;
                let parent1_col = (parent1_key % cols as i32) as i16;
                let parent1_value = match sheet.get_cell(parent1_row, parent1_col).value {
                    CellValue::Integer(val) => val,
                    _ => {
                        // Error in parent, propagate error
                        let mut cell = sheet.get_mut_cell(row, col);
                        cell.value = CellValue::Error;
                        return CommandStatus::CmdOk;
                    }
                };
                
                let parent2_row = (parent2_key / cols as i32) as i16;
                let parent2_col = (parent2_key % cols as i32) as i16;
                let parent2_value = match sheet.get_cell(parent2_row, parent2_col).value {
                    CellValue::Integer(val) => val,
                    _ => {
                        // Error in parent, propagate error
                        let mut cell = sheet.get_mut_cell(row, col);
                        cell.value = CellValue::Error;
                        return CommandStatus::CmdOk;
                    }
                };
                
                // Apply the operation
                let mut cell = sheet.get_mut_cell(row, col);
                match formula_code {
                    10 => cell.value = CellValue::Integer(parent1_value + parent2_value),
                    20 => cell.value = CellValue::Integer(parent1_value - parent2_value),
                    30 => {
                        if parent2_value == 0 {
                            cell.value = CellValue::Error; // Division by zero
                        } else {
                            cell.value = CellValue::Integer(parent1_value / parent2_value);
                        }
                    },
                    40 => cell.value = CellValue::Integer(parent1_value * parent2_value),
                    _ => unreachable!()
                }
            }
        },
        
        // Range functions (SUM, AVG, MIN, MAX, STDEV)
        5 | 6 | 7 | 8 | 9 => {
            // For range functions, reconstruct the range from parents
            let parents = sheet.relationships.get_parents(cell_key);
            
            if !parents.is_empty() {
                // Find the bounding box of all parent cells
                let mut min_row = i16::MAX;
                let mut min_col = i16::MAX;
                let mut max_row = i16::MIN;
                let mut max_col = i16::MIN;
                
                for &parent in &parents {
                    let pr = (parent / cols as i32) as i16;
                    let pc = (parent % cols as i32) as i16;
                    min_row = min_row.min(pr);
                    min_col = min_col.min(pc);
                    max_row = max_row.max(pr);
                    max_col = max_col.max(pc);
                }
                
                let range = Range {
                    start_row: min_row,
                    start_col: min_col,
                    end_row: max_row,
                    end_col: max_col
                };
                
                // Apply the appropriate range function
                match formula_code {
                    5 => return sum_value(sheet, row, col, &range),
                    6 => {
                        // AVG = SUM / COUNT
                        sum_value(sheet, row, col, &range);
                        let count = (range.end_row - range.start_row + 1) * 
                                  (range.end_col - range.start_col + 1) as i16;
                        let mut cell = sheet.get_mut_cell(row, col);
                        if let CellValue::Integer(sum) = cell.value {
                            cell.value = CellValue::Integer(sum / count as i32);
                        }
                    },
                    7 => return eval_min(sheet, row, col, &range),
                    8 => return eval_max(sheet, row, col, &range),
                    9 => return eval_variance(sheet, row, col, &range),
                    _ => unreachable!()
                }
            } else {
                // No parents, set error
                let mut cell = sheet.get_mut_cell(row, col);
                cell.value = CellValue::Error;
            }
        },
        
        // SLEEP function
        102 => {
            let parents = sheet.relationships.get_parents(cell_key);
            if let Some(&parent_key) = parents.first() {
                let parent_row = (parent_key / cols as i32) as i16;
                let parent_col = (parent_key % cols as i32) as i16;
                let parent_value = sheet.get_cell(parent_row, parent_col).value.clone();
                
                let mut cell = sheet.get_mut_cell(row, col);
                cell.value = parent_value.clone();
                
                // Update sleep time if needed
                if let CellValue::Integer(val) = parent_value {
                    *sleep_time += val as f64;
                }
            }
        },
        
        // Unknown formula code
        _ => return CommandStatus::CmdUnrecognized
    }
    
    CommandStatus::CmdOk
}

pub fn set_cell_value(sheet: &mut Spreadsheet, row: i16, col: i16, expr: &str, sleep_time: &mut f64) -> CommandStatus {
    // Use the re_evaluate_topological function for cell value setting
    let cell_key = get_key(row, col, sheet.cols);
    sheet.re_evaluate_topological(cell_key, expr, sleep_time)
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
