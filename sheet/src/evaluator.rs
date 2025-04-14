use crate::spreadsheet::{Spreadsheet, CommandStatus};
use crate::cell::{CellValue, parse_cell_reference};
use crate::formula::parse_range;
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
        // Get parent key before any borrowing
        let pkey = sheet.get_key(target_row, target_col);
        
        // Check for self-reference early (optimization)
        if row == target_row && col == target_col {
            return CommandStatus::CmdCircularRef;
        }
        
        // Get the value from parent cell
        let parent_value = sheet.get_cell(target_row, target_col).value.clone();
        
        // Store old metadata and value for possible restoration
        let old_meta = sheet.cell_meta.get(&cell_key).cloned();

        // Remove parents and update cell in one block
        remove_all_parents(sheet, row, col);
                
        // Set up the new cell metadata
        let meta = sheet.get_cell_meta(row, col);
        meta.parent1 = pkey;
        meta.parent2 = -1;
        meta.formula = 102;    // Custom formula code for sleep
        
        // Check for circular reference
        if detect_cycle(sheet, pkey, -1, 102, cell_key) {
            if let Some(old) = old_meta {
                let (parent1, parent2, formula) = (old.parent1, old.parent2, old.formula);
                sheet.cell_meta.insert(cell_key, old);
                add_children(sheet, parent1, parent2, formula, row, col);
            } else {
                sheet.cell_meta.remove(&cell_key);
            }
            return CommandStatus::CmdCircularRef;
        }
        
        // Add children and update sleep time
        add_children(sheet, pkey, -1, 102, row, col);
        // Update cell value
        sheet.get_mut_cell(row, col).value = parent_value.clone();
        // Add to sleep time if integer
        if let CellValue::Integer(val) = parent_value {
            *sleep_time += val as f64;
        }
    } 
    // Handle numeric literal case
    else if let Ok(val) = expr.parse::<i32>() {
        // Remove all parents and update cell in one sequence
        remove_all_parents(sheet, row, col);
        // Update cell value
        sheet.get_mut_cell(row, col).value = CellValue::Integer(val);

        // Update metadata directly through get_cell_meta
        let meta = sheet.get_cell_meta(row, col);
        meta.formula = -1;
        meta.parent1 = -1;
        meta.parent2 = -1;

        // Update sleep time
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
    
    // Case 1: Integer literal
    if let Ok(number) = expr.parse::<i32>() {
        remove_all_parents(sheet, row, col);
        
        let cell = sheet.get_mut_cell(row, col);
        cell.value = CellValue::Integer(number);
        
        // As no parents and formula remove the meta data from the set and map
        // to avoid memory leaks
        sheet.cell_meta.remove(&cell_key);
        // let meta = sheet.get_cell_meta(row, col);
        // meta.formula = -1;
        // meta.parent1 = -1;
        // meta.parent2 = -1;
        return CommandStatus::CmdOk;
    }
    
    // Case 2: Simple cell reference - check using bytes for better performance
    let mut all_alnum = true;
    for &b in expr.as_bytes() {
        if !(b.is_ascii_alphanumeric() || b == b'_') {
            all_alnum = false;
            break;
        }
    }
    
    if all_alnum {
        match parse_cell_reference(sheet, expr) {
            Ok((target_row, target_col)) => {
                
                // Get reference cell key and value
                let ref_cell_key = sheet.get_key(target_row, target_col);
                let ref_cell_value = sheet.get_cell(target_row, target_col).value.clone();
                let error_state = matches!(ref_cell_value, CellValue::Error);
                
                // Save old state
                let old_meta = sheet.cell_meta.get(&cell_key).cloned();
                
                // Remove old dependencies and set new ones
                remove_all_parents(sheet, row, col);
                
                // Update metadata
                let meta = sheet.get_cell_meta(row, col);
                meta.parent1 = ref_cell_key;
                meta.parent2 = -1;
                meta.formula = 82;  // Code for simple cell reference
                
                // Check for cycles
                if detect_cycle(sheet, ref_cell_key, -1, 82, cell_key) {
                    // Restore old state if cycle detected
                    if let Some(old) = old_meta {
                        let (parent1, parent2, formula) = (old.parent1, old.parent2, old.formula);
                        sheet.cell_meta.insert(cell_key, old);
                        add_children(sheet, parent1, parent2, formula, row, col);
                    } else {
                        sheet.cell_meta.remove(&cell_key);
                    }
                    return CommandStatus::CmdCircularRef;
                }
                
                // Add dependency
                add_children(sheet, ref_cell_key, -1, 82, row, col);
                
                // Update cell value
                let cell = sheet.get_mut_cell(row, col);
                if error_state {
                    cell.value = CellValue::Error;
                } else {
                    cell.value = ref_cell_value;
                }
                
                return CommandStatus::CmdOk;
            },
            Err(status) => return status
        }
    }

    // Case 3: Binary arithmetic expression
    // Find operator starting at index 1 (like C code, to handle leading minus sign)
    let bytes = expr.as_bytes();
    let mut op_idx = 0;
    let mut op = 0u8;
    
    // Start at index 1 to handle leading minus sign
    for i in 1..bytes.len() {
        match bytes[i] {
            b'+' | b'-' | b'*' | b'/' => {
                op = bytes[i];
                op_idx = i;
                break;
            },
            _ => {}
        }
    }
    
    if op_idx == 0 {
        return CommandStatus::CmdUnrecognized;
    }
    
    // Split into left and right parts
    let left = &expr[..op_idx].trim();
    let right = &expr[op_idx+1..].trim();
    
    if left.is_empty() || right.is_empty() {
        return CommandStatus::CmdUnrecognized;
    }
    
    // Variables to track cell references and values
    let mut left_val = 0;
    let mut right_val = 0;
    let mut left_is_cell = false;
    let mut right_is_cell = false;
    let mut error_found = false;
    let mut left_cell_key = -1;
    let mut right_cell_key = -1;
    
    // Parse left operand
    if let Ok(num) = left.parse::<i32>() {
        left_val = num;
    } else {
        // Try as cell reference
        match parse_cell_reference(sheet, left) {
            Ok((left_row, left_col)) => {      

                left_is_cell = true;
                left_cell_key = sheet.get_key(left_row, left_col);
                
                // Get reference cell value
                let left_cell = sheet.get_cell(left_row, left_col);
                match left_cell.value {
                    CellValue::Integer(val) => left_val = val,
                    _ => {
                        error_found = true;
                    }
                }
            },
            Err(status) => return status
        }
    }
    
    // Parse right operand
    if let Ok(num) = right.parse::<i32>() {
        right_val = num;
    } else {
        // Try as cell reference
        match parse_cell_reference(sheet, right) {
            Ok((right_row, right_col)) => {
                
                right_is_cell = true;
                right_cell_key = sheet.get_key(right_row, right_col);
                
                // Get reference cell value
                let right_cell = sheet.get_cell(right_row, right_col);
                match right_cell.value {
                    CellValue::Integer(val) => right_val = val,
                    _ => {
                        error_found = true;
                    }
                }
            },
            Err(status) => return status
        }
    }
    
    // Save old metadata for restoration if needed
    let old_meta = sheet.cell_meta.get(&cell_key).cloned();
    
    // Remove old dependencies
    remove_all_parents(sheet, row, col);
    
    // Determine formula type based on operator and operand types
    let mut formula_type = match op {
        b'+' => 10,
        b'-' => 20,
        b'*' => 40,
        b'/' => 30,
        _ => unreachable!()
    };
    
    // Adjust formula type based on cell references (like C code)
    if left_is_cell && right_is_cell {
        formula_type += 0;    // Both are cells, no adjustment needed
    } else if left_is_cell {
        formula_type += 2;
    } else if right_is_cell {
        formula_type += 3;
    }
    
    // Set metadata
    {
        let meta = sheet.get_cell_meta(row, col);
        meta.formula = formula_type;
        meta.parent1 = if left_is_cell { left_cell_key } else { left_val };
        meta.parent2 = if right_is_cell { right_cell_key } else { right_val };
    }
    
    // Check for circular references
    let mut has_cycle = false;
    
    if left_is_cell && right_is_cell {
        has_cycle = detect_cycle(sheet, left_cell_key, right_cell_key, formula_type, cell_key);
    }
    else if left_is_cell {
        has_cycle = detect_cycle(sheet, left_cell_key, -1, formula_type, cell_key);
    }
    else if right_is_cell {
        has_cycle = detect_cycle(sheet, -1, right_cell_key, formula_type, cell_key);
    }
    
    if has_cycle {
        // Restore old state
        if let Some(old) = old_meta {
            sheet.cell_meta.insert(cell_key, old.clone());
            add_children(sheet, old.parent1, old.parent2, old.formula, row, col);
        } else {
            sheet.cell_meta.remove(&cell_key);
        }

        return CommandStatus::CmdCircularRef;
    }
    
    // Add dependencies
    if left_is_cell && right_is_cell {
        // Add dependencies for both cells
        add_children(sheet, left_cell_key, right_cell_key, formula_type, row, col);
    }
    else if left_is_cell {
        add_children(sheet, left_cell_key, -1, formula_type, row, col);
    }
    else if right_is_cell {
        add_children(sheet, right_cell_key, -1, formula_type, row, col);
    }
    
    // Calculate result
    let cell = sheet.get_mut_cell(row, col);
    
    if error_found {
        cell.value = CellValue::Error;
    } else {
        match op {
            b'+' => cell.value = CellValue::Integer(left_val + right_val),
            b'-' => cell.value = CellValue::Integer(left_val - right_val),
            b'*' => cell.value = CellValue::Integer(left_val * right_val),
            b'/' => {
                if right_val == 0 {
                    cell.value = CellValue::Error;
                } else {
                    cell.value = CellValue::Integer(left_val / right_val);
                }
            },
            _ => unreachable!()
        }
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
    // Fast fail for empty expression
    if expr.is_empty() {
        return CommandStatus::CmdUnrecognized;
    }

    // Optimize function checks by using bytes for prefix matching
    let bytes = expr.as_bytes();
    
    // Check for range-based functions with a single pass
    let (is_formula, formula_type, prefix_len) = match bytes.get(0..3) {
        Some(b"AVG") if bytes.get(3) == Some(&b'(') => (true, 6, 4),
        Some(b"MIN") if bytes.get(3) == Some(&b'(') => (true, 7, 4),
        Some(b"MAX") if bytes.get(3) == Some(&b'(') => (true, 8, 4),
        Some(b"SUM") if bytes.get(3) == Some(&b'(') => (true, 5, 4),
        Some(b"SLE") if bytes.len() > 5 && 
                        bytes[3] == b'E' && 
                        bytes[4] == b'P' && 
                        bytes.get(5) == Some(&b'(') => {
            // Handle sleep function separately
            if !expr.ends_with(')') {
                return CommandStatus::CmdUnrecognized;
            }
            return handle_sleep(sheet, row, col, &expr[6..expr.len() - 1], sleep_time);
        },
        Some(b"STD") if bytes.len() > 5 && 
                        bytes[3] == b'E' && 
                        bytes[4] == b'V' && 
                        bytes.get(5) == Some(&b'(') => (true, 9, 6),
        _ => (false, 0, 0),
    };

    if is_formula {
        // Validate formula format
        if !expr.ends_with(')') {
            return CommandStatus::CmdUnrecognized;
        }

        // Extract the range string without allocating extra memory
        let range_str: &str = &expr[prefix_len..expr.len() - 1];

        // Parse range and validate early to avoid unnecessary work
        let range = match parse_range(sheet, range_str) {
            Ok(r) => r,
            Err(status) => return status,
        };

        let cell_key = sheet.get_key(row, col);
        let parent1 = sheet.get_key(range.start_row, range.start_col);
        let parent2 = sheet.get_key(range.end_row, range.end_col);

        // Store old metadata for possible restoration
        let old_meta = sheet.cell_meta.get(&cell_key).cloned();

        {
            remove_all_parents(sheet, row, col); 
            // Update metadata
            let meta = sheet.get_cell_meta(row, col);
            meta.parent1 = parent1;
            meta.parent2 = parent2;
            meta.formula = formula_type;
        }

        // Check for circular reference
        if detect_cycle(sheet, parent1, parent2, formula_type, cell_key) {
            // If a cycle is detected, restore the old parents and formula
            if let Some(old) = old_meta {
                let parent1 = old.parent1;
                let parent2 = old.parent2;
                let formula = old.formula;
                sheet.cell_meta.insert(cell_key, old);
                add_children(sheet, parent1, parent2, formula, row, col);
            } else {
                sheet.cell_meta.remove(&cell_key);
            }

            return CommandStatus::CmdCircularRef;
        }

        // Add children and evaluate the appropriate function
        add_children(sheet, parent1, parent2, formula_type, row, col);
        
        match formula_type {
            9 => eval_variance(sheet, row, col, &range),
            8 => eval_max(sheet, row, col, &range),
            7 => eval_min(sheet, row, col, &range),
            6 => {
                // AVG case
                let status = sum_value(sheet, row, col, &range);
                if status != CommandStatus::CmdOk {
                    return status;
                }
                
                let count = ((range.end_row - range.start_row + 1) as i32) * 
                           ((range.end_col - range.start_col + 1) as i32);
                           
                let cell: &mut crate::cell::Cell = sheet.get_mut_cell(row, col);
                if let CellValue::Integer(sum) = cell.value {
                    cell.value = CellValue::Integer(sum / count);
                } else {
                    cell.value = CellValue::Error;
                }
                CommandStatus::CmdOk
            },
            _ => sum_value(sheet, row, col, &range), // SUM case
        }
    } else {
        // Handle arithmetic expressions
        evaluate_arithmetic(sheet, row, col, expr)
    }
}

pub fn set_cell_value(sheet: &mut Spreadsheet, row: i16, col: i16, expr: &str, sleep_time: &mut f64) -> CommandStatus {
    let status: CommandStatus = evaluate_formula(sheet, row, col, expr, sleep_time);
    status
}

pub fn handle_command(
    sheet: &mut Spreadsheet,
    trimmed: &str,
    sleep_time: &mut f64,
) -> CommandStatus {    
    // Fast path for single-character commands to avoid string comparisons
    if trimmed.len() == 1 {
        match trimmed.as_bytes()[0] {
            b'w' | b'a' | b's' | b'd' => {
                // We've already validated it's one byte, so this is safe
                let direction = trimmed.chars().next().unwrap();
                sheet.scroll_viewport(direction);
                return CommandStatus::CmdOk;
            },
            b'q' => return CommandStatus::CmdOk, // Handle quit command if needed
            _ => {}
        }
    }
    
    // Use match for special commands for better branch prediction
    match trimmed {
        "disable_output" => {
            sheet.output_enabled = false;
            return CommandStatus::CmdOk;
        },
        "enable_output" => {
            sheet.output_enabled = true;
            return CommandStatus::CmdOk;
        },
        _ => {}
    }
    
    // Check for scroll_to command with byte-based comparison
    if trimmed.len() > 10 && &trimmed.as_bytes()[..9] == b"scroll_to" && trimmed.as_bytes()[9] == b' ' {
        let cell_ref = &trimmed[10..];
        return sheet.scroll_to_cell(cell_ref);
    }
    
    // Check for cell assignment using byte search for '='
    let bytes = trimmed.as_bytes();
    let mut eq_pos = None;
    
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'=' {
            eq_pos = Some(i);
            break;
        }
    }
    
    if let Some(pos) = eq_pos {
        // Use slice operations which are more efficient than split_at
        let cell_ref = trimmed[..pos].trim();
        let expr = trimmed[pos+1..].trim();
        
        // Parse the cell reference with direct result handling
        return match parse_cell_reference(sheet, cell_ref) {
            Ok((row, col)) => {
                // All bounds checks in one condition
                set_cell_value(sheet, row, col, expr, sleep_time)
            },
            Err(status) => status,
        };
    }
    // No recognized command
    CommandStatus::CmdUnrecognized
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_handle_command() {
//         let mut sheet = Spreadsheet::create(5, 5).unwrap();
//         let mut sleep_time = 0.0;
//         assert_eq!(handle_command(&mut sheet, "A1=42".to_string(), &mut sleep_time), CommandStatus::CmdOk);
//         assert_eq!(sheet.get_cell(0, 0).value, CellValue::Integer(42));
//         assert_eq!(handle_command(&mut sheet, "disable_output".to_string(), &mut sleep_time), CommandStatus::CmdOk);
//         assert_eq!(handle_command(&mut sheet, "w".to_string(), &mut sleep_time), CommandStatus::CmdOk);
//         assert_eq!(handle_command(&mut sheet, "scroll_to B2".to_string(), &mut sleep_time), CommandStatus::CmdOk);
//     }

//     #[test]
//     fn test_evaluate_arithmetic() {
//         let mut sheet = Spreadsheet::create(5, 5).unwrap();
//         assert_eq!(evaluate_arithmetic(&mut sheet, 0, 0, "42"), CommandStatus::CmdOk);
//         assert_eq!(sheet.get_cell(0, 0).value, CellValue::Integer(42));
//         assert_eq!(evaluate_arithmetic(&mut sheet, 0, 1, "A1"), CommandStatus::CmdOk);
//         assert_eq!(sheet.get_cell(0, 1).value, CellValue::Integer(42));
//         assert_eq!(evaluate_arithmetic(&mut sheet, 1, 0, "A1 + B1"), CommandStatus::CmdOk);
//         assert_eq!(sheet.get_cell(1, 0).value, CellValue::Integer(84));
//         assert_eq!(evaluate_arithmetic(&mut sheet, 1, 1, "A1 / B2"), CommandStatus::CmdOk); // B2 is 0
//         assert_eq!(sheet.get_cell(1, 1).value, CellValue::Error);
//     }

//     #[test]
//     fn test_handle_sleep() {
//         let mut sheet = Spreadsheet::create(5, 5).unwrap();
//         let mut sleep_time = 0.0;
//         assert_eq!(handle_sleep(&mut sheet, 0, 0, "2", &mut sleep_time), CommandStatus::CmdOk);
//         assert_eq!(sleep_time, 2.0);
//         assert_eq!(handle_sleep(&mut sheet, 0, 1, "A1", &mut sleep_time), CommandStatus::CmdOk);
//     }
// }