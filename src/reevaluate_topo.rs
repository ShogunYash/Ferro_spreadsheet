use crate::cell::CellValue;
use crate::formula::{eval_avg, eval_max, eval_min, eval_variance, sum_value};
use crate::spreadsheet::Spreadsheet;
use std::collections::HashSet;

/// Sets a cell value and accumulates sleep time if positive.
///
/// # Arguments
///
/// * `sheet` - The mutable spreadsheet.
/// * `row` - The target row.
/// * `col` - The target column.
/// * `value` - The value to set.
/// * `sleep_val` - Accumulates sleep time.
pub fn sleep_fn(sheet: &mut Spreadsheet, row: i16, col: i16, value: i32, sleep_val: &mut f64) {
    *sheet.get_mut_cell(row, col) = CellValue::Integer(value);
    if value < 0 {
        return;
    }
    *sleep_val += value as f64;
}

/// Reevaluates a cell’s formula based on its metadata.
///
/// # Arguments
///
/// * `sheet` - The mutable spreadsheet.
/// * `row` - The target row.
/// * `col` - The target column.
/// * `sleep_val` - Accumulates sleep time.
pub fn reevaluate_formula(sheet: &mut Spreadsheet, row: i16, col: i16, sleep_val: &mut f64) {
    #[cfg(feature = "extensions")]
    if sheet.is_cell_locked(row, col) {
        return;
    }
    let cell_meta = sheet.get_cell_meta(row, col);
    let rem = cell_meta.formula % 10;
    let msb = cell_meta.formula / 10;
    let parent1 = cell_meta.parent1;
    let parent2 = cell_meta.parent2;

    match rem {
        0 => {
            let par1 = sheet.get_key_cell(parent1);
            let par2 = sheet.get_key_cell(parent2);
            if CellValue::Error == *par1 || CellValue::Error == *par2 {
                *sheet.get_mut_cell(row, col) = CellValue::Error;
                return;
            }
            if let CellValue::Integer(p1_value) = par1 {
                if let CellValue::Integer(p2_value) = par2 {
                    match msb {
                        1 => {
                            *sheet.get_mut_cell(row, col) = CellValue::Integer(p1_value + p2_value);
                        }
                        2 => {
                            *sheet.get_mut_cell(row, col) = CellValue::Integer(p1_value - p2_value);
                        }
                        4 => {
                            *sheet.get_mut_cell(row, col) = CellValue::Integer(p1_value * p2_value);
                        }
                        _ => {
                            if *p2_value == 0 {
                                *sheet.get_mut_cell(row, col) = CellValue::Error;
                            } else {
                                *sheet.get_mut_cell(row, col) =
                                    CellValue::Integer(p1_value / p2_value);
                            }
                        }
                    }
                }
            }
        }
        2 => {
            let par1 = sheet.get_key_cell(parent1);
            if CellValue::Error == *par1 {
                *sheet.get_mut_cell(row, col) = CellValue::Error;
                return;
            }
            if let CellValue::Integer(p1_value) = par1 {
                match msb {
                    1 => {
                        *sheet.get_mut_cell(row, col) = CellValue::Integer(p1_value + parent2);
                    }
                    2 => {
                        *sheet.get_mut_cell(row, col) = CellValue::Integer(p1_value - parent2);
                    }
                    4 => {
                        *sheet.get_mut_cell(row, col) = CellValue::Integer(p1_value * parent2);
                    }
                    3 => {
                        if parent2 == 0 {
                            *sheet.get_mut_cell(row, col) = CellValue::Error;
                        } else {
                            *sheet.get_mut_cell(row, col) = CellValue::Integer(p1_value / parent2);
                        }
                    }
                    8 => {
                        *sheet.get_mut_cell(row, col) = CellValue::Integer(*p1_value);
                    }
                    _ => {
                        sleep_fn(sheet, row, col, *p1_value, sleep_val);
                    }
                }
            }
        }
        3 => {
            let par2 = sheet.get_key_cell(parent2);
            if CellValue::Error == *par2 {
                *sheet.get_mut_cell(row, col) = CellValue::Error;
                return;
            }
            if let CellValue::Integer(p2_value) = par2 {
                match msb {
                    1 => {
                        *sheet.get_mut_cell(row, col) = CellValue::Integer(parent1 + p2_value);
                    }
                    2 => {
                        *sheet.get_mut_cell(row, col) = CellValue::Integer(parent1 - p2_value);
                    }
                    4 => {
                        *sheet.get_mut_cell(row, col) = CellValue::Integer(parent1 * p2_value);
                    }
                    _ => {
                        if *p2_value == 0 {
                            *sheet.get_mut_cell(row, col) = CellValue::Error;
                        } else {
                            *sheet.get_mut_cell(row, col) = CellValue::Integer(parent1 / p2_value);
                        }
                    }
                }
            }
        }
        5 => {
            sum_value(sheet, row, col, parent1, parent2);
        }
        6 => {
            eval_avg(sheet, row, col, parent1, parent2);
        }
        7 => {
            eval_min(sheet, row, col, parent1, parent2);
        }
        8 => {
            eval_max(sheet, row, col, parent1, parent2);
        }
        _ => {
            eval_variance(sheet, row, col, parent1, parent2);
        }
    }
}

