use crate::cell::{CellValue, parse_cell_reference};
use crate::extensions::get_formula_string;
use crate::formula::Range;
use crate::formula::parse_range;
use crate::formula::{eval_avg, eval_max, eval_min, eval_variance, sum_value};
use crate::graph::{add_children, remove_all_parents};
use crate::reevaluate_topo::{sleep_fn, toposort_reval_detect_cycle};
use crate::spreadsheet::{CommandStatus, HighlightType, Spreadsheet};

/// Resolves a cell reference or named range to its coordinates.
///
/// # Arguments
///
/// * `sheet` - The spreadsheet containing named ranges.
/// * `s` - The string to resolve (cell reference or name).
///
/// # Returns
///
/// * `Ok((row, col))` - The zero-based coordinates.
/// * `Err(CommandStatus::Unrecognized)` - If resolution fails
fn resolve_cell_reference(sheet: &Spreadsheet, s: &str) -> Result<(i16, i16), CommandStatus> {
    if let Some(range) = sheet.named_ranges.get(s) {
        if range.start_row == range.end_row && range.start_col == range.end_col {
            Ok((range.start_row, range.start_col))
        } else {
            Err(CommandStatus::Unrecognized)
        }
    } else {
        parse_cell_reference(sheet, s)
    }
}

/// Handles the `SLEEP` command, setting a cell value and accumulating sleep time.
///
/// # Arguments
///
/// * `sheet` - The mutable spreadsheet.
/// * `row` - The target row.
/// * `col` - The target column.
/// * `expr` - The expression (cell reference or literal).
/// * `sleep_time` - Accumulates sleep duration in seconds.
///
/// # Returns
///
/// * `CommandStatus::CmdOk` - On success.
/// * `CommandStatus::CircularRef` - If self-referencing.
/// * `CommandStatus::Unrecognized` - If expression is invalid.
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
            return CommandStatus::CircularRef;
        }

        // Remove parents and update cell in one block
        remove_all_parents(sheet, row, col);

        // Set up the new cell metadata
        let meta = sheet.get_cell_meta(row, col);
        meta.parent1 = pkey;
        meta.parent2 = -1;
        meta.formula = 102; // Custom formula code for sleep

        // Add children and update sleep time
        add_children(sheet, pkey, -1, 102, row, col);
        // Add to sleep time if integer
        // Get the value from parent cell
        let parent_value = sheet.get_cell(target_row, target_col);
        if let CellValue::Integer(val) = parent_value {
            // Update cell value and sleep time
            sleep_fn(sheet, row, col, *val, sleep_time);
        } else {
            *sheet.get_mut_cell(row, col) = CellValue::Error;
        }
    }
    // Handle numeric literal case
    else if let Ok(val) = expr.parse::<i32>() {
        // Remove all parents and update cell in one sequence
        remove_all_parents(sheet, row, col);
        // Update cell value and sleep_time
        // Delete the cell meta entry
        sheet.cell_meta.remove(&cell_key);
        sleep_fn(sheet, row, col, val, sleep_time);
    } else {
        return CommandStatus::Unrecognized;
    }

    CommandStatus::CmdOk
}

