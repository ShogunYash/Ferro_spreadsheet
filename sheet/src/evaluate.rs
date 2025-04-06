use crate::spreadsheet::{Spreadsheet, CommandStatus};
use crate::cell::{Cell, CellValue};


pub fn get_key(row: i16, col: i16, cols: i16) -> i32 {
    ((row as i32 )* (cols as i32) + (col as i32)) as i32
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

        cell.cell1 = get_key(range.start_row, range.start_col, sheet.cols);
        cell.cell2 = get_key(range.end_row, range.end_col, sheet.cols);


        // Evaluate the function.
        if is_stdev {
            variance(sheet, cell);
        } else if is_max {
            min_max(sheet, cell, false);
        } else if is_min {
            min_max(sheet, cell, true);
        } else if is_avg {
            sum_value(sheet, cell);
            let count = (range.end_row - range.start_row + 1) * (range.end_col - range.start_col + 1);
            cell.value = cell.value / count;
        } else {
            sum_value(sheet, cell);
        }
        return CommandStatus::CmdOk;
    }

    CommandStatus::CmdUnrecognized
}







pub fn set_cell_value(sheet: &mut Spreadsheet, row: i16, col: i16, expr: &str, sleep_time: &mut f64) -> CommandStatus {
        let cell = sheet.get_mut_cell(row, col).unwrap();
        let status = evaluate(sheet, cell, row, col, expr, sleep_time);
        status
}