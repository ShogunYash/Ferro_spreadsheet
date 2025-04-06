use crate::spreadsheet::{Spreadsheet, CommandStatus};
use crate::cell::{Cell, CellValue};
use crate::formula::{min_max, sum_value, variance};
use regex::Regex;
use lazy_static::lazy_static;

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
    let cell_ref = cell.parse_cell_reference(sheet, expr);
    if cell_ref == Ok(_) {
        let (row, col) = cell_ref.unwrap();
        let parent_cell = sheet.get_cell(row, col).unwrap();
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
    else if Ok(val) == expr.parse::<i32>() {
        cell.value = CellValue::Integer(val);
        *sleep_time += val;
        cell.formula = -1;
        cell.parent1 = -1;
        cell.parent2 = -1;
    }
    else{
        return CmdUnrecognized;
    }

    CmdOk
}

// Parse an expression
fn parse_expression(sheet: &mut Spreadsheet, row: i16, col: i16, expr: &str) -> Result<CellValue, CommandStatus> {
    lazy_static! {
        // Regex for functions
        static ref FUNCTION: Regex = Regex::new(r"^(MIN|MAX|AVG|SUM|STDEV|SLEEP)\((.+)\)$").unwrap();
        
        // Regex for arithmetic operations
        static ref ARITHMETIC: Regex = Regex::new(r"^(.+?)([+\-*])(.+)$").unwrap();
        
        // Regex for cell references
        static ref CELL_REF: Regex = Regex::new(r"^([A-Z]+[0-9]+)$").unwrap();
        
        // Regex for ranges
        static ref RANGE: Regex = Regex::new(r"^([A-Z]+[0-9]+):([A-Z]+[0-9]+)$").unwrap();
    }
    
    // Try to parse as an integer constant
    if let Ok(value) = expr.parse::<i32>() {
        return Ok(CellValue::Integer(value));
    }
    
    // Try to parse as a function
    if let Some(caps) = FUNCTION.captures(expr) {
        let function_name = caps.get(1).unwrap().as_str();
        let args = caps.get(2).unwrap().as_str();
        
        return match function_name {
            "MIN" => eval_min(sheet, row, col, args),
            "MAX" => eval_max(sheet, row, col, args),
            "AVG" => eval_avg(sheet, row, col, args),
            "SUM" => eval_sum(sheet, row, col, args),
            "STDEV" => eval_stdev(sheet, row, col, args),
            "SLEEP" => eval_sleep(sheet, row, col, args),
            _ => Err(CommandStatus::CmdUnrecognized),
        };
    }
    
    // Try to parse as an arithmetic operation
    if let Some(caps) = ARITHMETIC.captures(expr) {
        let left = caps.get(1).unwrap().as_str();
        let operator = caps.get(2).unwrap().as_str();
        let right = caps.get(3).unwrap().as_str();
        return eval_arithmetic(sheet, row, col, left, operator, right);
    }
    
    // Try to parse as a cell reference
    if let Some(caps) = CELL_REF.captures(expr) {
        let cell_ref = caps.get(1).unwrap().as_str();
        return eval_cell_reference(sheet, row, col, cell_ref);
    }
    
    Err(CommandStatus::CmdUnrecognized)
}


pub fn set_cell_value(sheet: &mut Spreadsheet, row: i16, col: i16, expr: &str, sleep_time: &mut f64) -> CommandStatus {
        let cell = sheet.get_mut_cell(row, col).unwrap();
        let status = evaluate(sheet, cell, row, col, expr, sleep_time);
        status
}