/// Evaluates an arithmetic expression and updates a cell.
///
/// Supports literals, cell references, and operations (+, -, *, /).
///
/// # Arguments
///
/// * `sheet` - The mutable spreadsheet.
/// * `row` - The target row.
/// * `col` - The target column.
/// * `expr` - The arithmetic expression.
///
/// # Returns
///
/// * `CommandStatus::CmdOk` - On success.
/// * `CommandStatus::Unrecognized` - If expression is invalid.
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
        // As no parents and formula remove the meta data from the set and map
        // to avoid memory leaks
        sheet.cell_meta.remove(&cell_key);
        *sheet.get_mut_cell(row, col) = CellValue::Integer(number);

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
        match resolve_cell_reference(sheet, expr) {
            Ok((target_row, target_col)) => {
                // Get reference cell key and value
                let ref_cell_key = sheet.get_key(target_row, target_col);

                // Remove old dependencies and set new ones
                remove_all_parents(sheet, row, col);

                // Update metadata
                let meta = sheet.get_cell_meta(row, col);
                meta.parent1 = ref_cell_key;
                meta.parent2 = -1;
                meta.formula = 82; // Code for simple cell reference

                // Add dependency
                add_children(sheet, ref_cell_key, -1, 82, row, col);

                // Update cell value
                *sheet.get_mut_cell(row, col) = sheet.get_cell(target_row, target_col).clone();

                return CommandStatus::CmdOk;
            }
            Err(status) => return status,
        }
    }

    // Case 3: Binary arithmetic expression
    // Find operator starting at index 1 (like C code, to handle leading minus sign)
    let bytes = expr.as_bytes();
    let mut op_idx = 0;
    let mut op = 0u8;

    // Start at index 1 to handle leading minus sign
    for (i, &byte) in bytes.iter().enumerate().skip(1) {
        match byte {
            b'+' | b'-' | b'*' | b'/' => {
                op = byte;
                op_idx = i;
                break;
            }
            _ => {}
        }
    }

    if op_idx == 0 {
        return CommandStatus::Unrecognized;
    }

    // Split into left and right parts
    let left = &expr[..op_idx];
    let right = &expr[op_idx + 1..];

    if left.is_empty() || right.is_empty() {
        return CommandStatus::Unrecognized;
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
                match left_cell {
                    CellValue::Integer(val) => left_val = *val,
                    _ => {
                        error_found = true;
                    }
                }
            }
            Err(status) => return status,
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
                match right_cell {
                    CellValue::Integer(val) => right_val = *val,
                    _ => {
                        error_found = true;
                    }
                }
            }
            Err(status) => return status,
        }
    }

    // Remove old dependencies
    remove_all_parents(sheet, row, col);

    // Determine formula type based on operator and operand types
    let mut formula_type = match op {
        b'+' => 10,
        b'-' => 20,
        b'*' => 40,
        b'/' => 30,
        _ => unreachable!(),
    };

    // Adjust formula type based on cell references (like C code)
    if left_is_cell && right_is_cell {
        formula_type += 0; // Both are cells, no adjustment needed
    } else if left_is_cell {
        formula_type += 2;
    } else if right_is_cell {
        formula_type += 3;
    }

    // Set metadata
    let meta = sheet.get_cell_meta(row, col);
    meta.formula = formula_type;
    meta.parent1 = if left_is_cell {
        left_cell_key
    } else {
        left_val
    };
    meta.parent2 = if right_is_cell {
        right_cell_key
    } else {
        right_val
    };

    // Check for circular references

    // Add dependencies
    if left_is_cell && right_is_cell {
        // Add dependencies for both cells
        add_children(sheet, left_cell_key, right_cell_key, formula_type, row, col);
    } else if left_is_cell {
        // Ordering of Cells matters
        add_children(sheet, left_cell_key, -1, formula_type, row, col);
    } else if right_is_cell {
        // Ordering of Cells matters
        add_children(sheet, -1, right_cell_key, formula_type, row, col);
    }

    // Calculate result
    let cell = sheet.get_mut_cell(row, col);

    if error_found {
        *cell = CellValue::Error;
    } else {
        match op {
            b'+' => *cell = CellValue::Integer(left_val + right_val),
            b'-' => *cell = CellValue::Integer(left_val - right_val),
            b'*' => *cell = CellValue::Integer(left_val * right_val),
            b'/' => {
                if right_val == 0 {
                    *cell = CellValue::Error;
                } else {
                    *cell = CellValue::Integer(left_val / right_val);
                }
            }
            _ => unreachable!(),
        }
    }

    CommandStatus::CmdOk
}

/// Evaluates a formula, supporting arithmetic and range functions.
///
/// # Arguments
///
/// * `sheet` - The mutable spreadsheet.
/// * `row` - The target row.
/// * `col` - The target column.
/// * `expr` - The formula (e.g., "A1+B1", "SUM(A1:B2)").
/// * `sleep_time` - Accumulates sleep time for `SLEEP`.
///
/// # Returns
///
/// * `CommandStatus::CmdOk` - On success.
/// * `CommandStatus::Unrecognized` - If formula is invalid
pub fn evaluate_formula(
    sheet: &mut Spreadsheet,
    row: i16,
    col: i16,
    expr: &str,
    sleep_time: &mut f64,
) -> CommandStatus {
    // Fast fail for empty expression
    if expr.is_empty() {
        return CommandStatus::Unrecognized;
    }

    // Optimize function checks by using bytes for prefix matching
    let bytes = expr.as_bytes();

    // Check for range-based functions with a single pass
    let (is_formula, formula_type, prefix_len) = match bytes.get(0..3) {
        Some(b"AVG") if bytes.get(3) == Some(&b'(') => (true, 6, 4),
        Some(b"MIN") if bytes.get(3) == Some(&b'(') => (true, 7, 4),
        Some(b"MAX") if bytes.get(3) == Some(&b'(') => (true, 8, 4),
        Some(b"SUM") if bytes.get(3) == Some(&b'(') => (true, 5, 4),
        Some(b"SLE")
            if bytes.len() > 5
                && bytes[3] == b'E'
                && bytes[4] == b'P'
                && bytes.get(5) == Some(&b'(') =>
        {
            // Handle sleep function separately
            if !expr.ends_with(')') {
                return CommandStatus::Unrecognized;
            }
            return handle_sleep(sheet, row, col, &expr[6..expr.len() - 1], sleep_time);
        }
        Some(b"STD")
            if bytes.len() > 5
                && bytes[3] == b'E'
                && bytes[4] == b'V'
                && bytes.get(5) == Some(&b'(') =>
        {
            (true, 9, 6)
        }
        _ => (false, -1, 0),
    };

    if is_formula {
        // Validate formula format
        if !expr.ends_with(')') {
            return CommandStatus::Unrecognized;
        }

        // Extract the range string without allocating extra memory
        let range_str: &str = &expr[prefix_len..expr.len() - 1];

        // Parse range and validate early to avoid unnecessary work
        let range: Range = if let Some(named_range) = sheet.named_ranges.get(range_str) {
            named_range.clone()
        } else {
            match parse_range(sheet, range_str) {
                Ok(r) => r,
                Err(status) => return status,
            }
        };

        // let cell_key = sheet.get_key(row, col);
        let parent1 = sheet.get_key(range.start_row, range.start_col);
        let parent2 = sheet.get_key(range.end_row, range.end_col);
        remove_all_parents(sheet, row, col);
        // Update metadata
        let meta = sheet.get_cell_meta(row, col);
        meta.parent1 = parent1;
        meta.parent2 = parent2;
        meta.formula = formula_type;

        // Add children and evaluate the appropriate function
        add_children(sheet, parent1, parent2, formula_type, row, col);

        match formula_type {
            9 => eval_variance(sheet, row, col, parent1, parent2),
            8 => eval_max(sheet, row, col, parent1, parent2),
            7 => eval_min(sheet, row, col, parent1, parent2),
            6 => eval_avg(sheet, row, col, parent1, parent2),
            _ => sum_value(sheet, row, col, parent1, parent2), // SUM case
        }
    } else {
        // Handle arithmetic expressions
        evaluate_arithmetic(sheet, row, col, expr)
    }
}

