pub enum CellValue {
    Integer(i32),
    Error,
}

pub fn parse_cell_reference(
    sheet: &Spreadsheet,
    cell_ref: &str,
) -> Result<(i16, i16), CommandStatus> {
    let cell_ref = cell_ref.as_bytes();
    if cell_ref.is_empty() {
        return Err(CommandStatus::CmdUnrecognized);
    }

    /// Find column/row split point in one pass
    let mut split_idx = 0;
    let mut col_length = 0;

    while split_idx < cell_ref.len() && cell_ref[split_idx] >= b'A' && cell_ref[split_idx] <= b'Z' {
        col_length += 1;
        if col_length > 3 {
            return Err(CommandStatus::CmdUnrecognized);
        }
        split_idx += 1;
    }

    // Verify we have columns and rows
    if col_length == 0 || split_idx == cell_ref.len() {
        return Err(CommandStatus::CmdUnrecognized);
    }

    // Verify remaining chars are digits
    for i in split_idx..cell_ref.len() {
        if !cell_ref[i].is_ascii_digit() {
            return Err(CommandStatus::CmdUnrecognized);
        }
    }

    // Get column reference as a string slice (no allocation)
    let col_name =
        std::str::from_utf8(&cell_ref[0..split_idx]).map_err(|_| CommandStatus::CmdUnrecognized)?;

    // Parse row directly from bytes (avoid string allocation)
    let mut row: i16 = 0;
    for &byte in &cell_ref[split_idx..] {
        row = row * 10 + (byte - b'0') as i16;
    }

    // Convert to 0-based
    let row = row - 1;

    // Convert column name to index
    let col = sheet.column_name_to_index(col_name);
    // Check row and column bounds
    if row < 0 || col < 0 || row > 998 || col > 18277 {
        return Err(CommandStatus::CmdUnrecognized);
    }
    Ok((row, col))
}
use crate::cell::{CellValue, parse_cell_reference};
use crate::formula::parse_range;
use crate::formula::{eval_avg, eval_max, eval_min, eval_variance, sum_value};
use crate::graph::{add_children, remove_all_parents};
use crate::reevaluate_topo::{sleep_fn, toposort_reval_detect_cycle};
use crate::spreadsheet::{CommandStatus, Spreadsheet};

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

        // Remove parents and update cell in one block
        remove_all_parents(sheet, row, col);

        // Set up the new cell metadata
        let meta = sheet.get_cell_meta(row, col);
        meta.parent1 = pkey;
        meta.parent2 = -1;
        meta.formula = 102; // Custom formula code for sleep

        // // Check for circular reference
        // if detect_cycle(sheet, pkey, -1, 102, cell_key) {
        //     if let Some(old) = old_meta {
        //         let (parent1, parent2, formula) = (old.parent1, old.parent2, old.formula);
        //         sheet.cell_meta.insert(cell_key, old);
        //         add_children(sheet, parent1, parent2, formula, row, col);
        //     } else {
        //         sheet.cell_meta.remove(&cell_key);
        //     }
        //     return CommandStatus::CmdCircularRef;
        // }

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
        match parse_cell_reference(sheet, expr) {
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
                *sheet.get_mut_cell(row, col) = match sheet.get_cell(target_row, target_col) {
                    CellValue::Integer(val) => CellValue::Integer(*val),
                    _ => CellValue::Error,
                };
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
    for i in 1..bytes.len() {
        match bytes[i] {
            b'+' | b'-' | b'*' | b'/' => {
                op = bytes[i];
                op_idx = i;
                break;
            }
            _ => {}
        }
    }

    if op_idx == 0 {
        return CommandStatus::CmdUnrecognized;
    }

    // Split into left and right parts
    let left = &expr[..op_idx];
    let right = &expr[op_idx + 1..];

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
        Some(b"SLE")
            if bytes.len() > 5
                && bytes[3] == b'E'
                && bytes[4] == b'P'
                && bytes.get(5) == Some(&b'(') =>
        {
            // Handle sleep function separately
            if !expr.ends_with(')') {
                return CommandStatus::CmdUnrecognized;
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
            return CommandStatus::CmdUnrecognized;
        }

        // Extract the range string without allocating extra memory
        let range_str: &str = &expr[prefix_len..expr.len() - 1];

        // Parse range and validate early to avoid unnecessary work
        let range = match parse_range(sheet, range_str) {
            Ok(r) => r,
            Err(status) => return status,
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

pub fn set_cell_value(
    sheet: &mut Spreadsheet,
    row: i16,
    col: i16,
    expr: &str,
    sleep_time: &mut f64,
) -> CommandStatus {
    let old_meta = sheet.cell_meta.get(&sheet.get_key(row, col)).cloned();
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
                sheet.cell_meta.insert(sheet.get_key(row, col), old);
                add_children(sheet, parent1, parent2, formula, row, col);
            } else {
                sheet.cell_meta.remove(&sheet.get_key(row, col));
            }

            return CommandStatus::CmdCircularRef;
        }
    }
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
        _ => {}
    }

    // Check for cell dependency visualization command
    if trimmed.starts_with("visualize ") {
        let cell_ref = &trimmed[10..]; // Skip "visualize " prefix
        match parse_cell_reference(sheet, cell_ref) {
            Ok((row, col)) => {
                return sheet.visualize_cell_relationships(row, col);
            }
            Err(status) => {
                return status;
            }
        }
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
    // No recognized command
    CommandStatus::CmdUnrecognized
}
use crate::cell::{CellValue, parse_cell_reference};
use crate::spreadsheet::{CommandStatus, Spreadsheet};

