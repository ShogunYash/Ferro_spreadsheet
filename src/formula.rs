use crate::cell::{CellValue, parse_cell_reference};
use crate::spreadsheet::{Spreadsheet, CommandStatus};

#[derive(Debug, PartialEq)]
pub struct Range {
    pub start_row: i16,
    pub start_col: i16,
    pub end_row: i16,
    pub end_col: i16,
}

// Optimize the sum_value function for large ranges
pub fn sum_value(sheet: &mut Spreadsheet, row: i16, col: i16, parent1: i32, parent2: i32) -> CommandStatus {    
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
pub fn eval_variance(sheet: &mut Spreadsheet, row:i16 , col:i16, parent1: i32, parent2: i32) -> CommandStatus {
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
    }
    else {
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


pub fn eval_min(sheet: &mut Spreadsheet, row: i16, col: i16, parent1: i32, parent2: i32) -> CommandStatus {    
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
pub fn eval_max(sheet: &mut Spreadsheet, row: i16, col: i16, parent1: i32, parent2: i32) -> CommandStatus {
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


pub fn eval_avg(sheet: &mut Spreadsheet, row: i16, col: i16, parent1: i32, parent2: i32) -> CommandStatus {
    let (start_row, start_col) = sheet.get_row_col(parent1);
    let (end_row, end_col) = sheet.get_row_col(parent2);
    let count = ((end_row - start_row + 1) as i32) * ((end_col - start_col + 1) as i32);
    match sum_value(sheet, row, col, parent1, parent2) {
        CommandStatus::CmdOk => {
            let cell_value = sheet.get_mut_cell(row, col);
            if let CellValue::Integer(value) = cell_value {
                *cell_value = CellValue::Integer(*value / count);
            }
        },
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
    use crate::cell::CellValue;
    use crate::spreadsheet::{CommandStatus, Spreadsheet};

    fn create_test_spreadsheet(rows: i16, cols: i16) -> Spreadsheet {
        Spreadsheet::create(rows, cols).unwrap()
    }

    #[test]
    fn test_sum_value() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(1);
        *sheet.get_mut_cell(0, 1) = CellValue::Integer(2);
        let parent1 = sheet.get_key(0, 0);
        let parent2 = sheet.get_key(0, 1);
        assert_eq!(
            sum_value(&mut sheet, 1, 1, parent1, parent2),
            CommandStatus::CmdOk
        );
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(3));
    }

    #[test]
    fn test_sum_value_error() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(1);
        *sheet.get_mut_cell(0, 1) = CellValue::Error;
        let parent1 = sheet.get_key(0, 0);
        let parent2 = sheet.get_key(0, 1);
        assert_eq!(
            sum_value(&mut sheet, 1, 1, parent1, parent2),
            CommandStatus::CmdOk
        );
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Error);
    }

    #[test]
    fn test_eval_variance() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(2);
        *sheet.get_mut_cell(0, 1) = CellValue::Integer(4);
        let parent1 = sheet.get_key(0, 0);
        let parent2 = sheet.get_key(0, 1);
        assert_eq!(
            eval_variance(&mut sheet, 1, 1, parent1, parent2),
            CommandStatus::CmdOk
        );
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(1));
    }

    #[test]
    fn test_eval_variance_error() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Error;
        *sheet.get_mut_cell(0, 1) = CellValue::Integer(4);
        let parent1 = sheet.get_key(0, 0);
        let parent2 = sheet.get_key(0, 1);
        assert_eq!(
            eval_variance(&mut sheet, 1, 1, parent1, parent2),
            CommandStatus::CmdOk
        );
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Error);
    }

    #[test]
    fn test_eval_min() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(1);
        *sheet.get_mut_cell(0, 1) = CellValue::Integer(3);
        let parent1 = sheet.get_key(0, 0);
        let parent2 = sheet.get_key(0, 1);
        assert_eq!(
            eval_min(&mut sheet, 1, 1, parent1, parent2),
            CommandStatus::CmdOk
        );
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(1));
    }

    #[test]
    fn test_eval_max() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(1);
        *sheet.get_mut_cell(0, 1) = CellValue::Integer(3);
        let parent1 = sheet.get_key(0, 0);
        let parent2 = sheet.get_key(0, 1);
        assert_eq!(
            eval_max(&mut sheet, 1, 2, parent1, parent2),
            CommandStatus::CmdOk
        );
        assert_eq!(*sheet.get_cell(1, 2), CellValue::Integer(3));
    }

    #[test]
    fn test_eval_min_max_error() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(1);
        *sheet.get_mut_cell(0, 1) = CellValue::Error;
        let parent1 = sheet.get_key(0, 0);
        let parent2 = sheet.get_key(0, 1);
        assert_eq!(
            eval_min(&mut sheet, 1, 1, parent1, parent2),
            CommandStatus::CmdOk
        );
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Error);
        assert_eq!(
            eval_max(&mut sheet, 1, 2, parent1, parent2),
            CommandStatus::CmdOk
        );
        assert_eq!(*sheet.get_cell(1, 2), CellValue::Error);
    }

    #[test]
    fn test_eval_avg() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(2);
        *sheet.get_mut_cell(0, 1) = CellValue::Integer(4);
        let parent1 = sheet.get_key(0, 0);
        let parent2 = sheet.get_key(0, 1);
        assert_eq!(
            eval_avg(&mut sheet, 1, 1, parent1, parent2),
            CommandStatus::CmdOk
        );
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(3));
    }

    #[test]
    fn test_parse_range_valid() {
        let sheet = create_test_spreadsheet(5, 5);
        let range = parse_range(&sheet, "A1:B2").unwrap();
        assert_eq!(range.start_row, 0);
        assert_eq!(range.start_col, 0);
        assert_eq!(range.end_row, 1);
        assert_eq!(range.end_col, 1);
    }

    #[test]
    fn test_parse_range_invalid() {
        let sheet = create_test_spreadsheet(5, 5);
        assert_eq!(
            parse_range(&sheet, "A"),
            Err(CommandStatus::CmdUnrecognized)
        );
        assert_eq!(
            parse_range(&sheet, "A1:"),
            Err(CommandStatus::CmdUnrecognized)
        );
        assert_eq!(
            parse_range(&sheet, ":A1"),
            Err(CommandStatus::CmdUnrecognized)
        );
        assert_eq!(
            parse_range(&sheet, "B2:A1"),
            Err(CommandStatus::CmdUnrecognized)
        );
    }
}