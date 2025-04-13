use crate::cell::{Cell, CellValue, parse_cell_reference};
use crate::spreadsheet::{Spreadsheet, CommandStatus};
pub struct Range {
    pub start_row: i16,
    pub start_col: i16,
    pub end_row: i16,
    pub end_col: i16,
}
// Add missing functions and fix errors
pub fn sum_value(sheet: &mut Spreadsheet, row: i16, col: i16, range: &Range) -> CommandStatus {
    let row1 = range.start_row;
    let col1 = range.start_col;
    let row2 = range.end_row;
    let col2 = range.end_col;
    
    let mut sum = 0;
    let mut has_error = false;
    
    // First collect all values (immutable borrows)
    for i in row1..=row2 {
        for j in col1..=col2 {
            let ref_cell = sheet.get_cell(i, j);
                if let CellValue::Integer(value) = ref_cell.value {
                    sum += value;
                } else {
                    has_error = true;
                    break;
                }
        }
        if has_error {
            break;
        }
    }
    
    // Now get the mutable cell and set its value (mutable borrow)
    let cell = sheet.get_mut_cell(row, col);
    
    if has_error {
        cell.value = CellValue::Error;
        return CommandStatus::CmdOk;
    }
    
    cell.value = CellValue::Integer(sum);
    CommandStatus::CmdOk
}

// Add variance evaluation function
pub fn eval_variance(sheet: &mut Spreadsheet, row:i16 , col:i16 , range: &Range) -> CommandStatus {
    
    let row1 = range.start_row;
    let col1 = range.start_col;
    let row2 = range.end_row;
    let col2 = range.end_col;
    
    let count = (row2 - row1 + 1) * (col2 - col1 + 1);
    
    // First calculate sum to get mean
    let mut sum = 0;
    let mut has_error = false;
    for i in row1..=row2 {
        for j in col1..=col2 {
            let ref_cell = sheet.get_cell(i, j);
                if let CellValue::Integer(value) = ref_cell.value {
                    sum += value;
                } else {
                    has_error = true;
                    break;
                }
        }
        if has_error {
            break;
        }
    }
    
    
    // Calculate mean
    let mean = sum as f64 / count as f64;
    //has_error=false;
    // Calculate variance
    let mut variance = 0.0;
    for i in row1..=row2 {
        for j in col1..=col2 {
            let ref_cell = sheet.get_cell(i, j);
                if let CellValue::Integer(value) = ref_cell.value {
                    let diff = value as f64 - mean;
                    variance += diff * diff;
                } else {
                    has_error = true;
                    break;
                }
        }
        if has_error {
            break;
        }
    }
    let cell = sheet.get_mut_cell(row, col);
    
    if has_error {
        cell.value = CellValue::Error;
        return CommandStatus::CmdOk;
    }
   

    
    variance /= count as f64;
    
    // Calculate standard deviation and round to integer
    use std::f64;
    cell.value = CellValue::Integer(f64::sqrt(variance).round() as i32);
    
    CommandStatus::CmdOk
}

pub fn eval_min(sheet: &mut Spreadsheet, row: i16, col: i16, range: &Range) -> CommandStatus {
    let row1 = range.start_row;
    let col1 = range.start_col;
    let row2 = range.end_row;
    let col2 = range.end_col;
    
    let mut min_value = i32::MAX;
    let mut has_error = false;
    
    // First collect all values (immutable borrows)
    for r in row1..=row2 {
        for c in col1..=col2 {
            let parent_cell= sheet.get_cell(r, c);
                if let CellValue::Integer(value) = parent_cell.value {
                    min_value = std::cmp::min(min_value, value);
                } else {
                    has_error = true;
                    break;
                }
        }
        if has_error {
            break;
        }
    }
    
    // Now get the mutable cell and set its value (mutable borrow)
    let cell = sheet.get_mut_cell(row, col);
    
    if has_error {
        cell.value = CellValue::Error;
        return CommandStatus::CmdOk;
    }
    
    cell.value = CellValue::Integer(min_value);
    CommandStatus::CmdOk
}

// Fix eval_max implementation
pub fn eval_max(sheet: &mut Spreadsheet, row: i16, col: i16, range: &Range) -> CommandStatus {
    let mut max_value = i32::MIN;
    let mut has_error = false;
    
    // First collect all values (immutable borrows)
    for r in range.start_row..=range.end_row {
        for c in range.start_col..=range.end_col {
            let parent_cell= sheet.get_cell(r, c); 
                if let CellValue::Integer(value) = parent_cell.value {
                    max_value = std::cmp::max(max_value, value);
                } else {
                    has_error = true;
                    break;
                }
        }
        if has_error {
            break;
        }
    }
    
    // Now get the mutable cell and set its value (mutable borrow)
    let cell = sheet.get_mut_cell(row, col);
    
    if has_error {
        cell.value = CellValue::Error;
        return CommandStatus::CmdOk;
    }
    
    cell.value = CellValue::Integer(max_value);
    CommandStatus::CmdOk
}

// Keep the Range struct and parse_range function


pub fn parse_range(spreadsheet: &Spreadsheet,range_str: &str) -> Result<Range, CommandStatus> {
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
    let (start_row, start_col) = parse_cell_reference(&spreadsheet, start_cell)
        .map_err(|_| CommandStatus::CmdUnrecognized)?;
    if start_row < 0 || start_col < 0 {
        return Err(CommandStatus::CmdUnrecognized);
    }
    
    // Parse the end cell reference.
    let (end_row, end_col) = parse_cell_reference(&spreadsheet,end_cell)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sum_value() {
        let mut sheet = Spreadsheet::create(5, 5).unwrap();
        sheet.get_mut_cell(0, 0).value = CellValue::Integer(1);
        sheet.get_mut_cell(0, 1).value = CellValue::Integer(2);
        let range = Range { start_row: 0, start_col: 0, end_row: 0, end_col: 1 };
        assert_eq!(sum_value(&mut sheet, 1, 1, &range), CommandStatus::CmdOk);
        assert_eq!(sheet.get_cell(1, 1).value, CellValue::Integer(3));
    }

    #[test]
    fn test_eval_variance() {
        let mut sheet = Spreadsheet::create(5, 5).unwrap();
        sheet.get_mut_cell(0, 0).value = CellValue::Integer(2);
        sheet.get_mut_cell(0, 1).value = CellValue::Integer(4);
        let range = Range { start_row: 0, start_col: 0, end_row: 0, end_col: 1 };
        assert_eq!(eval_variance(&mut sheet, 1, 1, &range), CommandStatus::CmdOk);
    }

    #[test]
    fn test_eval_min_max() {
        let mut sheet = Spreadsheet::create(5, 5).unwrap();
        sheet.get_mut_cell(0, 0).value = CellValue::Integer(1);
        sheet.get_mut_cell(0, 1).value = CellValue::Integer(3);
        let range = Range { start_row: 0, start_col: 0, end_row: 0, end_col: 1 };
        assert_eq!(eval_min(&mut sheet, 1, 1, &range), CommandStatus::CmdOk);
        assert_eq!(sheet.get_cell(1, 1).value, CellValue::Integer(1));
        assert_eq!(eval_max(&mut sheet, 1, 2, &range), CommandStatus::CmdOk);
        assert_eq!(sheet.get_cell(1, 2).value, CellValue::Integer(3));
    }

    #[test]
    fn test_parse_range() {
        let sheet = Spreadsheet::create(5, 5).unwrap();
        assert!(parse_range(&sheet, "A1:B2").is_ok());
        assert!(parse_range(&sheet, "A").is_err());
    }
}