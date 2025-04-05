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
}