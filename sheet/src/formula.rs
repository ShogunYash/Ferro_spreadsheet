use std::collections::HashSet;
use regex::Regex;
use lazy_static::lazy_static;

use crate::cell::{Cell, CellValue, parse_cell_reference};
use crate::spreadsheet::{Spreadsheet, CommandStatus};

// Evaluate a formula in the context of a cell
pub fn evaluate(sheet: &mut Spreadsheet, row: usize, col: usize, formula: &str) -> Result<CellValue, String> {
    // Check for circular references
    let mut visited = HashSet::new();
    if has_circular_reference(sheet, row, col, formula, &mut visited) {
        return Err("Circular reference detected".to_string());
    }
    
    // Parse and evaluate the formula
    parse_expression(sheet, row, col, formula)
}

// Check if a formula would create a circular reference
fn has_circular_reference(
    sheet: &Spreadsheet,
    curr_row: usize, 
    curr_col: usize, 
    formula: &str, 
    visited: &mut HashSet<(usize, usize)>
) -> bool {
    visited.insert((curr_row, curr_col));
    
    // Look for cell references in the formula
    lazy_static! {
        static ref CELL_REF: Regex = Regex::new(r"([A-Z]+[0-9]+)").unwrap();
    }
    
    for cap in CELL_REF.captures_iter(formula) {
        let cell_ref = cap.get(1).unwrap().as_str();
        
        // Parse the cell reference
        if let Ok((ref_row, ref_col)) = parse_cell_reference(cell_ref) {
            // If we reference ourselves directly, that's a circular reference
            if ref_row == curr_row && ref_col == curr_col {
                return true;
            }
            
            // If we've already visited this cell, we have a circular reference
            if visited.contains(&(ref_row, ref_col)) {
                return true;
            }
            
            // Check if the referenced cell has a formula
            if let Some(cell) = sheet.get_cell(ref_row, ref_col) {
                if let Some(ref_formula) = cell.value.get_formula() {
                    // Recursively check for circular references
                    if has_circular_reference(sheet, ref_row, ref_col, &ref_formula, visited) {
                        return true;
                    }
                }
            }
        }
    }
    
    // Remove ourselves from the visited set as we backtrack
    visited.remove(&(curr_row, curr_col));
    false
}


// Evaluate a cell reference
fn eval_cell_reference(sheet: &mut Spreadsheet, from_row: usize, from_col: usize, cell_ref: &str) -> Result<CellValue, CommandStatus> {
    // let (to_row, to_col) = sheet.parse_cell_reference(cell_ref);
    if let Ok((parent_row, parent_col)) = sheet.parse_cell_reference(cell_ref) {
        let cell = sheet.get_mut_cell(parent_row, parent_col).unwrap();
        
    } else {
        return Err(CommandStatus::CmdUnrecognized);
    }
    
    // Get the cell value
    sheet.get_cell_value(to_row, to_col)
}

pub struct Range {
    pub start_row: i16,
    pub start_col: i16,
    pub end_row: i16,
    pub end_col: i16,
}

pub fn parse_range(range_str: &str) -> Result<Range, CommandStatus> {
    // Find the colon in the range string.
    let colon_index = range_str.find(':').ok_or(CommandStatus::CmdUnrecognized)?;
    
    // Ensure the colon is not the first character and that there is at least one character after.
    if colon_index == 0 || colon_index + 1 >= range_str.len() {
        return Err(CommandStatus::CmdUnrecognized);
    }
    
    // Split the string into the start and end cell strings.
    let start_cell = &range_str[..colon_index];
    let end_cell = &range_str[colon_index + 1..];
    
    // Parse the start cell reference.
    let (start_row, start_col) = parse_cell_reference(start_cell)
        .map_err(|_| CommandStatus::CmdUnrecognized)?;
    if start_row < 0 || start_col < 0 {
        return Err(CommandStatus::CmdUnrecognized);
    }
    
    // Parse the end cell reference.
    let (end_row, end_col) = parse_cell_reference(end_cell)
        .map_err(|_| CommandStatus::CmdUnrecognized)?;
    if end_row < 0 || end_col < 0 {
        return Err(CommandStatus::CmdUnrecognized);
    }
    
    // Ensure the range is valid.
    if start_row > end_row || start_col > end_col {
        return Err(CommandStatus::CmdUnrecognized);
    }
    
    Ok(Range {
        start_row,
        start_col,
        end_row,
        end_col,
    })
}

