use std::fs::OpenOptions;
use std::io::{BufReader, BufWriter, Write, Read, BufRead};
use std::path::Path;
use std::fs::File;
use crate::spreadsheet::CommandStatus;
use crate::spreadsheet::Spreadsheet;
use crate::graph;
use crate::cell::{CellValue,parse_cell_reference};

pub fn save_spreadsheet(sheet: &Spreadsheet, filename: &str) -> CommandStatus {
    let path = Path::new(filename);
    
    // Open file for writing
    let file = match OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
    {
        Ok(file) => file,
        Err(_) => return CommandStatus::CmdUnrecognized,
    };

    // Create a buffered writer
    let mut writer = BufWriter::new(file);

    // Write header with dimensions
    if let Err(_) = writeln!(writer, "DIMS,{},{}", sheet.rows, sheet.cols) {
        return CommandStatus::CmdUnrecognized;
    }

    // Write cell data with formulas
    for row in 0..sheet.rows {
        for col in 0..sheet.cols {
            let key = sheet.get_key(row, col);
            let cell_value = sheet.get_cell(row, col);
            
            // Only write cells with non-zero values or formulas
            let is_nonzero = match cell_value {
                CellValue::Integer(0) => false,
                _ => true,
            };
            
            // Check if cell has formula metadata
            let has_metadata = sheet.cell_meta.contains_key(&key);
            
            if is_nonzero || has_metadata {
                let cell_ref = format!("{}{}", sheet.get_column_name(col), row + 1);
                
                // Write the cell value
                match cell_value {
                    CellValue::Integer(val) => {
                        write!(writer, "CELL,{},{}", cell_ref, val).unwrap();
                    },
                    CellValue::Error => {
                        write!(writer, "CELL,{},ERR", cell_ref).unwrap();
                    },
                }
                
                // If the cell has formula metadata, write it too
                if let Some(meta) = sheet.cell_meta.get(&key) {
                    if meta.formula != -1 {
                        // Get the parent cells as references
                        let parent1_ref = if meta.parent1 != -1 {
                            let (p1_row, p1_col) = sheet.get_row_col(meta.parent1);
                            format!("{}{}", sheet.get_column_name(p1_col), p1_row + 1)
                        } else {
                            String::from("")
                        };
                        
                        let parent2_ref = if meta.parent2 != -1 {
                            let (p2_row, p2_col) = sheet.get_row_col(meta.parent2);
                            format!("{}{}", sheet.get_column_name(p2_col), p2_row + 1)
                        } else {
                            String::from("")
                        };
                        
                        write!(writer, ",FORMULA,{},{},{}", 
                            meta.formula, parent1_ref, parent2_ref).unwrap();
                    }
                }
                
                // End the line
                writeln!(writer, "").unwrap();
            }
        }
    }

    CommandStatus::CmdOk
}

// Load a spreadsheet from a file
pub fn load_spreadsheet(sheet: &mut Spreadsheet, filename: &str) -> CommandStatus {
    let path = Path::new(filename);

    // Open file for reading
    let file = match File::open(path) {
        Ok(file) => file,
        Err(_) => return CommandStatus::CmdUnrecognized,
    };

    // Create a buffered reader
    let reader = BufReader::new(file);
    
    // Clear the existing spreadsheet
    for row in 0..sheet.rows {
        for col in 0..sheet.cols {
            let key = sheet.get_key(row, col);
            // Clear cell value
            let index = sheet.get_index(row, col);
            sheet.grid[index] = CellValue::Integer(0);
            
            // Clear metadata and dependencies
            if sheet.cell_meta.contains_key(&key) {
                // Remove all parent-child relationships
                graph::remove_all_parents(sheet, row, col);
                sheet.cell_meta.remove(&key);
            }
        }
    }

    // Read and parse the file
    for line_result in reader.lines() {
        let line = match line_result {
            Ok(line) => line,
            Err(_) => continue,
        };
        
        let parts: Vec<&str> = line.split(',').collect();
        if parts.is_empty() {
            continue;
        }
        
        // Process line based on type
        match parts[0] {
            "DIMS" => {
                // Dimensions line: DIMS,rows,cols
                if parts.len() >= 3 {
                    // We don't resize the sheet here, just validate dimensions
                    let file_rows: i16 = parts[1].parse().unwrap_or(0);
                    let file_cols: i16 = parts[2].parse().unwrap_or(0);
                    
                    if file_rows > sheet.rows || file_cols > sheet.cols {
                        eprintln!("Warning: File contains a larger spreadsheet than current dimensions");
                    }
                }
            },
            "CELL" => {
                // Cell data line: CELL,ref,value[,FORMULA,formula_code,parent1,parent2]
                if parts.len() >= 3 {
                    let cell_ref = parts[1];
                    let value_str = parts[2];
                    
                    // Parse cell reference
                    if let Ok((row, col)) = parse_cell_reference(sheet, cell_ref) {
                        // Set cell value
                        let cell_value = if value_str == "ERR" {
                            CellValue::Error
                        } else {
                            match value_str.parse::<i32>() {
                                Ok(val) => CellValue::Integer(val),
                                Err(_) => continue,
                            }
                        };
                        
                        let index = sheet.get_index(row, col);
                        sheet.grid[index] = cell_value;
                        
                        // If there's formula data, process it
                        if parts.len() >= 6 && parts[3] == "FORMULA" {
                            let formula: i16 = parts[4].parse().unwrap_or(-1);
                            let parent1_ref = parts[5];
                            let parent2_ref = if parts.len() > 6 { parts[6] } else { "" };
                            
                            if formula != -1 {
                                // Get parent cell keys
                                let parent1_key = if !parent1_ref.is_empty() {
                                    if let Ok((p1_row, p1_col)) = parse_cell_reference(sheet, parent1_ref) {
                                        sheet.get_key(p1_row, p1_col)
                                    } else {
                                        -1
                                    }
                                } else {
                                    -1
                                };
                                
                                let parent2_key = if !parent2_ref.is_empty() {
                                    if let Ok((p2_row, p2_col)) = parse_cell_reference(sheet, parent2_ref) {
                                        sheet.get_key(p2_row, p2_col)
                                    } else {
                                        -1
                                    }
                                } else {
                                    -1
                                };
                                
                                // Set cell metadata
                                let key = sheet.get_key(row, col);
                                let meta = sheet.cell_meta.entry(key).or_insert_with(|| crate::spreadsheet::CellMeta::new());
                                meta.formula = formula;
                                meta.parent1 = parent1_key;
                                meta.parent2 = parent2_key;
                                
                                // Add dependencies
                                graph::add_children(sheet, parent1_key, parent2_key, formula, row, col);
                            }
                        }
                    }
                }
            },
            _ => continue,
        }
    }

    CommandStatus::CmdOk
}