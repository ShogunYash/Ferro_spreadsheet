use crate::spreadsheet::{Spreadsheet, CommandStatus}; // Importing Spreadsheet and CommandStatus from spreadsheet module

// Cell value representation
#[derive(Debug, Clone)]
pub enum CellValue {
    Integer(i32),
    Error,
}

// Represents a cell in the spreadsheet
#[derive(Debug, Clone)]
pub struct Cell {
        pub  parent1: i32,              // Stores parent cell key or start of range or custom value
        pub  parent2: i32,              // Stores parent cell key or end of range or custom value
        pub  value: CellValue,          // Stores the value of the cell and error state
        pub  formula: i16,              // Stores the formula code
}

impl Cell {
    pub fn new() -> Self {
        Cell {
            parent1: 0,
            parent2: 0,
            value: CellValue::Integer(0),
            formula: -1,  
        }
    }
    pub fn parse_cell_reference(&self, sheet: &Spreadsheet, cell_ref: &str) -> Result<(i16, i16), CommandStatus> {
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
            Err(_) => return Err(CommandStatus::CmdUnrecognized),
        };
        
        // Convert column name to column index
        let col = sheet.column_name_to_index(&col_name);
        
        Ok((row, col))
    }
}