/// Sets a cell’s value based on an expression, managing dependencies.
///
/// # Arguments
///
/// * `sheet` - The mutable spreadsheet.
/// * `row` - The target row.
/// * `col` - The target column.
/// * `expr` - The expression to evaluate.
/// * `sleep_time` - Accumulates sleep time.
///
/// # Returns
///
/// * `CommandStatus::CmdOk` - On success.
/// * `CommandStatus::CircularRef` - If a cycle is detected.
/// * `CommandStatus::LockedCell` - If the cell is locked.
/// * `CommandStatus::Unrecognized` - If expression is invalid.
pub fn set_cell_value(
    sheet: &mut Spreadsheet,
    row: i16,
    col: i16,
    expr: &str,
    sleep_time: &mut f64,
) -> CommandStatus {
    if sheet.is_cell_locked(row, col) {
        return CommandStatus::LockedCell;
    }
    let cell_key = sheet.get_key(row, col);

    // Save old state
    let old_meta = sheet.cell_meta.get(&cell_key).cloned();
    let old_value = match sheet.get_cell(row, col) {
        CellValue::Integer(val) => CellValue::Integer(*val),
        _ => CellValue::Error,
    };
    let status: CommandStatus = evaluate_formula(sheet, row, col, expr, sleep_time);
    if let CommandStatus::CmdOk = status {
        // Reevaluate the cell dependents graphs i.e. all of its children
        // Also at same time check for cycle in the graph as it will save time and memory
        let has_cycle = toposort_reval_detect_cycle(sheet, row, col, sleep_time);
        if has_cycle {
            // If a cycle is detected, restore the old parents and formula
            // Remove the new parents and formula
            remove_all_parents(sheet, row, col);
            // Restore the old value
            *sheet.get_mut_cell(row, col) = old_value;
            // Old meta
            if let Some(old) = old_meta {
                let (parent1, parent2, formula) = (old.parent1, old.parent2, old.formula);
                sheet.cell_meta.insert(cell_key, old);
                add_children(sheet, parent1, parent2, formula, row, col);
            } else {
                sheet.cell_meta.remove(&cell_key);
            }

            return CommandStatus::CircularRef;
        } else {
            // If no cycle, update the cell history with the old value
            sheet
                .cell_history
                .entry(cell_key)
                .or_default()
                .push(old_value);
            sheet.set_last_edited(row, col);
        }
    }
    status
}

