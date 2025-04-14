use crate::cell::{CellValue, parse_cell_reference};
use crate::spreadsheet::{Spreadsheet, CommandStatus};
pub struct Range {
    pub start_row: i16,
    pub start_col: i16,
    pub end_row: i16,
    pub end_col: i16,
}

// Optimize the sum_value function for large ranges
pub fn sum_value(sheet: &mut Spreadsheet, row: i16, col: i16, range: &Range) -> CommandStatus {    
    let mut sum = 0;

    // For smaller ranges, use the original approach
    for i in range.start_row..=range.end_row {
        for j in range.start_col..=range.end_col {
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
pub fn eval_variance(sheet: &mut Spreadsheet, row:i16 , col:i16 , range: &Range) -> CommandStatus {
    let count = ((range.end_row - range.start_row + 1) as i32) * ((range.end_col - range.start_col + 1) as i32);
    sum_value(sheet, row, col, range);
    let cell_value = sheet.get_mut_cell(row, col);

    if let CellValue::Integer(value) = cell_value {
        *cell_value = CellValue::Integer((*value / count) as i32);
    }
    else {
        return CommandStatus::CmdOk;
    }

    let mean_value = match *sheet.get_cell(row, col) {
        CellValue::Integer(value) => value as f64,
        _ => return CommandStatus::CmdOk,
    };

    let mut variance = 0.0;
    for i in range.start_row..=range.end_row {
        for j in range.start_col..=range.end_col {
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

pub fn eval_min(sheet: &mut Spreadsheet, row: i16, col: i16, range: &Range) -> CommandStatus {    
    let mut min_value = i32::MAX;
    
    // First collect all values (immutable borrows)
    for r in range.start_row..=range.end_row {
        for c in range.start_col..=range.end_col {
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
pub fn eval_max(sheet: &mut Spreadsheet, row: i16, col: i16, range: &Range) -> CommandStatus {
    let mut max_value = i32::MIN;    
    // First collect all values (immutable borrows)
    for r in range.start_row..=range.end_row {
        for c in range.start_col..=range.end_col {
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
    if start_row < 0 || start_col < 0 || end_row < 0 || end_col < 0 || 
       start_row > end_row || start_col > end_col {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sum_value() {
        let mut sheet = Spreadsheet::create(5, 5).unwrap();
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(1);
        *sheet.get_mut_cell(0, 1) = CellValue::Integer(2);
        let range = Range { start_row: 0, start_col: 0, end_row: 0, end_col: 1 };
        assert_eq!(sum_value(&mut sheet, 1, 1, &range), CommandStatus::CmdOk);
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(3));
    }

    #[test]
    fn test_eval_variance() {
        let mut sheet = Spreadsheet::create(5, 5).unwrap();
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(2);
        *sheet.get_mut_cell(0, 1) = CellValue::Integer(4);
        let range = Range { start_row: 0, start_col: 0, end_row: 0, end_col: 1 };
        assert_eq!(eval_variance(&mut sheet, 1, 1, &range), CommandStatus::CmdOk);
    }

    #[test]
    fn test_eval_min_max() {
        let mut sheet = Spreadsheet::create(5, 5).unwrap();
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(1);
        *sheet.get_mut_cell(0, 1) = CellValue::Integer(3);
        let range = Range { start_row: 0, start_col: 0, end_row: 0, end_col: 1 };
        assert_eq!(eval_min(&mut sheet, 1, 1, &range), CommandStatus::CmdOk);
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(1));
        assert_eq!(eval_max(&mut sheet, 1, 2, &range), CommandStatus::CmdOk);
        assert_eq!(*sheet.get_cell(1, 2), CellValue::Integer(3));
    }

    #[test]
    fn test_parse_range() {
        let sheet = Spreadsheet::create(5, 5).unwrap();
        assert!(parse_range(&sheet, "A1:B2").is_ok());
        assert!(parse_range(&sheet, "A").is_err());
    }
}