#[derive(Debug, PartialEq)]
pub struct Range {
    pub start_row: i16,
    pub start_col: i16,
    pub end_row: i16,
    pub end_col: i16,
}

// Optimize the sum_value function for large ranges
pub fn sum_value(
    sheet: &mut Spreadsheet,
    row: i16,
    col: i16,
    parent1: i32,
    parent2: i32,
) -> CommandStatus {
    let mut sum = 0;
    let (start_row, start_col) = sheet.get_row_col(parent1);
    let (end_row, end_col) = sheet.get_row_col(parent2);
    // For smaller ranges, use the original approach
    for i in start_row..=end_row {
        for j in start_col..=end_col {
            let ref_cell_value = sheet.get_cell(i, j);
            if let CellValue::Integer(value) = ref_cell_value {
                sum += value;
            } else {
                *sheet.get_mut_cell(row, col) = CellValue::Error;
                return CommandStatus::CmdOk;
            }
        }
    }

    // Now set the result
    *sheet.get_mut_cell(row, col) = CellValue::Integer(sum);
    CommandStatus::CmdOk
}

// Add variance evaluation function
pub fn eval_variance(
    sheet: &mut Spreadsheet,
    row: i16,
    col: i16,
    parent1: i32,
    parent2: i32,
) -> CommandStatus {
    let (start_row, start_col) = sheet.get_row_col(parent1);
    let (end_row, end_col) = sheet.get_row_col(parent2);
    let count = ((end_row - start_row + 1) as i32) * ((end_col - start_col + 1) as i32);
    sum_value(sheet, row, col, parent1, parent2);
    // Check if sum_value was successful
    let cell_value = sheet.get_mut_cell(row, col);
    let mean_value;
    if let CellValue::Integer(value) = cell_value {
        let val = *value / count;
        *cell_value = CellValue::Integer(val);
        mean_value = val as f64;
    } else {
        return CommandStatus::CmdOk;
    }

    let mut variance = 0.0;

    for i in start_row..=end_row {
        for j in start_col..=end_col {
            if let CellValue::Integer(value) = *sheet.get_cell(i, j) {
                variance += ((value as f64) - (mean_value)).powi(2);
            }
        }
    }

    variance /= count as f64;
    let std_dev = (variance.sqrt() + 0.5) as i32;
    *sheet.get_mut_cell(row, col) = CellValue::Integer(std_dev);
    CommandStatus::CmdOk
}

pub fn eval_min(
    sheet: &mut Spreadsheet,
    row: i16,
    col: i16,
    parent1: i32,
    parent2: i32,
) -> CommandStatus {
    let mut min_value = i32::MAX;
    let (start_row, start_col) = sheet.get_row_col(parent1);
    let (end_row, end_col) = sheet.get_row_col(parent2);
    // First collect all values (immutable borrows)
    for r in start_row..=end_row {
        for c in start_col..=end_col {
            if let CellValue::Integer(value) = sheet.get_cell(r, c) {
                min_value = std::cmp::min(min_value, *value);
            } else {
                *sheet.get_mut_cell(row, col) = CellValue::Error;
                return CommandStatus::CmdOk;
            }
        }
    }

    // Now get the mutable cell and set its value (mutable borrow)
    *sheet.get_mut_cell(row, col) = CellValue::Integer(min_value);
    CommandStatus::CmdOk
}

