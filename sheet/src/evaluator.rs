
use crate::spreadsheet::{Spreadsheet, CommandStatus};
use crate::cell::{Cell, CellValue, parse_cell_reference};
// use crate::formula::{sum_value, min_max, variance};
use crate::formula::parse_range;
use crate::formula::Range;
use crate::formula::{eval_max,eval_min};
pub fn get_key(row: i16, col: i16, cols: i16) -> i32 {
    ((row as i32 )* (cols as i32) + (col as i32)) as i32
}

pub fn handle_sleep(
    sheet: &mut Spreadsheet,
    cell: &mut Cell,
    row: i16,
    col: i16,
    expr: &str,
    sleep_time: &mut f64,
) -> CommandStatus{
    let cell_ref = parse_cell_reference(sheet, expr);
    if cell_ref == Ok(_) {
        let (row, col) = cell_ref.unwrap();
        let parent_cell = sheet.get_cell(row, col);
        cell.formula = 102;    // Custom formula code for sleep
        cell.parent1 = get_key(row, col, sheet.cols);
        cell.parent2 = -1;    // No second parent for sleep
        cell.value = parent_cell.value.clone();
        if let CellValue::Integer(value) = parent_cell.value {
            *sleep_time += value as f64;
        }
        else{
            cell.value = CellValue::Error;
        }
    }
    else if let Ok(val) = expr.parse::<i32>() {
        cell.value = CellValue::Integer(val);
        *sleep_time += val as f64;
        cell.formula = -1;
        cell.parent1 = -1;
        cell.parent2 = -1;
    }
    else{
        return CommandStatus::CmdUnrecognized;
    }

    CommandStatus::CmdOk
}

pub fn evaluate_formula(
    sheet: &mut Spreadsheet,
    cell: &mut Cell,
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

        // Extract the range string without allocating extra memory.
        let range_str = &expr[prefix_len..expr_len - 1];
        let range = match parse_range(range_str) {
            Ok(r) => r,
            Err(status) => return status,
        };

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

        cell.parent1 = get_key(range.start_row, range.start_col, sheet.cols);
        cell.parent2 = get_key(range.end_row, range.end_col, sheet.cols);


        // Evaluate the function.
        if is_stdev {
            eval_variance(sheet, cell, );
        } else if is_max {
            eval_max(sheet, cell, &range);
        } else if is_min {
            eval_min(sheet, cell);
        } else if is_avg {
            sum_value(sheet, cell);
            let count = (range.end_row - range.start_row + 1) * (range.end_col - range.start_col + 1);
            cell.value = cell.value / count;
        } else {
            sum_value(sheet, cell);
        }
        return CommandStatus::CmdOk;
    }

    else if expr.starts_with("SLEEP(") {
        if !expr.ends_with(')') {
            return CommandStatus::CmdUnrecognized;
        }

        let sleep_str = &expr[6..expr_len - 1];
        handle_sleep(sheet, cell, row, col, sleep_str, sleep_time);
    }
    CommandStatus::CmdOk
}


pub fn set_cell_value(sheet: &mut Spreadsheet, row: i16, col: i16, expr: &str, sleep_time: &mut f64) -> CommandStatus {
        let cell = sheet.get_mut_cell(row, col);
        let status = evaluate_formula(sheet, cell, row, col, expr, sleep_time);
        status
}