// Arithmetic operations
fn eval_arithmetic(sheet: &mut Spreadsheet, row: usize, col: usize, left: &str, operator: &str, right: &str) -> Result<CellValue, String> {
    let left_value = parse_expression(sheet, row, col, left)?.as_int()?;
    let right_value = parse_expression(sheet, row, col, right)?.as_int()?;
    
    let result = match operator {
        "+" => left_value + right_value,
        "-" => left_value - right_value,
        "*" => left_value * right_value,
        "/" => {
            if right_value == 0 {
                return Err("Division by zero".to_string());
            }
            left_value / right_value
        },
        _ => return Err(format!("Unknown operator: {}", operator)),
    };
    
    Ok(CellValue::Integer(result))
}

// Aggregation functions
pub fn eval_min(sheet: &mut Spreadsheet, cell: &mut Cell, range: &Range){
    let mut min_value = i32::MAX;
    
    // Iterate over each cell in the range.
    for r in range.start_row..=range.end_row {
        for c in range.start_col..=range.end_col {
            // Retrieve the parent cell.
            let parent_cell = sheet.get_cell(r, c);
            // Extract the integer value; if it is not an integer, skip it.
            if let CellValue::Integer(parent_value) = parent_cell.value {
                min_value = std::cmp::min(min_value, parent_value);
            }
            else{
                cell.value = CellValue::Error;
                return;
            }
        }
    }
    // Set the computed minimum value in the target cell.
    cell.value = CellValue::Integer(min_value);
}

pub fn eval_max(sheet: &mut Spreadsheet, cell: &mut Cell, range: &Range){
    let mut max_value = i32::MIN;
    
    // Iterate over each cell in the range.
    for r in range.start_row..=range.end_row {
        for c in range.start_col..=range.end_col {
            // Retrieve the parent cell.
            let parent_cell = sheet.get_cell(r, c);
            
            
            // Extract the integer value; if it is not an integer, skip it.
            if let CellValue::Integer(parent_value) = parent_cell.value {
                max_value = std::cmp::max(max_value, parent_value);
            }
            else{
                cell.value = CellValue::Error;
                return;
            }
        }
    }
    // Set the computed minimum value in the target cell.
    cell.value = CellValue::Integer(max_value);
}

fn eval_sum(sheet: &mut Spreadsheet, row: usize, col: usize, range_str: &str) -> Result<CellValue, String> {
    let values = get_range_values(sheet, row, col, range_str)?;
    
    let sum: i32 = values.iter().sum();
    Ok(CellValue::Integer(sum))
}

fn eval_avg(sheet: &mut Spreadsheet, row: usize, col: usize, range_str: &str) -> Result<CellValue, String> {
    let values = get_range_values(sheet, row, col, range_str)?;
    
    if values.is_empty() {
        return Err("Empty range".to_string());
    }
    
    let sum: i32 = values.iter().sum();
    let avg = sum / values.len() as i32;
    Ok(CellValue::Integer(avg))
}

fn eval_stdev(sheet: &mut Spreadsheet, row: usize, col: usize, range_str: &str) -> Result<CellValue, String> {
    let values = get_range_values(sheet, row, col, range_str)?;
    
    if values.len() <= 1 {
        return Err("Need at least two values for standard deviation".to_string());
    }
    
    // Calculate mean
    let sum: i32 = values.iter().sum();
    let mean = sum as f64 / values.len() as f64;
    
    // Calculate sum of squared differences
    let variance_sum: f64 = values.iter()
        .map(|&v| {
            let diff = v as f64 - mean;
            diff * diff
        })
        .sum();
    
    // Standard deviation
    let stdev = (variance_sum / (values.len() - 1) as f64).sqrt();
    
    // Convert to integer (truncating decimal part)
    Ok(CellValue::Integer(stdev as i32))
}

fn eval_sleep(sheet: &mut Spreadsheet, row: usize, col: usize, arg: &str) -> Result<CellValue, String> {
    // Parse the argument
    let value = parse_expression(sheet, row, col, arg)?.as_int()?;
    
    // Sleep for the specified number of seconds
    let result = sheet.sleep(value);
    
    Ok(CellValue::Integer(result))
}