// Fix eval_max implementation
pub fn eval_max(
    sheet: &mut Spreadsheet,
    row: i16,
    col: i16,
    parent1: i32,
    parent2: i32,
) -> CommandStatus {
    let mut max_value = i32::MIN;
    let (start_row, start_col) = sheet.get_row_col(parent1);
    let (end_row, end_col) = sheet.get_row_col(parent2);
    // First collect all values (immutable borrows)
    for r in start_row..=end_row {
        for c in start_col..=end_col {
            if let CellValue::Integer(value) = sheet.get_cell(r, c) {
                max_value = std::cmp::max(max_value, *value);
            } else {
                *sheet.get_mut_cell(row, col) = CellValue::Error;
                return CommandStatus::CmdOk;
            }
        }
    }

    // Now get the mutable cell and set its value (mutable borrow)
    *sheet.get_mut_cell(row, col) = CellValue::Integer(max_value);
    CommandStatus::CmdOk
}

pub fn eval_avg(
    sheet: &mut Spreadsheet,
    row: i16,
    col: i16,
    parent1: i32,
    parent2: i32,
) -> CommandStatus {
    let (start_row, start_col) = sheet.get_row_col(parent1);
    let (end_row, end_col) = sheet.get_row_col(parent2);
    let count = ((end_row - start_row + 1) as i32) * ((end_col - start_col + 1) as i32);
    match sum_value(sheet, row, col, parent1, parent2) {
        CommandStatus::CmdOk => {
            let cell_value = sheet.get_mut_cell(row, col);
            if let CellValue::Integer(value) = cell_value {
                *cell_value = CellValue::Integer(*value / count);
            }
        }
        _ => return CommandStatus::CmdOk,
    }
    CommandStatus::CmdOk
}

// Keep the Range struct and parse_range function
pub fn parse_range(spreadsheet: &Spreadsheet, range_str: &str) -> Result<Range, CommandStatus> {
    // Check for minimum valid range pattern length (like "A1:A1")
    if range_str.len() < 3 {
        return Err(CommandStatus::CmdUnrecognized);
    }

    // Find the colon index using bytes to avoid UTF-8 decoding
    let bytes = range_str.as_bytes();
    let mut colon_index = 0;

    for (i, &b) in bytes.iter().enumerate() {
        if b == b':' {
            colon_index = i;
            break;
        }
    }

    // Validate colon position (must exist and have chars on both sides)
    if colon_index == 0 || colon_index + 1 >= range_str.len() {
        return Err(CommandStatus::CmdUnrecognized);
    }

    // Avoid creating new strings by using slices
    let start_cell = &range_str[..colon_index];
    let end_cell = &range_str[colon_index + 1..];

    // Parse cell references and validate them in one step
    let (start_row, start_col) = parse_cell_reference(spreadsheet, start_cell)?;
    let (end_row, end_col) = parse_cell_reference(spreadsheet, end_cell)?;

    // Ensure coordinates are valid and range is properly ordered
    if start_row < 0
        || start_col < 0
        || end_row < 0
        || end_col < 0
        || start_row > end_row
        || start_col > end_col
    {
        return Err(CommandStatus::CmdUnrecognized);
    }

    // Construct the Range directly
    Ok(Range {
        start_row,
        start_col,
        end_row,
        end_col,
    })
}
use crate::cell::{CellValue, parse_cell_reference};
use crate::visualize_cells;
use std::cmp::min;
use std::collections::HashMap;
use std::collections::HashSet;

// Constants
const MAX_ROWS: i16 = 999; // Maximum number of rows in the spreadsheet   
const MAX_COLS: i16 = 18278; // Maximum number of columns in the spreadsheet

// Structure to represent a range-based child relationship
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RangeChild {
    pub start_key: i32, // Range start cell key
    pub end_key: i32,   // Range end cell key
    pub child_key: i32, // Child cell key
}

#[derive(Debug, PartialEq)]
pub enum CommandStatus {
    CmdOk,
    CmdUnrecognized,
    CmdCircularRef,
    CmdInvalidCell,
}

// Modified CellMeta to remove children (they're now stored separately)
#[derive(Debug, Clone)]
pub struct CellMeta {
    pub formula: i16,
    pub parent1: i32,
    pub parent2: i32,
}

impl CellMeta {
    pub fn new() -> Self {
        CellMeta {
            formula: -1,
            parent1: -1,
            parent2: -1,
        }
    }
}

// Spreadsheet structure with HashMap of boxed HashSets for children
pub struct Spreadsheet {
    pub grid: Vec<CellValue>, // Vector of CellValues (contiguous in memory)
    pub children: HashMap<i32, Box<HashSet<i32>>>, // Map from cell key to boxed HashSet of children
    pub range_children: Vec<RangeChild>, // Vector of range-based child relationships
    pub cell_meta: HashMap<i32, CellMeta>, // Map from cell key to metadata
    pub rows: i16,
    pub cols: i16,
    pub viewport_row: i16,
    pub viewport_col: i16,
    pub output_enabled: bool,
}

