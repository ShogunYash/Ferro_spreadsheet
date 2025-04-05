use std::collections::HashSet;
use regex::Regex;
use lazy_static::lazy_static;

use crate::cell::CellValue;
use crate::spreadsheet::Spreadsheet;

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
        if let Ok((ref_row, ref_col)) = sheet.parse_cell_reference(cell_ref) {
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

// Parse an expression
fn parse_expression(sheet: &mut Spreadsheet, row: usize, col: usize, expr: &str) -> Result<CellValue, String> {
    lazy_static! {
        // Regex for functions
        static ref FUNCTION: Regex = Regex::new(r"^(MIN|MAX|AVG|SUM|STDEV|SLEEP)\((.+)\)$").unwrap();
        
        // Regex for arithmetic operations
        static ref ARITHMETIC: Regex = Regex::new(r"^(.+?)([+\-*/])(.+)$").unwrap();
        
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
            _ => Err(format!("Unknown function: {}", function_name)),
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
    
    Err(format!("Invalid expression: {}", expr))
}

// Evaluate a cell reference
fn eval_cell_reference(sheet: &mut Spreadsheet, from_row: usize, from_col: usize, cell_ref: &str) -> Result<CellValue, String> {
    let (to_row, to_col) = sheet.parse_cell_reference(cell_ref)?;
    
    // Register the dependency
    sheet.register_dependency(from_row, from_col, to_row, to_col);
    
    // Get the cell value
    sheet.get_cell_value(to_row, to_col)
}

// Evaluate a range of cells for functions
fn get_range_values(sheet: &mut Spreadsheet, from_row: usize, from_col: usize, range_str: &str) -> Result<Vec<i32>, String> {
    lazy_static! {
        static ref RANGE: Regex = Regex::new(r"^([A-Z]+[0-9]+):([A-Z]+[0-9]+)$").unwrap();
    }
    
    if let Some(caps) = RANGE.captures(range_str) {
        let start_ref = caps.get(1).unwrap().as_str();
        let end_ref = caps.get(2).unwrap().as_str();
        
        let (start_row, start_col) = sheet.parse_cell_reference(start_ref)?;
        let (end_row, end_col) = sheet.parse_cell_reference(end_ref)?;
        
        // Ensure range is valid (start <= end)
        if start_row > end_row || start_col > end_col {
            return Err("Invalid range".to_string());
        }
        
        let mut values = Vec::new();
        
        for r in start_row..=end_row {
            for c in start_col..=end_col {
                // Register dependency for each cell in the range
                sheet.register_dependency(from_row, from_col, r, c);
                
                // Get cell value
                match sheet.get_cell_value(r, c)?.as_int() {
                    Ok(value) => values.push(value),
                    Err(e) => return Err(e),
                }
            }
        }
        
        Ok(values)
    } else {
        Err(format!("Invalid range: {}", range_str))
    }
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
fn eval_min(sheet: &mut Spreadsheet, row: usize, col: usize, range_str: &str) -> Result<CellValue, String> {
    let values = get_range_values(sheet, row, col, range_str)?;
    
    if values.is_empty() {
        return Err("Empty range".to_string());
    }
    
    let min_value = values.iter().min().cloned().unwrap();
    Ok(CellValue::Integer(min_value))
}

fn eval_max(sheet: &mut Spreadsheet, row: usize, col: usize, range_str: &str) -> Result<CellValue, String> {
    let values = get_range_values(sheet, row, col, range_str)?;
    
    if values.is_empty() {
        return Err("Empty range".to_string());
    }
    
    let max_value = values.iter().max().cloned().unwrap();
    Ok(CellValue::Integer(max_value))
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