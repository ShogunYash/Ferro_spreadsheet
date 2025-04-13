use crate::spreadsheet::{Spreadsheet, CommandStatus}; // Importing Spreadsheet and CommandStatus from spreadsheet module
use crate::linked_list::Node; // Importing Node from linked_list module


// Cell value representation
#[derive(Debug, Clone, PartialEq)]
pub enum CellValue {
    Integer(i32),
    Error,
}

// Represents a cell in the spreadsheet
// Fields are ordered by size (largest to smallest) to minimize padding
pub struct Cell {
    pub children: Option<Box<Node>>,  // Largest field (pointer)
    pub parent1: i32,                 // 4 bytes
    pub parent2: i32,                 // 4 bytes
    pub value: CellValue,             // enum (typically 8 bytes with tag)
    pub formula: i16,                 // 2 bytes - smallest field
} 

impl Cell {
    pub fn new() -> Self {
        Cell {
            children: None,
            parent1: -1,
            parent2: -1,
            value: CellValue::Integer(0),
            formula: -1,
        }
    }
}

pub fn parse_cell_reference(sheet: &Spreadsheet, cell_ref: &str) -> Result<(i16, i16), CommandStatus> {
    // Extract column letters
    let mut i = 0;
    let mut col_name = String::new();
    
    for c in cell_ref.chars() {
        if c.is_ascii_uppercase() {
            if i >= 3 {
                return Err(CommandStatus::CmdUnrecognized);
            }
            col_name.push(c);
            i += 1;
        } else {
            break;
        }
    }
    
    // Make sure we have at least one letter and digits follow
    if col_name.is_empty() || i >= cell_ref.len() {
    
        return Err(CommandStatus::CmdUnrecognized);
    }
    
    // Extract row number
    let row_str = &cell_ref[i..];
    
    if row_str.is_empty() {
        return Err(CommandStatus::CmdUnrecognized);
    }
    
    // Validate that all remaining characters are digits
    if !row_str.chars().all(|c| c.is_ascii_digit()) {
       
        return Err(CommandStatus::CmdUnrecognized);
    }
    
    // Parse row number (convert to 0-based)
    let row = match row_str.parse::<i16>() {
        Ok(r) => r - 1,
        Err(_) => return Err(CommandStatus::CmdUnrecognized)
    };
    
    // Convert column name to column index
    let col = sheet.column_name_to_index(&col_name);
  
    Ok((row, col))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_new() {
        let cell = Cell::new();
        assert_eq!(cell.value, CellValue::Integer(0));
        assert_eq!(cell.formula, -1);
        assert_eq!(cell.parent1, -1);
        assert_eq!(cell.parent2, -1);
        assert!(cell.children.is_none());
    }

    #[test]
    fn test_parse_cell_reference() {
        let sheet = Spreadsheet::create(10, 10).unwrap();
        assert_eq!(parse_cell_reference(&sheet, "A1"), Ok((0, 0)));
        assert_eq!(parse_cell_reference(&sheet, "B2"), Ok((1, 1)));
        assert_eq!(parse_cell_reference(&sheet, "AA10"), Ok((9, 26)));
        assert_eq!(parse_cell_reference(&sheet, "1A"), Err(CommandStatus::CmdUnrecognized));
        assert_eq!(parse_cell_reference(&sheet, "A"), Err(CommandStatus::CmdUnrecognized));
        assert_eq!(parse_cell_reference(&sheet, "A1B"), Err(CommandStatus::CmdUnrecognized));
        assert_eq!(parse_cell_reference(&sheet, "AAAA1"), Err(CommandStatus::CmdUnrecognized));
    }
}