impl Spreadsheet {
    // Create a new spreadsheet with specified dimensions
    pub fn create(rows: i16, cols: i16) -> Option<Spreadsheet> {
        if rows < 1 || rows > MAX_ROWS || cols < 1 || cols > MAX_COLS {
            eprintln!("Invalid spreadsheet dimensions");
            return None;
        }

        // Create empty cells - initialize with Integer(0)
        let total = rows as usize * cols as usize;
        let grid = vec![CellValue::Integer(0); total];

        Some(Spreadsheet {
            grid,
            children: HashMap::new(),
            range_children: Vec::with_capacity(32), // Preallocate with initial size
            cell_meta: HashMap::new(),
            rows,
            cols,
            viewport_row: 0,
            viewport_col: 0,
            output_enabled: true,
        })
    }

    // Helper to get cell key from coordinates
    pub fn get_key(&self, row: i16, col: i16) -> i32 {
        (row as i32 * self.cols as i32 + col as i32) as i32
    }

    // Helper to get coordinates from cell key
    pub fn get_row_col(&self, key: i32) -> (i16, i16) {
        let row = (key / (self.cols as i32)) as i16;
        let col = (key % (self.cols as i32)) as i16;
        (row, col)
    }

    // Helper to get index from row and column
    pub fn get_index(&self, row: i16, col: i16) -> usize {
        (row as usize) * (self.cols as usize) + (col as usize)
    }

    // Get cell metadata, creating it if it doesn't exist
    pub fn get_cell_meta(&mut self, row: i16, col: i16) -> &mut CellMeta {
        let key = self.get_key(row, col);
        self.cell_meta.entry(key).or_insert_with(CellMeta::new)
    }

    pub fn get_column_name(&self, mut col: i16) -> String {
        // Pre-calculate the length needed for the string
        let mut temp_col = col + 1; // Convert from 0-based to 1-based
        let mut len = 0;
        while temp_col > 0 {
            len += 1;
            temp_col = (temp_col - 1) / 26;
        }

        // Add column letters directly in reverse order
        col += 1; // Convert from 0-based to 1-based

        // Handle special case for col = 0
        if col == 0 {
            return "A".to_string();
        }

        // Create a buffer of bytes to avoid repeated string operations
        let mut buffer = vec![0; len];
        let mut i = len;

        while col > 0 {
            i -= 1;
            buffer[i] = b'A' + ((col - 1) % 26) as u8;
            col = (col - 1) / 26;
        }

        // Convert the byte buffer to a string in one operation
        unsafe {
            // This is safe because we know our bytes are valid ASCII from b'A' to b'Z'
            String::from_utf8_unchecked(buffer)
        }
    }

    pub fn column_name_to_index(&self, name: &str) -> i16 {
        let bytes = name.as_bytes();
        let mut index: i16 = 0;
        for &b in bytes {
            index = index * 26 + ((b - b'A') as i16 + 1);
        }
        index - 1 // Convert from 1-based to 0-based
    }

    pub fn get_cell(&self, row: i16, col: i16) -> &CellValue {
        let index = self.get_index(row, col);
        &self.grid[index]
    }

    pub fn get_key_cell(&self, cell_key: i32) -> &CellValue {
        &self.grid[cell_key as usize]
    }

    pub fn get_mut_cell(&mut self, row: i16, col: i16) -> &mut CellValue {
        let index = self.get_index(row, col);
        &mut self.grid[index]
    }

    // Add a range-based child relationship
    pub fn add_range_child(&mut self, start_key: i32, end_key: i32, child_key: i32) {
        self.range_children.push(RangeChild {
            start_key,
            end_key,
            child_key,
        });
    }

    // Remove range-based child relationships for a given child
    pub fn remove_range_child(&mut self, child_key: i32) {
        self.range_children.retain(|rc| rc.child_key != child_key);
    }

    // Check if a cell is within a range
    pub fn is_cell_in_range(&self, cell_key: i32, start_key: i32, end_key: i32) -> bool {
        let (cell_row, cell_col) = self.get_row_col(cell_key);
        let (start_row, start_col) = self.get_row_col(start_key);
        let (end_row, end_col) = self.get_row_col(end_key);

        cell_row >= start_row && cell_row <= end_row && cell_col >= start_col && cell_col <= end_col
    }

