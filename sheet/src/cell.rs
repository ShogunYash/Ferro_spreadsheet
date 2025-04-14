use crate::spreadsheet::{Spreadsheet, CommandStatus}; // Importing Spreadsheet and CommandStatus from spreadsheet module

// Cell value representation
#[derive(Debug, Clone, PartialEq)]
pub enum CellValue {
    Integer(i32),
    Error,
}
#[derive(Debug, Clone)]
// Represents a cell in the spreadsheet - now only contains the value
pub struct Cell {
    pub value: CellValue,
}

impl Cell {
    pub fn new() -> Self {
        Cell {
            value: CellValue::Integer(0),
        }
    }
}

pub fn parse_cell_reference(sheet: &Spreadsheet, cell_ref: &str) -> Result<(i16, i16), CommandStatus> {
    let cell_ref = cell_ref.as_bytes();
    if cell_ref.is_empty() {
        return Err(CommandStatus::CmdUnrecognized);
    }
    
    // Find column/row split point in one pass
    let mut split_idx = 0;
    let mut col_length = 0;
    
    while split_idx < cell_ref.len() && 
          cell_ref[split_idx] >= b'A' && cell_ref[split_idx] <= b'Z' {
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
    let col_name = std::str::from_utf8(&cell_ref[0..split_idx])
        .map_err(|_| CommandStatus::CmdUnrecognized)?;
    
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

// pub fn parse_cell_reference(sheet: &Spreadsheet, cell_ref: &str) -> Result<(i16, i16), CommandStatus> {
//     // Extract column letters
//     let mut i = 0;
//     let mut col_name = String::new();
    
//     for c in cell_ref.chars() {
//         if c.is_ascii_uppercase() {
//             if i >= 3 {
//                 return Err(CommandStatus::CmdUnrecognized);
//             }
//             col_name.push(c);
//             i += 1;
//         } else {
//             break;
//         }
//     }
    
//     // Make sure we have at least one letter and digits follow
//     if col_name.is_empty() || i >= cell_ref.len() {
     
//         return Err(CommandStatus::CmdUnrecognized);
//     }
    
//     // Extract row number
//     let row_str = &cell_ref[i..];
    
//     if row_str.is_empty() {
//         return Err(CommandStatus::CmdUnrecognized);
//     }
    
//     // Validate that all remaining characters are digits
//     if !row_str.chars().all(|c| c.is_ascii_digit()) {
       
//         return Err(CommandStatus::CmdUnrecognized);
//     }
    
//     // Parse row number (convert to 0-based)
//     let row = match row_str.parse::<i16>() {
//         Ok(r) => r - 1,
//         Err(_) => return Err(CommandStatus::CmdUnrecognized)
//     };
    
//     // Convert column name to column index
//     let col = sheet.column_name_to_index(&col_name);
  
//     Ok((row, col))
// }

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_cell_new() {
//         let cell = Cell::new();
//         assert_eq!(cell.value, CellValue::Integer(0));
//         assert_eq!(cell.formula, -1);
//         assert_eq!(cell.parent1, -1);
//         assert_eq!(cell.parent2, -1);
//         assert!(cell.children.is_none());
//     }

//     #[test]
//     fn test_parse_cell_reference() {
//         let sheet = Spreadsheet::create(10, 10).unwrap();
//         assert_eq!(parse_cell_reference(&sheet, "A1"), Ok((0, 0)));
//         assert_eq!(parse_cell_reference(&sheet, "B2"), Ok((1, 1)));
//         assert_eq!(parse_cell_reference(&sheet, "AA10"), Ok((9, 26)));
//         assert_eq!(parse_cell_reference(&sheet, "1A"), Err(CommandStatus::CmdUnrecognized));
//         assert_eq!(parse_cell_reference(&sheet, "A"), Err(CommandStatus::CmdUnrecognized));
//         assert_eq!(parse_cell_reference(&sheet, "A1B"), Err(CommandStatus::CmdUnrecognized));
//         assert_eq!(parse_cell_reference(&sheet, "AAAA1"), Err(CommandStatus::CmdUnrecognized));
//     }
// }