/// Performs a topological sort to reevaluate dependent cells and detect cycles.
///
/// Uses depth-first search to order cells and identify circular dependencies.
///
/// # Arguments
///
/// * `sheet` - The mutable spreadsheet.
/// * `row` - The updated cell’s row.
/// * `col` - The updated cell’s column.
/// * `sleep_val` - Accumulates sleep time.
///
/// # Returns
///
/// * `true` - If a cycle is detected.
/// * `false` - If no cycle is detected.
pub fn toposort_reval_detect_cycle(
    sheet: &mut Spreadsheet,
    row: i16,
    col: i16,
    sleep_val: &mut f64,
) -> bool {
    let cell_key = sheet.get_key(row, col);
    // These collections will be used for the topological sort and cycle detection
    let mut fully_visited: HashSet<i32> = HashSet::new();
    let mut result: Vec<i32> = Vec::new();
    let mut dfs_stack: Vec<(i32, bool)> = Vec::new();
    let mut in_current_path: HashSet<i32> = HashSet::new();

    // Helper to push all dependents (both direct and range-based) for a given cell key
    fn push_dependents(
        cell_key: i32,
        sheet: &Spreadsheet,
        stack: &mut Vec<(i32, bool)>,
        fully_visited: &HashSet<i32>,
    ) {
        // Direct children from standard dependencies
        if let Some(children) = sheet.get_cell_children(cell_key) {
            for child in children {
                if !fully_visited.contains(child) {
                    stack.push((*child, false));
                }
            }
        }

        for range_child in &sheet.range_children {
            if !fully_visited.contains(&range_child.child_key)
                && sheet.is_cell_in_range(cell_key, range_child.start_key, range_child.end_key)
            {
                stack.push((range_child.child_key, false));
            }
        }
    }

    // Start from all direct children and range-based children of the updated cell
    push_dependents(cell_key, sheet, &mut dfs_stack, &fully_visited);

    while let Some((current, expanded)) = dfs_stack.pop() {
        if expanded {
            // If we're processing a fully expanded node:
            in_current_path.remove(&current);
            if !result.contains(&current) {
                result.push(current);
            }
            fully_visited.insert(current);
        } else {
            // If we haven't expanded this node yet:
            if in_current_path.contains(&current) {
                // Cycle detected
                // Debugging output
                // println!("Cycle detected at cell: {}", current);
                // Uncomment the following line to see the cycle path
                // println!("Cycle path: {:?}", in_current_path);
                return true;
            }

            // Add back the current node as expanded
            dfs_stack.push((current, true));
            in_current_path.insert(current);

            // Process all its dependents (both direct and range-based)
            push_dependents(current, sheet, &mut dfs_stack, &fully_visited);
        }
    }

    // Reverse the result to get the correct topological order
    result.reverse();

    // Now reevaluate all cells in the topological order
    for key in result {
        if key >= 0 {
            let (row, col) = sheet.get_row_col(key);
            reevaluate_formula(sheet, row, col, sleep_val);
        }
    }

    false // No cycle detected
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::CellValue;
    use crate::evaluator::set_cell_value;
    use crate::spreadsheet::{CommandStatus, Spreadsheet};

    fn create_test_spreadsheet(rows: i16, cols: i16) -> Spreadsheet {
        Spreadsheet::create(rows, cols).unwrap()
    }

    #[test]
    fn test_sleep_fn_positive() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let mut sleep_time = 0.0;
        sleep_fn(&mut sheet, 0, 0, 5, &mut sleep_time);
        assert_eq!(*sheet.get_cell(0, 0), CellValue::Integer(5));
        assert_eq!(sleep_time, 5.0);
    }

    #[test]
    fn test_sleep_fn_negative() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let mut sleep_time = 0.0;
        sleep_fn(&mut sheet, 0, 0, -5, &mut sleep_time);
        assert_eq!(*sheet.get_cell(0, 0), CellValue::Integer(-5));
        assert_eq!(sleep_time, 0.0);
    }

    #[test]
    fn test_reevaluate_formula_arithmetic() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(3);
        *sheet.get_mut_cell(0, 1) = CellValue::Integer(2);
        {
            let key1 = sheet.get_key(0, 0);
            let key2 = sheet.get_key(0, 1);
            let meta = sheet.get_cell_meta(1, 1);
            meta.parent1 = key1;
            meta.parent2 = key2;
            meta.formula = 10; // Addition
        }
        let mut sleep_time = 0.0;
        reevaluate_formula(&mut sheet, 1, 1, &mut sleep_time);
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(5));
    }
    #[test]
    fn test_reevaluate_formula_arithmetic2() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(3);
        *sheet.get_mut_cell(0, 1) = CellValue::Integer(2);
        {
            let key1 = sheet.get_key(0, 0);
            let key2 = sheet.get_key(0, 1);
            let meta = sheet.get_cell_meta(1, 1);
            meta.parent1 = key1;
            meta.parent2 = key2;
            meta.formula = 20;
        }
        let mut sleep_time = 0.0;
        reevaluate_formula(&mut sheet, 1, 1, &mut sleep_time);
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(1));
    }
    #[test]
    fn test_reevaluate_formula_arithmetic3() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(3);
        *sheet.get_mut_cell(0, 1) = CellValue::Integer(3);
        {
            let key1 = sheet.get_key(0, 0);
            let key2 = sheet.get_key(0, 1);
            let meta = sheet.get_cell_meta(1, 1);
            meta.parent1 = key1;
            meta.parent2 = key2;
            meta.formula = 30;
        }
        let mut sleep_time = 0.0;
        reevaluate_formula(&mut sheet, 1, 1, &mut sleep_time);
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(1));
    }

    #[test]
    fn test_reevaluate_formula_arithmetic4() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(3);
        *sheet.get_mut_cell(0, 1) = CellValue::Integer(2);
        {
            let key1 = sheet.get_key(0, 0);
            let key2 = sheet.get_key(0, 1);
            let meta = sheet.get_cell_meta(1, 1);
            meta.parent1 = key1;
            meta.parent2 = key2;
            meta.formula = 40;
        }
        let mut sleep_time = 0.0;
        reevaluate_formula(&mut sheet, 1, 1, &mut sleep_time);
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(6));
    }

    #[test]
    fn test_reevaluate_formula_with_constant() {
        let mut sheet = create_test_spreadsheet(5, 5);
        // Set up cell value we'll reference
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(3);

        {
            let key1 = sheet.get_key(0, 0);
            let meta = sheet.get_cell_meta(1, 1);
            meta.parent1 = key1; // First parent is a cell reference
            meta.parent2 = 7; // -1 indicates no cell reference (constant)
            meta.formula = 12; // Addition
            // The constant value to add
        }

        let mut sleep_time = 0.0;
        reevaluate_formula(&mut sheet, 1, 1, &mut sleep_time);
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(10)); // 3 + 7 = 10
    }
    #[test]
    fn test_reevaluate_formula_with_constant2() {
        let mut sheet = create_test_spreadsheet(5, 5);
        // Set up cell value we'll reference
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(3);

        {
            let key1 = sheet.get_key(0, 0);
            let meta = sheet.get_cell_meta(1, 1);
            meta.parent1 = key1; // First parent is a cell reference
            meta.parent2 = 3; // -1 indicates no cell reference (constant)
            meta.formula = 22; // Addition
            // The constant value to add
        }

        let mut sleep_time = 0.0;
        reevaluate_formula(&mut sheet, 1, 1, &mut sleep_time);
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(0)); // 3 + 7 = 10
    }
    #[test]
    fn test_reevaluate_formula_with_constant3() {
        let mut sheet = create_test_spreadsheet(5, 5);
        // Set up cell value we'll reference
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(3);

        {
            let key1 = sheet.get_key(0, 0);
            let meta = sheet.get_cell_meta(1, 1);
            meta.parent1 = key1; // First parent is a cell reference
            meta.parent2 = 3; // -1 indicates no cell reference (constant)
            meta.formula = 32; // Addition
            // The constant value to add
        }

        let mut sleep_time = 0.0;
        reevaluate_formula(&mut sheet, 1, 1, &mut sleep_time);
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(1)); // 3 + 7 = 10
    }
    #[test]
    fn test_reevaluate_formula_with_constant4() {
        let mut sheet = create_test_spreadsheet(5, 5);
        // Set up cell value we'll reference
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(3);

        {
            let key1 = sheet.get_key(0, 0);
            let meta = sheet.get_cell_meta(1, 1);
            meta.parent1 = key1; // First parent is a cell reference
            meta.parent2 = 3; // -1 indicates no cell reference (constant)
            meta.formula = 42; // Addition
            // The constant value to add
        }

        let mut sleep_time = 0.0;
        reevaluate_formula(&mut sheet, 1, 1, &mut sleep_time);
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(9)); // 3 + 7 = 10
    }
    #[test]
    fn test_reevaluate_formula_with_constant4_error() {
        let mut sheet = create_test_spreadsheet(5, 5);
        // Set up cell value we'll reference
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(3);

        {
            let key1 = sheet.get_key(0, 0);
            let meta = sheet.get_cell_meta(1, 1);
            meta.parent1 = key1; // First parent is a cell reference
            meta.parent2 = 0; // -1 indicates no cell reference (constant)
            meta.formula = 32; // Addition
            // The constant value to add
        }

        let mut sleep_time = 0.0;
        reevaluate_formula(&mut sheet, 1, 1, &mut sleep_time);
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Error); // 3 + 7 = 10
    }
    #[test]
    fn test_reevaluate_formula_with_constant5() {
        let mut sheet = create_test_spreadsheet(5, 5);
        // Set up cell value we'll reference
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(3);

        {
            let key1 = sheet.get_key(0, 0);
            let meta = sheet.get_cell_meta(1, 1);
            meta.parent1 = key1; // First parent is a cell reference
            meta.parent2 = 3; // -1 indicates no cell reference (constant)
            meta.formula = 42; // Addition
            // The constant value to add
        }

        let mut sleep_time = 0.0;
        reevaluate_formula(&mut sheet, 1, 1, &mut sleep_time);
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(9)); // 3 + 7 = 10
    }

    #[test]
    fn test_reevaluate_formula_remainder1_constant() {
        let mut sheet = create_test_spreadsheet(5, 5);
        // Parent 1 is a constant, parent 2 is a cell reference
        *sheet.get_mut_cell(0, 1) = CellValue::Integer(5);

        {
            let key2 = sheet.get_key(0, 1);
            let meta = sheet.get_cell_meta(1, 1);
            meta.parent1 = 10; // Constant value
            meta.parent2 = key2; // Cell reference
            meta.formula = 13; // Addition with constant first operand
        }

        let mut sleep_time = 0.0;
        reevaluate_formula(&mut sheet, 1, 1, &mut sleep_time);
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(15)); // 10 + 5 = 15
    }

    #[test]
    fn test_reevaluate_formula_remainder1_subtraction() {
        let mut sheet = create_test_spreadsheet(5, 5);
        // Parent 1 is a constant, parent 2 is a cell reference
        *sheet.get_mut_cell(0, 1) = CellValue::Integer(5);

        {
            let key2 = sheet.get_key(0, 1);
            let meta = sheet.get_cell_meta(1, 1);
            meta.parent1 = 10; // Constant value
            meta.parent2 = key2; // Cell reference
            meta.formula = 23; // Subtraction with constant first operand
        }

        let mut sleep_time = 0.0;
        reevaluate_formula(&mut sheet, 1, 1, &mut sleep_time);
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(5)); // 10 - 5 = 5
    }

    #[test]
    fn test_reevaluate_formula_remainder1_multiplication() {
        let mut sheet = create_test_spreadsheet(5, 5);
        // Parent 1 is a constant, parent 2 is a cell reference
        *sheet.get_mut_cell(0, 1) = CellValue::Integer(5);

        {
            let key2 = sheet.get_key(0, 1);
            let meta = sheet.get_cell_meta(1, 1);
            meta.parent1 = 10; // Constant value
            meta.parent2 = key2; // Cell reference
            meta.formula = 43; // Multiplication with constant first operand
        }

        let mut sleep_time = 0.0;
        reevaluate_formula(&mut sheet, 1, 1, &mut sleep_time);
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(50)); // 10 * 5 = 50
    }

    #[test]
    fn test_reevaluate_formula_remainder1_division() {
        let mut sheet = create_test_spreadsheet(5, 5);
        // Parent 1 is a constant, parent 2 is a cell reference
        *sheet.get_mut_cell(0, 1) = CellValue::Integer(5);

        {
            let key2 = sheet.get_key(0, 1);
            let meta = sheet.get_cell_meta(1, 1);
            meta.parent1 = 10; // Constant value
            meta.parent2 = key2; // Cell reference
            meta.formula = 33; // Division with constant first operand
        }

        let mut sleep_time = 0.0;
        reevaluate_formula(&mut sheet, 1, 1, &mut sleep_time);
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(2)); // 10 / 5 = 2
    }

    #[test]
    fn test_reevaluate_formula_remainder1_division_by_zero() {
        let mut sheet = create_test_spreadsheet(5, 5);
        // Parent 1 is a constant, parent 2 is a cell reference
        *sheet.get_mut_cell(0, 1) = CellValue::Integer(0);

        {
            let key2 = sheet.get_key(0, 1);
            let meta = sheet.get_cell_meta(1, 1);
            meta.parent1 = 10; // Constant value
            meta.parent2 = key2; // Cell reference
            meta.formula = 33; // Division with constant first operand
        }

        let mut sleep_time = 0.0;
        reevaluate_formula(&mut sheet, 1, 1, &mut sleep_time);
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Error); // Division by zero
    }

    #[test]
    fn test_reevaluate_formula_remainder1_error_reference() {
        let mut sheet = create_test_spreadsheet(5, 5);
        // Parent 2 contains an error
        *sheet.get_mut_cell(0, 1) = CellValue::Error;

        {
            let key2 = sheet.get_key(0, 1);
            let meta = sheet.get_cell_meta(1, 1);
            meta.parent1 = 10; // Constant value
            meta.parent2 = key2; // Cell reference with error
            meta.formula = 13; // Addition with constant first operand
        }

        let mut sleep_time = 0.0;
        reevaluate_formula(&mut sheet, 1, 1, &mut sleep_time);
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Error); // Propagates error
    }

    #[test]
    fn test_reevaluate_formula_copy_value() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(42);

        {
            let key1 = sheet.get_key(0, 0);
            let meta = sheet.get_cell_meta(1, 1);
            meta.parent1 = key1;
            meta.parent2 = 0; // Not used
            meta.formula = 82; // Copy value (8 is msb, 2 is remainder)
        }

        let mut sleep_time = 0.0;
        reevaluate_formula(&mut sheet, 1, 1, &mut sleep_time);
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(42)); // Just copies the value
    }

    #[test]
    fn test_reevaluate_formula_sleep() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(3);

        {
            let key1 = sheet.get_key(0, 0);
            let meta = sheet.get_cell_meta(1, 1);
            meta.parent1 = key1;
            meta.parent2 = 0; // Not used
            meta.formula = 92; // Sleep function (9 is msb, 2 is remainder)
        }

        let mut sleep_time = 0.0;
        reevaluate_formula(&mut sheet, 1, 1, &mut sleep_time);
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(3)); // Value copied
        assert_eq!(sleep_time, 3.0); // Sleep value incremented
    }

    #[test]
    fn test_reevaluate_formula_avg() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(2);
        *sheet.get_mut_cell(0, 1) = CellValue::Integer(4);
        *sheet.get_mut_cell(0, 2) = CellValue::Integer(6);
        *sheet.get_mut_cell(0, 3) = CellValue::Integer(8);

        {
            let start_key = sheet.get_key(0, 0);
            let end_key = sheet.get_key(0, 3);
            let meta = sheet.get_cell_meta(1, 1);
            meta.parent1 = start_key;
            meta.parent2 = end_key;
            meta.formula = 6; // AVG function (0 is msb, 6 is remainder)
        }

        let mut sleep_time = 0.0;
        reevaluate_formula(&mut sheet, 1, 1, &mut sleep_time);
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(5)); // AVG of 2,4,6,8 = 5
    }

    #[test]
    fn test_reevaluate_formula_min() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(3);
        *sheet.get_mut_cell(0, 1) = CellValue::Integer(1);
        *sheet.get_mut_cell(0, 2) = CellValue::Integer(5);

        {
            let start_key = sheet.get_key(0, 0);
            let end_key = sheet.get_key(0, 2);
            let meta = sheet.get_cell_meta(1, 1);
            meta.parent1 = start_key;
            meta.parent2 = end_key;
            meta.formula = 7; // MIN function (0 is msb, 7 is remainder)
        }

        let mut sleep_time = 0.0;
        reevaluate_formula(&mut sheet, 1, 1, &mut sleep_time);
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(1)); // MIN of 3,1,5 = 1
    }

    #[test]
    fn test_reevaluate_formula_max() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(3);
        *sheet.get_mut_cell(0, 1) = CellValue::Integer(9);
        *sheet.get_mut_cell(0, 2) = CellValue::Integer(5);

        {
            let start_key = sheet.get_key(0, 0);
            let end_key = sheet.get_key(0, 2);
            let meta = sheet.get_cell_meta(1, 1);
            meta.parent1 = start_key;
            meta.parent2 = end_key;
            meta.formula = 8; // MAX function (0 is msb, 8 is remainder)
        }

        let mut sleep_time = 0.0;
        reevaluate_formula(&mut sheet, 1, 1, &mut sleep_time);
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(9)); // MAX of 3,9,5 = 9
    }

    #[test]
    fn test_reevaluate_formula_variance() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(2);
        *sheet.get_mut_cell(0, 1) = CellValue::Integer(4);
        *sheet.get_mut_cell(0, 2) = CellValue::Integer(6);

        {
            let start_key = sheet.get_key(0, 0);
            let end_key = sheet.get_key(0, 2);
            let meta = sheet.get_cell_meta(1, 1);
            meta.parent1 = start_key;
            meta.parent2 = end_key;
            meta.formula = 9; // VARIANCE function (0 is msb, 9 is remainder)
        }

        let mut sleep_time = 0.0;
        reevaluate_formula(&mut sheet, 1, 1, &mut sleep_time);
        // Variance of [2,4,6] = ((2-4)² + (4-4)² + (6-4)²) / 3 = (4 + 0 + 4) / 3 = 8/3 ≈ 2.67
        // Integer truncation gives us 2
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(2));
    }
    #[test]
    fn test_reevaluate_formula_arithmetic_div_zero() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(5);
        *sheet.get_mut_cell(0, 1) = CellValue::Integer(0);
        let key1 = sheet.get_key(0, 0);
        let key2 = sheet.get_key(0, 1);
        let meta = sheet.get_cell_meta(1, 1);
        meta.parent1 = key1;
        meta.parent2 = key2;
        meta.formula = 30; // Division
        let mut sleep_time = 0.0;
        reevaluate_formula(&mut sheet, 1, 1, &mut sleep_time);
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Error);
    }

    #[test]
    fn test_reevaluate_formula_error() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Error;
        let key1 = sheet.get_key(0, 0);
        let key2 = sheet.get_key(0, 1);
        let meta = sheet.get_cell_meta(1, 1);
        meta.parent1 = key1;
        meta.parent2 = key2;
        meta.formula = 10; // Addition
        let mut sleep_time = 0.0;
        reevaluate_formula(&mut sheet, 1, 1, &mut sleep_time);
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Error);
    }

    #[test]
    fn test_reevaluate_formula_sum() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(1);
        *sheet.get_mut_cell(0, 1) = CellValue::Integer(2);
        let key1 = sheet.get_key(0, 0);
        let key2 = sheet.get_key(0, 1);
        let meta = sheet.get_cell_meta(1, 1);
        meta.parent1 = key1;
        meta.parent2 = key2;
        meta.formula = 5; // SUM
        let mut sleep_time = 0.0;
        reevaluate_formula(&mut sheet, 1, 1, &mut sleep_time);
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(3));
    }

    #[test]
    fn test_cycle_prevention() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(1);
        let mut sleep_time = 0.0;
        assert_eq!(
            set_cell_value(&mut sheet, 1, 1, "A1", &mut sleep_time),
            CommandStatus::CmdOk
        );
        assert_eq!(
            set_cell_value(&mut sheet, 0, 0, "B2", &mut sleep_time),
            CommandStatus::CircularRef
        );
        assert!(!toposort_reval_detect_cycle(
            &mut sheet,
            0,
            0,
            &mut sleep_time
        ));
        assert_eq!(*sheet.get_cell(0, 0), CellValue::Integer(1)); // A1 unchanged
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(1)); // B2 still references A1
    }

    #[test]
    fn test_toposort_reval_no_cycle() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(1);
        let mut sleep_time = 0.0;
        set_cell_value(&mut sheet, 1, 1, "A1", &mut sleep_time);
        assert!(!toposort_reval_detect_cycle(
            &mut sheet,
            1,
            1,
            &mut sleep_time
        ));
    }

    #[test]
    fn test_reevaluate_formula_div_by_zero() {
        let mut sheet: Spreadsheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(5);
        *sheet.get_mut_cell(0, 1) = CellValue::Integer(0);
        let parent1_key = sheet.get_key(0, 0);
        let parent2_key = sheet.get_key(0, 1);
        let meta = sheet.get_cell_meta(1, 1);
        meta.parent1 = parent1_key;
        meta.parent2 = parent2_key;
        meta.formula = 30; // Division
        let mut sleep_time = 0.0;
        reevaluate_formula(&mut sheet, 1, 1, &mut sleep_time);
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Error);
    }
}