    // Add a child to a cell's dependents (modified for HashMap of boxed HashSets)
    pub fn add_child(&mut self, parent_key: &i32, child_key: &i32) {
        self.children
            .entry(*parent_key)
            .or_insert_with(|| Box::new(HashSet::with_capacity(5)))
            .insert(*child_key);
    }

    // Remove a child from a cell's dependents (modified for HashMap of boxed HashSets)
    pub fn remove_child(&mut self, parent_key: i32, child_key: i32) {
        if let Some(children) = self.children.get_mut(&parent_key) {
            children.remove(&child_key);

            // If the hashset is now empty, remove it from the HashMap to save memory
            if children.is_empty() {
                self.children.remove(&parent_key);
            }
        }
    }

    // Get children for a cell (immutable) (modified for HashMap of boxed HashSets)
    pub fn get_cell_children(&self, key: i32) -> Option<&HashSet<i32>> {
        self.children.get(&key).map(|boxed_set| boxed_set.as_ref())
    }

    pub fn print_spreadsheet(&self) {
        if !self.output_enabled {
            return;
        }

        let start_row = self.viewport_row;
        let start_col = self.viewport_col;
        let display_row = min(self.rows - start_row, 10); // Display only a portion of the spreadsheet
        let display_col = min(self.cols - start_col, 10);

        // Print column headers
        print!("     ");
        for i in 0..display_col {
            print!("{:<8} ", self.get_column_name(start_col + i));
        }
        println!();

        // Print rows with data
        for i in 0..display_row {
            print!("{:<4} ", start_row + i + 1); // Show 1-based row numbers
            for j in 0..display_col {
                let cell_value = self.get_cell(start_row + i, start_col + j);
                match cell_value {
                    CellValue::Integer(value) => print!("{:<8} ", value),
                    CellValue::Error => print!("{:<8} ", "ERR"),
                }
            }
            println!();
        }
    }

    pub fn scroll_to_cell(&mut self, cell: &str) -> CommandStatus {
        match parse_cell_reference(self, cell) {
            Ok((row, col)) => {
                if row >= 0 && row < self.rows && col >= 0 && col < self.cols {
                    self.viewport_row = row;
                    self.viewport_col = col;
                    return CommandStatus::CmdOk;
                } else {
                    return CommandStatus::CmdInvalidCell;
                }
            }
            Err(_) => {
                return CommandStatus::CmdUnrecognized;
            }
        }
    }

    pub fn scroll_viewport(&mut self, direction: char) {
        const VIEWPORT_SIZE: i16 = 10;
        match direction {
            'w' => {
                self.viewport_row = if self.viewport_row > 10 {
                    self.viewport_row - 10
                } else {
                    0
                };
            }
            's' => {
                if self.viewport_row + VIEWPORT_SIZE < self.rows {
                    self.viewport_row += 10;
                } else {
                    self.viewport_row = self.rows - VIEWPORT_SIZE;
                }
            }
            'a' => {
                self.viewport_col = if self.viewport_col > 10 {
                    self.viewport_col - 10
                } else {
                    0
                };
            }

            'd' => {
                if self.viewport_col + VIEWPORT_SIZE < self.cols {
                    self.viewport_col += 10;
                } else {
                    self.viewport_col = self.cols - VIEWPORT_SIZE;
                }
            }
            _ => {} // Invalid direction, do nothing
        }
    }

    pub fn visualize_cell_relationships(&self, row: i16, col: i16) -> CommandStatus {
        // Check if the cell is valid
        visualize_cells::visualize_cell_relationships(self, row, col)
    }
}
mod cell;
mod evaluator;
mod formula;
mod graph;
mod reevaluate_topo;
mod spreadsheet;
mod vim_mode;
mod visualize_cells;
use std::env;
use std::io::{self, Write};
use std::process;
use std::thread::sleep;
use std::time::{Duration, Instant};
// use sys_info;  // Add the system information library

use evaluator::handle_command;
use spreadsheet::CommandStatus;
use spreadsheet::Spreadsheet;
const DEFAULT_FILENAME: &str = "untitled.sheet";

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

    let mut sleep_time = 0.0; // Initialize sleep time
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
        let filename = Some(DEFAULT_FILENAME.to_string());
        vim_mode::run_editor(&mut sheet, filename);
    } else {
        let mut command_time = start.elapsed().as_secs_f64();
        let mut last_time = command_time; // Update last_time with the command time

        let mut last_status = "ok"; // Placeholder for last status
        let mut input = String::with_capacity(128);

        // Main loop for command input
        loop {
            sheet.print_spreadsheet();
            print!("[{:.1}s ({}) > ", last_time, last_status);
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
}