/// Sets a cell's value directly, bypassing formula evaluation.
///
/// # Arguments
///
/// * `sheet` - The mutable spreadsheet.
/// * `row` - The target row.
/// * `col` - The target column.
/// * `value` - The `CellValue` to set.
/// * `sleep_time` - Accumulates sleep time.
///
/// # Returns
///
/// * `CommandStatus::CmdOk` - On success.
/// * `CommandStatus::LockedCell` - If the cell is locked.
fn set_cell_to_value(
    sheet: &mut Spreadsheet,
    row: i16,
    col: i16,
    value: CellValue,
    sleep_time: &mut f64,
) -> CommandStatus {
    // Check if the cell is locked before setting the value
    if sheet.is_cell_locked(row, col) {
        return CommandStatus::LockedCell;
    }
    // Check if the value is a valid integer
    let cell_key = sheet.get_key(row, col);
    // remove all parents and set the value
    remove_all_parents(sheet, row, col);
    sheet.cell_meta.remove(&cell_key);
    *sheet.get_mut_cell(row, col) = value;
    toposort_reval_detect_cycle(sheet, row, col, sleep_time);
    sheet.set_last_edited(row, col);
    CommandStatus::CmdOk
}

/// Processes user commands, updating the spreadsheet accordingly.
///
/// Supports cell assignments, scrolling, locking, and more.
///
/// # Arguments
///
/// * `sheet` - The mutable spreadsheet.
/// * `trimmed` - The command string (trimmed).
/// * `sleep_time` - Accumulates sleep time.
///
/// # Returns
///
/// The status of command execution (e.g., `CmdOk`, `Unrecognized`)
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
            }
            b'q' => return CommandStatus::CmdOk, // Handle quit command if needed
            _ => {}
        }
    }

    // Use match for special commands for better branch prediction
    match trimmed {
        "disable_output" => {
            sheet.output_enabled = false;
            return CommandStatus::CmdOk;
        }
        "enable_output" => {
            sheet.output_enabled = true;
            return CommandStatus::CmdOk;
        }
        "last_edit" => {
            sheet.scroll_to_last_edited();
            return CommandStatus::CmdOk;
        }
        _ => {}
    }

    // Check for scroll_to command with byte-based comparison
    if trimmed.len() > 10
        && &trimmed.as_bytes()[..9] == b"scroll_to"
        && trimmed.as_bytes()[9] == b' '
    {
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
        let expr = trimmed[pos + 1..].trim();

        // Parse the cell reference with direct result handling
        return match parse_cell_reference(sheet, cell_ref) {
            Ok((row, col)) => {
                // All bounds checks in one condition
                set_cell_value(sheet, row, col, expr, sleep_time)
            }
            Err(status) => status,
        };
    }

    if trimmed.starts_with("lock_cell ") {
        let lock_target = trimmed.get(10..).unwrap_or("").trim();
        if lock_target.contains(':') {
            match parse_range(sheet, lock_target) {
                Ok(range) => {
                    sheet.lock_range(range);
                    return CommandStatus::CmdOk;
                }
                Err(_) => return CommandStatus::Unrecognized,
            }
        } else {
            match resolve_cell_reference(sheet, lock_target) {
                Ok((row, col)) => {
                    let range = Range {
                        start_row: row,
                        start_col: col,
                        end_row: row,
                        end_col: col,
                    };
                    sheet.lock_range(range);
                    return CommandStatus::CmdOk;
                }
                Err(status) => return status,
            }
        }
    }

    // Check for cell dependency visualization command
    if let Some(cell_ref) = trimmed.strip_prefix("visual ") {
        match parse_cell_reference(sheet, cell_ref) {
            Ok((row, col)) => {
                return sheet.visualize_cell_relationships(row, col);
            }
            Err(status) => {
                return status;
            }
        }
    }

    if trimmed.starts_with("unlock_cell ") {
        let unlock_target = trimmed.get(11..).unwrap_or("").trim();
        if unlock_target.contains(':') {
            match parse_range(sheet, unlock_target) {
                Ok(range) => {
                    sheet.unlock_range(range);
                    return CommandStatus::CmdOk;
                }
                Err(_) => return CommandStatus::Unrecognized,
            }
        } else {
            match resolve_cell_reference(sheet, unlock_target) {
                Ok((row, col)) => {
                    let range = Range {
                        start_row: row,
                        start_col: col,
                        end_row: row,
                        end_col: col,
                    };
                    sheet.unlock_range(range);
                    return CommandStatus::CmdOk;
                }
                Err(status) => return status,
            }
        }
    }

    if let Some(cell_ref) = trimmed.strip_prefix("is_locked ") {
        let cell_ref = cell_ref.trim();
        match resolve_cell_reference(sheet, cell_ref) {
            Ok((row, col)) => {
                if sheet.is_cell_locked(row, col) {
                    return CommandStatus::LockedCell;
                } else {
                    return CommandStatus::NotLockedCell;
                }
            }
            Err(status) => return status,
        }
    }

    if let Some(name_arg) = trimmed.strip_prefix("name ") {
        let parts: Vec<&str> = name_arg.split_whitespace().collect();
        if parts.len() == 2 {
            let target = parts[0];
            let name = parts[1];
            if let Ok(range) = parse_range(sheet, target) {
                sheet.named_ranges.insert(name.to_string(), range);
                return CommandStatus::CmdOk;
            } else if let Ok((row, col)) = parse_cell_reference(sheet, target) {
                let range = Range {
                    start_row: row,
                    start_col: col,
                    end_row: row,
                    end_col: col,
                };
                sheet.named_ranges.insert(name.to_string(), range);
                return CommandStatus::CmdOk;
            }
        }
        return CommandStatus::Unrecognized;
    }

    if let Some(stripped) = trimmed.strip_prefix("history ") {
        let cell_ref = stripped.trim();
        return match resolve_cell_reference(sheet, cell_ref) {
            Ok((row, col)) => {
                let cell_key = sheet.get_key(row, col);
                if let Some(history) = sheet.cell_history.get_mut(&cell_key) {
                    if let Some(prev_value) = history.pop() {
                        set_cell_to_value(sheet, row, col, prev_value, sleep_time)
                    } else {
                        CommandStatus::CmdOk
                    }
                } else {
                    CommandStatus::CmdOk
                }
            }
            Err(status) => status,
        };
    }

    if let Some(stripped) = trimmed.strip_prefix("formula ") {
        let cell_ref = stripped.trim();
        match resolve_cell_reference(sheet, cell_ref) {
            Ok((row, col)) => {
                let formula_str = get_formula_string(sheet, row, col);
                println!("{}", formula_str);
                return CommandStatus::CmdOk;
            }
            Err(status) => return status,
        }
    }

    // Check for highlight commands
    if let Some(cell_ref) = trimmed.strip_prefix("HLP ") {
        if let Ok((row, col)) = parse_cell_reference(sheet, cell_ref) {
            sheet.set_highlight(row, col, HighlightType::Parent);
            return CommandStatus::CmdOk;
        } else {
            return CommandStatus::InvalidCell;
        }
    }

    if let Some(cell_ref) = trimmed.strip_prefix("HLC ") {
        if let Ok((row, col)) = parse_cell_reference(sheet, cell_ref) {
            sheet.set_highlight(row, col, HighlightType::Child);
            return CommandStatus::CmdOk;
        } else {
            return CommandStatus::InvalidCell;
        }
    }

    if let Some(cell_ref) = trimmed.strip_prefix("HLPC ") {
        if let Ok((row, col)) = parse_cell_reference(sheet, cell_ref) {
            sheet.set_highlight(row, col, HighlightType::Both);
            return CommandStatus::CmdOk;
        } else {
            return CommandStatus::InvalidCell;
        }
    }

    if trimmed == "HLOFF" {
        sheet.disable_highlight();
        return CommandStatus::CmdOk;
    }

    // No recognized command
    CommandStatus::Unrecognized
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::CellValue;
    use crate::spreadsheet::{CommandStatus, Spreadsheet};

    fn create_test_spreadsheet(rows: i16, cols: i16) -> Spreadsheet {
        Spreadsheet::create(rows, cols).unwrap()
    }

    #[test]
    fn test_handle_sleep_with_reference() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(5);
        let mut sleep_time = 0.0;
        assert_eq!(
            handle_sleep(&mut sheet, 1, 1, "A1", &mut sleep_time),
            CommandStatus::CmdOk
        );
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(5));
        assert_eq!(sleep_time, 5.0);
        let meta = sheet.cell_meta.get(&sheet.get_key(1, 1)).unwrap();
        assert_eq!(meta.formula, 102);
        assert_eq!(meta.parent1, sheet.get_key(0, 0));
    }

    #[test]
    fn test_handle_sleep_with_literal() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let mut sleep_time = 0.0;
        assert_eq!(
            handle_sleep(&mut sheet, 1, 1, "3", &mut sleep_time),
            CommandStatus::CmdOk
        );
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(3));
        assert_eq!(sleep_time, 3.0);
        assert!(!sheet.cell_meta.contains_key(&sheet.get_key(1, 1)));
    }

    #[test]
    fn test_handle_sleep_invalid() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let mut sleep_time = 0.0;
        assert_eq!(
            handle_sleep(&mut sheet, 1, 1, "INVALID", &mut sleep_time),
            CommandStatus::Unrecognized
        );
    }

    #[test]
    fn test_handle_sleep_self_reference() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let mut sleep_time = 0.0;
        assert_eq!(
            handle_sleep(&mut sheet, 1, 1, "B2", &mut sleep_time),
            CommandStatus::CircularRef
        );
    }

    #[test]
    fn test_evaluate_arithmetic_literal() {
        let mut sheet = create_test_spreadsheet(5, 5);
        assert_eq!(
            evaluate_arithmetic(&mut sheet, 0, 0, "42"),
            CommandStatus::CmdOk
        );
        assert_eq!(*sheet.get_cell(0, 0), CellValue::Integer(42));
        assert!(!sheet.cell_meta.contains_key(&sheet.get_key(0, 0)));
    }

    #[test]
    fn test_evaluate_arithmetic_cell_ref() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(10);
        assert_eq!(
            evaluate_arithmetic(&mut sheet, 1, 1, "A1"),
            CommandStatus::CmdOk
        );
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(10));
        let meta = sheet.cell_meta.get(&sheet.get_key(1, 1)).unwrap();
        assert_eq!(meta.formula, 82);
        assert_eq!(meta.parent1, sheet.get_key(0, 0));
    }

    #[test]
    fn test_evaluate_arithmetic_binary_add() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(5);
        assert_eq!(
            evaluate_arithmetic(&mut sheet, 1, 1, "A1+3"),
            CommandStatus::CmdOk
        );
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(8));
        let meta = sheet.cell_meta.get(&sheet.get_key(1, 1)).unwrap();
        assert_eq!(meta.formula, 12);
        assert_eq!(meta.parent1, sheet.get_key(0, 0));
        assert_eq!(meta.parent2, 3);
    }

    #[test]
    fn test_evaluate_arithmetic_binary_div_zero() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(5);
        assert_eq!(
            evaluate_arithmetic(&mut sheet, 1, 1, "A1/0"),
            CommandStatus::CmdOk
        );
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Error);
    }

    #[test]
    fn test_evaluate_formula_sum() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(1);
        *sheet.get_mut_cell(0, 1) = CellValue::Integer(2);
        let mut sleep_time = 0.0;
        assert_eq!(
            evaluate_formula(&mut sheet, 1, 1, "SUM(A1:B1)", &mut sleep_time),
            CommandStatus::CmdOk
        );
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(3));
        let meta = sheet.cell_meta.get(&sheet.get_key(1, 1)).unwrap();
        assert_eq!(meta.formula, 5);
    }

    #[test]
    fn test_evaluate_formula_invalid() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let mut sleep_time = 0.0;
        assert_eq!(
            evaluate_formula(&mut sheet, 1, 1, "SUM(A1)", &mut sleep_time),
            CommandStatus::Unrecognized
        );
    }

    #[test]
    fn test_set_cell_value_with_cycle() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let mut sleep_time = 0.0;
        assert_eq!(
            set_cell_value(&mut sheet, 0, 0, "A1", &mut sleep_time),
            CommandStatus::CircularRef
        );
    }

    #[test]
    fn test_handle_command_scroll() {
        let mut sheet = create_test_spreadsheet(50, 50);
        let mut sleep_time = 0.0;
        assert_eq!(
            handle_command(&mut sheet, "s", &mut sleep_time),
            CommandStatus::CmdOk
        );
        assert_eq!(sheet.viewport_row, 10);
        assert_eq!(
            handle_command(&mut sheet, "d", &mut sleep_time),
            CommandStatus::CmdOk
        );
        assert_eq!(sheet.viewport_col, 10);
    }

    #[test]
    fn test_handle_command_output_toggle() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let mut sleep_time = 0.0;
        assert_eq!(
            handle_command(&mut sheet, "disable_output", &mut sleep_time),
            CommandStatus::CmdOk
        );
        assert!(!sheet.output_enabled);
        assert_eq!(
            handle_command(&mut sheet, "enable_output", &mut sleep_time),
            CommandStatus::CmdOk
        );
        assert!(sheet.output_enabled);
    }

    #[test]
    fn test_handle_command_visualize() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let mut sleep_time = 0.0;
        assert_eq!(
            handle_command(&mut sheet, "visual A1", &mut sleep_time),
            CommandStatus::CmdOk
        );
        assert_eq!(
            handle_command(&mut sheet, "visual Z9", &mut sleep_time),
            CommandStatus::Unrecognized
        );
    }

    #[test]
    fn test_handle_command_scroll_to() {
        let mut sheet = create_test_spreadsheet(50, 50);
        let mut sleep_time = 0.0;
        assert_eq!(
            handle_command(&mut sheet, "scroll_to B2", &mut sleep_time),
            CommandStatus::CmdOk
        );
        assert_eq!(sheet.viewport_row, 1);
        assert_eq!(sheet.viewport_col, 1);
    }

    #[test]
    fn test_handle_command_assignment() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let mut sleep_time = 0.0;
        assert_eq!(
            handle_command(&mut sheet, "A1=42", &mut sleep_time),
            CommandStatus::CmdOk
        );
        assert_eq!(*sheet.get_cell(0, 0), CellValue::Integer(42));
    }

    #[test]
    fn test_set_cell_value_circular_ref() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let mut sleep_time = 0.0;
        assert_eq!(
            set_cell_value(&mut sheet, 0, 0, "A1", &mut sleep_time),
            CommandStatus::CircularRef
        );
        assert_eq!(*sheet.get_cell(0, 0), CellValue::Integer(0));
    }

    #[test]
    fn test_handle_command_last_edit() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let mut sleep_time = 0.0;
        handle_command(&mut sheet, "B2=42", &mut sleep_time);
        assert_eq!(sheet.last_edited, Some((1, 1)));
        assert_eq!(
            handle_command(&mut sheet, "last_edit", &mut sleep_time),
            CommandStatus::CmdOk
        );
        assert_eq!(sheet.viewport_row, 1);
        assert_eq!(sheet.viewport_col, 1);
    }

    #[test]
    fn test_handle_command_lock_cell() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let mut sleep_time = 0.0;
        assert_eq!(
            handle_command(&mut sheet, "lock_cell B2", &mut sleep_time),
            CommandStatus::CmdOk
        );
        assert!(sheet.is_cell_locked(1, 1));
    }

    #[test]
    fn test_handle_command_history() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let mut sleep_time = 0.0;
        assert_eq!(
            handle_command(&mut sheet, "A2=2", &mut sleep_time),
            CommandStatus::CmdOk
        );
        assert_eq!(
            handle_command(&mut sheet, "A2=3", &mut sleep_time),
            CommandStatus::CmdOk
        );
        assert_eq!(
            handle_command(&mut sheet, "history A2", &mut sleep_time),
            CommandStatus::CmdOk
        );
        assert_eq!(*sheet.get_cell(1, 0), CellValue::Integer(2));
        assert_eq!(sheet.last_edited, Some((1, 0)));
    }

    #[test]
    fn test_evaluate_formula_max() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(5);
        *sheet.get_mut_cell(0, 1) = CellValue::Integer(3);
        let mut sleep_time = 0.0;
        assert_eq!(
            evaluate_formula(&mut sheet, 1, 1, "MAX(A1:B1)", &mut sleep_time),
            CommandStatus::CmdOk
        );
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(5));
    }

    #[test]
    fn test_resolve_cell_reference_named_range() {
        let mut sheet = create_test_spreadsheet(5, 5);
        sheet.named_ranges.insert(
            "test".to_string(),
            Range {
                start_row: 1,
                start_col: 1,
                end_row: 1,
                end_col: 1,
            },
        );
        assert_eq!(resolve_cell_reference(&sheet, "test"), Ok((1, 1)));
    }

    #[test]
    fn test_evaluate_arithmetic_both_cells() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(5);
        *sheet.get_mut_cell(0, 1) = CellValue::Integer(3);
        assert_eq!(
            evaluate_arithmetic(&mut sheet, 1, 1, "A1+B1"),
            CommandStatus::CmdOk
        );
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(8));
    }

    #[test]
    fn test_evaluate_formula_avg() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(2);
        *sheet.get_mut_cell(0, 1) = CellValue::Integer(4);
        let mut sleep_time = 0.0;
        assert_eq!(
            evaluate_formula(&mut sheet, 1, 1, "AVG(A1:B1)", &mut sleep_time),
            CommandStatus::CmdOk
        );
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(3));
    }

    #[test]
    fn test_evaluate_formula_stdev() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(2);
        *sheet.get_mut_cell(0, 1) = CellValue::Integer(4);
        let mut sleep_time = 0.0;
        assert_eq!(
            evaluate_formula(&mut sheet, 1, 1, "STDEV(A1:B1)", &mut sleep_time),
            CommandStatus::CmdOk
        );
    }

    #[test]
    fn test_set_cell_value_locked() {
        let mut sheet = create_test_spreadsheet(5, 5);
        sheet.lock_range(Range {
            start_row: 0,
            start_col: 0,
            end_row: 0,
            end_col: 0,
        });
        let mut sleep_time = 0.0;
        assert_eq!(
            set_cell_value(&mut sheet, 0, 0, "42", &mut sleep_time),
            CommandStatus::LockedCell
        );
    }

    #[test]
    fn test_handle_command_highlight() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let mut sleep_time = 0.0;
        assert_eq!(
            handle_command(&mut sheet, "HLP A1", &mut sleep_time),
            CommandStatus::CmdOk
        );
        assert_eq!(sheet.highlight_type, HighlightType::Parent);
        assert_eq!(
            handle_command(&mut sheet, "HLOFF", &mut sleep_time),
            CommandStatus::CmdOk
        );
        assert_eq!(sheet.highlight_type, HighlightType::None);
    }

    #[test]
    fn test_evaluate_arithmetic_invalid_operator_position() {
        let mut sheet = create_test_spreadsheet(5, 5);
        assert_eq!(
            evaluate_arithmetic(&mut sheet, 0, 0, "A1=+B1"),
            CommandStatus::Unrecognized
        );
    }

    #[test]
    fn test_evaluate_arithmetic_incomplete_expression() {
        let mut sheet = create_test_spreadsheet(5, 5);
        assert_eq!(
            evaluate_arithmetic(&mut sheet, 0, 0, "A1/"),
            CommandStatus::Unrecognized
        );
    }

    #[test]
    fn test_evaluate_arithmetic_multiple_operands() {
        let mut sheet = create_test_spreadsheet(5, 5);
        assert_eq!(
            evaluate_arithmetic(&mut sheet, 0, 0, "A1*B1*C1"),
            CommandStatus::Unrecognized
        );
    }

    #[test]
    fn test_evaluate_arithmetic_division_by_zero() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 1) = CellValue::Integer(0);
        assert_eq!(
            evaluate_arithmetic(&mut sheet, 1, 1, "A1/B1"),
            CommandStatus::CmdOk
        );
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Error);
    }

    #[test]
    fn test_set_cell_value_circular_ref_both_cells() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let mut sleep_time = 0.0;
        assert_eq!(
            set_cell_value(&mut sheet, 0, 0, "A1+B1", &mut sleep_time),
            CommandStatus::CircularRef
        );
        assert_eq!(*sheet.get_cell(0, 0), CellValue::Integer(0));
    }

    #[test]
    fn test_evaluate_formula_avg_single_cell() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(10);
        let mut sleep_time = 0.0;
        assert_eq!(
            evaluate_formula(&mut sheet, 1, 1, "AVG(A1:A1)", &mut sleep_time),
            CommandStatus::CmdOk
        );
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(10));
    }

    #[test]
    fn test_evaluate_formula_min_with_error() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(5);
        *sheet.get_mut_cell(0, 1) = CellValue::Error;
        let mut sleep_time = 0.0;
        assert_eq!(
            evaluate_formula(&mut sheet, 1, 1, "MIN(A1:B1)", &mut sleep_time),
            CommandStatus::CmdOk
        );
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Error);
    }

    #[test]
    fn test_evaluate_formula_max_negative_numbers() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(-5);
        *sheet.get_mut_cell(0, 1) = CellValue::Integer(-3);
        let mut sleep_time = 0.0;
        assert_eq!(
            evaluate_formula(&mut sheet, 1, 1, "MAX(A1:B1)", &mut sleep_time),
            CommandStatus::CmdOk
        );
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(-3));
    }

    #[test]
    fn test_evaluate_formula_stdev_large_range() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(1);
        *sheet.get_mut_cell(0, 1) = CellValue::Integer(3);
        *sheet.get_mut_cell(1, 0) = CellValue::Integer(5);
        let mut sleep_time = 0.0;
        assert_eq!(
            evaluate_formula(&mut sheet, 2, 2, "STDEV(A1:B2)", &mut sleep_time),
            CommandStatus::CmdOk
        );
    }

    #[test]
    fn test_evaluate_formula_with_named_range() {
        let mut sheet = create_test_spreadsheet(5, 5);
        sheet.named_ranges.insert(
            "data".to_string(),
            Range {
                start_row: 0,
                start_col: 0,
                end_row: 0,
                end_col: 0,
            },
        );
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(15);
        let mut sleep_time = 0.0;
        assert_eq!(
            evaluate_formula(&mut sheet, 1, 1, "SUM(data)", &mut sleep_time),
            CommandStatus::CmdOk
        );
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(15));
    }

    #[test]
    fn test_sleep_error() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Error;
        let mut sleep_time = 0.0;
        assert_eq!(
            handle_sleep(&mut sheet, 1, 1, "A1", &mut sleep_time),
            CommandStatus::CmdOk
        );
    }

    #[test]
    fn test_wrong_cell_reference() {
        let mut sheet = create_test_spreadsheet(5, 5);
        assert_eq!(evaluate_arithmetic(
            &mut sheet,
            0,
            0,
            "ZZZ999",),CommandStatus::Unrecognized);
    }

    #[test]
    fn left_and_right_val(){
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 1) = CellValue::Error;
        let mut sleep_time = 0.0;
        assert_eq!(
            evaluate_formula(&mut sheet, 1, 1, "4+B1", &mut sleep_time),
            CommandStatus::CmdOk
        );
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Error);
    }

    #[test]
    fn left_error_right_cell(){
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Error;
        *sheet.get_mut_cell(0, 1) = CellValue::Integer(1);
        let mut sleep_time = 0.0;
        assert_eq!(
            evaluate_formula(&mut sheet, 1, 1, "A1+B1", &mut sleep_time),
            CommandStatus::CmdOk
        );
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Error);
    }

    #[test]
    fn test_handle_command_history_empty() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let mut sleep_time = 0.0;
        assert_eq!(
            handle_command(&mut sheet, "history A1", &mut sleep_time),
            CommandStatus::CmdOk
        );
        assert_eq!(*sheet.get_cell(0, 0), CellValue::Integer(0));
    }

}
