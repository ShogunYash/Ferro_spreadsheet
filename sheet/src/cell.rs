use std::fmt;

// Cell value representation
#[derive(Debug, Clone)]
pub enum CellValue {
    Integer(i32),
    Formula(String),
    Error(String),
    Empty,
}

impl CellValue {
    pub fn as_int(&self) -> Result<i32, String> {
        match self {
            CellValue::Integer(value) => Ok(*value),
            CellValue::Error(msg) => Err(msg.clone()),
            CellValue::Empty => Ok(0),
            _ => Err("Cannot convert to integer".to_string()),
        }
    }
    
    pub fn is_formula(&self) -> bool {
        matches!(self, CellValue::Formula(_))
    }
    
    pub fn get_formula(&self) -> Option<String> {
        match self {
            CellValue::Formula(formula) => Some(formula.clone()),
            _ => None,
        }
    }
}

impl fmt::Display for CellValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CellValue::Integer(value) => write!(f, "{}", value),
            CellValue::Formula(_) => write!(f, "FORMULA"), // This should never be displayed directly
            CellValue::Error(_) => write!(f, "ERR"),
            CellValue::Empty => write!(f, "0"),
        }
    }
}

// Represents a cell in the spreadsheet
#[derive(Debug, Clone)]
pub struct Cell {
    pub value: CellValue,
    pub display_value: CellValue, // The calculated value shown to the user
    pub dependencies: Vec<(usize, usize)>, // Cells this one depends on
    pub dependents: Vec<(usize, usize)>, // Cells that depend on this one
}

impl Cell {
    pub fn new() -> Self {
        Cell {
            value: CellValue::Empty,
            display_value: CellValue::Empty,
            dependencies: Vec::new(),
            dependents: Vec::new(),
        }
    }
    
    pub fn set_value(&mut self, value: CellValue) {
        self.value = value;
        self.update_display_value();
    }
    
    pub fn set_display_value(&mut self, value: CellValue) {
        self.display_value = value;
    }
    
    pub fn update_display_value(&mut self) {
        match &self.value {
            CellValue::Formula(_) => {}, // Don't update yet, needs evaluation
            value => self.display_value = value.clone(),
        }
    }
    
    pub fn add_dependency(&mut self, row: usize, col: usize) {
        if !self.dependencies.contains(&(row, col)) {
            self.dependencies.push((row, col));
        }
    }
    
    pub fn add_dependent(&mut self, row: usize, col: usize) {
        if !self.dependents.contains(&(row, col)) {
            self.dependents.push((row, col));
        }
    }
    
    pub fn clear_dependencies(&mut self) {
        self.dependencies.clear();
    }
    
    pub fn clear_dependent(&mut self, row: usize, col: usize) {
        self.dependents.retain(|&dep| dep != (row, col));
    }
}

impl fmt::Display for Cell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_value)
    }
}