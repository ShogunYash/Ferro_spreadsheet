use crate::spreadsheet::Spreadsheet;

/// Generates a string representation of a cell’s formula.
///
/// # Arguments
///
/// * `sheet` - The spreadsheet.
/// * `row` - The cell’s row.
/// * `col` - The cell’s column.
///
/// # Returns
///
/// A string like "A1+B1" or "SUM(A1:B2)", or "No formula" if none
pub fn get_formula_string(sheet: &Spreadsheet, row: i16, col: i16) -> String {
    let meta = sheet.get_cell_meta_ref(row, col);
    if meta.formula == -1 {
        return "No formula".to_string();
    }
    let rem = meta.formula % 10;
    let msb = meta.formula / 10;
    let parent1 = meta.parent1;
    let parent2 = meta.parent2;

    match rem {
        0 => {
            let (left, right) = {
                let (left_row, left_col) = sheet.get_row_col(parent1);
                let (right_row, right_col) = sheet.get_row_col(parent2);
                let left_name = sheet.get_cell_name(left_row, left_col);
                let right_name = sheet.get_cell_name(right_row, right_col);
                (left_name, right_name)
            };
            match msb {
                1 => format!("{}+{}", left, right),
                2 => format!("{}-{}", left, right),
                3 => format!("{}/{}", left, right),
                _ => format!("{}*{}", left, right),
            }
        }
        2 => {
            let (left, right) = {
                let (left_row, left_col) = sheet.get_row_col(parent1);
                let left_name = sheet.get_cell_name(left_row, left_col);
                (left_name, parent2.to_string())
            };
            match msb {
                1 => format!("{}+{}", left, right),
                2 => format!("{}-{}", left, right),
                4 => format!("{}*{}", left, right),
                3 => format!("{}/{}", left, right),
                8 => left.to_string(),
                _ => format!("SLEEP({})", left),
            }
        }
        3 => {
            let (left, right) = {
                let (right_row, right_col) = sheet.get_row_col(parent2);
                let right_name = sheet.get_cell_name(right_row, right_col);
                (parent1.to_string(), right_name)
            };
            match msb {
                1 => format!("{}+{}", left, right),
                2 => format!("{}-{}", left, right),
                3 => format!("{}/{}", left, right),
                _ => format!("{}*{}", left, right),
            }
        }
        5 => {
            let (start_row, start_col) = sheet.get_row_col(parent1);
            let (end_row, end_col) = sheet.get_row_col(parent2);
            let start_name = sheet.get_cell_name(start_row, start_col);
            let end_name = sheet.get_cell_name(end_row, end_col);
            format!("SUM({}:{})", start_name, end_name)
        }
        6 => {
            let (start_row, start_col) = sheet.get_row_col(parent1);
            let (end_row, end_col) = sheet.get_row_col(parent2);
            let start_name = sheet.get_cell_name(start_row, start_col);
            let end_name = sheet.get_cell_name(end_row, end_col);
            format!("AVG({}:{})", start_name, end_name)
        }
        7 => {
            let (start_row, start_col) = sheet.get_row_col(parent1);
            let (end_row, end_col) = sheet.get_row_col(parent2);
            let start_name = sheet.get_cell_name(start_row, start_col);
            let end_name = sheet.get_cell_name(end_row, end_col);
            format!("MIN({}:{})", start_name, end_name)
        }
        8 => {
            let (start_row, start_col) = sheet.get_row_col(parent1);
            let (end_row, end_col) = sheet.get_row_col(parent2);
            let start_name = sheet.get_cell_name(start_row, start_col);
            let end_name = sheet.get_cell_name(end_row, end_col);
            format!("MAX({}:{})", start_name, end_name)
        }
        9 => {
            let (start_row, start_col) = sheet.get_row_col(parent1);
            let (end_row, end_col) = sheet.get_row_col(parent2);
            let start_name = sheet.get_cell_name(start_row, start_col);
            let end_name = sheet.get_cell_name(end_row, end_col);
            format!("STDEV({}:{})", start_name, end_name)
        }
        _ => "Unknown formula".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save_load::{load_spreadsheet, save_spreadsheet};
    use crate::spreadsheet::Spreadsheet;
    use std::fs::{self, File};
    use std::io::Write;

    #[test]
    fn test_get_formula_string_basic_operations() {
        let mut sheet = Spreadsheet::create(10, 10).unwrap();

        // Test subtraction (A1 - B2)
        let a1_pos = sheet.get_key(0, 0);
        let b2_pos = sheet.get_key(1, 1);
        let meta = sheet.get_cell_meta(0, 1);
        meta.formula = 20; // 2 (subtraction) * 10 + 0 (both are cell refs)
        meta.parent1 = a1_pos;
        meta.parent2 = b2_pos;
        assert_eq!(get_formula_string(&sheet, 0, 1), "A1-B2");

        // Test multiplication (A1 * 5)
        let meta = sheet.get_cell_meta(0, 2);
        meta.formula = 43; // 4 (multiplication) * 10 + 3 (first is cell, second is literal)
        meta.parent1 = 5;
        meta.parent2 = a1_pos;
        assert_eq!(get_formula_string(&sheet, 0, 2), "5*A1");

        // Test division (10 / B2)
        let meta = sheet.get_cell_meta(0, 3);
        meta.formula = 32; // 3 (division) * 10 + 2 (first is literal, second is cell)
        meta.parent1 = b2_pos;
        meta.parent2 = 10;
        assert_eq!(get_formula_string(&sheet, 0, 3), "B2/10");
    }

    #[test]
    fn test_get_formula_string_functions() {
        let mut sheet = Spreadsheet::create(10, 10).unwrap();

        let a1_pos = sheet.get_key(0, 0);
        let c3_pos = sheet.get_key(2, 2);

        // Test SUM function
        let meta = sheet.get_cell_meta(1, 1);
        meta.formula = 5; // Range function with code 5 for SUM
        meta.parent1 = a1_pos;
        meta.parent2 = c3_pos;
        assert_eq!(get_formula_string(&sheet, 1, 1), "SUM(A1:C3)");

        // Test AVG function
        let meta = sheet.get_cell_meta(1, 2);
        meta.formula = 6; // Range function with code 6 for AVG
        meta.parent1 = a1_pos;
        meta.parent2 = c3_pos;
        assert_eq!(get_formula_string(&sheet, 1, 2), "AVG(A1:C3)");

        // Test MIN function
        let meta = sheet.get_cell_meta(1, 3);
        meta.formula = 7; // Range function with code 7 for MIN
        meta.parent1 = a1_pos;
        meta.parent2 = c3_pos;
        assert_eq!(get_formula_string(&sheet, 1, 3), "MIN(A1:C3)");

        // Test MAX function
        let meta = sheet.get_cell_meta(1, 4);
        meta.formula = 8; // Range function with code 8 for MAX
        meta.parent1 = a1_pos;
        meta.parent2 = c3_pos;
        assert_eq!(get_formula_string(&sheet, 1, 4), "MAX(A1:C3)");

        // Test STDEV function
        let meta = sheet.get_cell_meta(1, 5);
        meta.formula = 9; // Range function with code 9 for STDEV
        meta.parent1 = a1_pos;
        meta.parent2 = c3_pos;
        assert_eq!(get_formula_string(&sheet, 1, 5), "STDEV(A1:C3)");
    }

    #[test]
    fn test_get_formula_string_special_cases() {
        let mut sheet = Spreadsheet::create(10, 10).unwrap();

        let a1_pos = sheet.get_key(0, 0);

        // Test cell reference
        let meta = sheet.get_cell_meta(2, 0);
        meta.formula = 82; // Special formula for cell reference
        meta.parent1 = a1_pos;
        meta.parent2 = -1;
        assert_eq!(get_formula_string(&sheet, 2, 0), "A1");

        // Test SLEEP function
        let meta = sheet.get_cell_meta(2, 1);
        meta.formula = 102; // Special formula for SLEEP
        meta.parent1 = a1_pos;
        meta.parent2 = -1;
        assert_eq!(get_formula_string(&sheet, 2, 1), "SLEEP(A1)");

        // Test direct SLEEP function with code 102
        let meta = sheet.get_cell_meta(2, 2);
        meta.formula = 102;
        meta.parent1 = a1_pos;
        meta.parent2 = -1;
        assert_eq!(get_formula_string(&sheet, 2, 2), "SLEEP(A1)");
    }

    #[test]
    fn test_get_formula_string_invalid_cases() {
        let mut sheet = Spreadsheet::create(10, 10).unwrap();

        // Test no formula
        let meta = sheet.get_cell_meta(3, 0);
        meta.formula = -1;
        meta.parent1 = -1;
        meta.parent2 = -1;
        assert_eq!(get_formula_string(&sheet, 3, 0), "No formula");
    }

    #[test]
    fn test_save_spreadsheet_invalid_path() {
        let sheet = Spreadsheet::create(5, 5).unwrap();
        // Try to save to an invalid path (should fail)
        let status = save_spreadsheet(&sheet, "/invalid_path/should_fail.sheet");
        assert_eq!(status, crate::spreadsheet::CommandStatus::CmdUnrecognized);
    }

    #[test]
    fn test_load_spreadsheet_nonexistent_file() {
        let mut sheet = Spreadsheet::create(5, 5).unwrap();
        // Try to load a file that doesn't exist
        let status = load_spreadsheet(&mut sheet, "this_file_should_not_exist_123456.sheet");
        assert_eq!(status, crate::spreadsheet::CommandStatus::CmdUnrecognized);
    }

    #[test]
    fn test_load_spreadsheet_invalid_lines() {
        let mut sheet = Spreadsheet::create(5, 5).unwrap();
        let filename = "test_invalid_lines.sheet";
        let mut file = File::create(filename).unwrap();
        // Write invalid lines
        writeln!(file, "INVALID_LINE").unwrap();
        writeln!(file, "CELL,A1,ERR").unwrap();
        writeln!(file, "CELL,A1,notanumber").unwrap();
        writeln!(file, "CELL,ZZZ,42").unwrap(); // Invalid cell ref
        writeln!(file, "CELL,A1,42,FORMULA,notanumber,A1,B1").unwrap(); // Invalid formula code
        writeln!(file, "DIMS,notanumber,notanumber").unwrap(); // Invalid DIMS
        file.flush().unwrap();

        // Should not panic, should continue on errors
        let status = load_spreadsheet(&mut sheet, filename);
        assert_eq!(status, crate::spreadsheet::CommandStatus::CmdOk);

        // Clean up
        let _ = fs::remove_file(filename);
    }

    #[test]
    fn test_load_spreadsheet_invalid_dims_warning() {
        let mut sheet = Spreadsheet::create(2, 2).unwrap();
        let filename = "test_invalid_dims.sheet";
        let mut file = File::create(filename).unwrap();
        // Write a DIMS line with larger dims than sheet
        writeln!(file, "DIMS,10,10").unwrap();
        file.flush().unwrap();

        // Should print a warning but still succeed
        let status = load_spreadsheet(&mut sheet, filename);
        assert_eq!(status, crate::spreadsheet::CommandStatus::CmdOk);

        // Clean up
        let _ = fs::remove_file(filename);
    }

    #[test]
    fn test_load_spreadsheet_formula_parent_refs_empty() {
        let mut sheet = Spreadsheet::create(5, 5).unwrap();
        let filename = "test_formula_parent_refs_empty.sheet";
        let mut file = File::create(filename).unwrap();
        // parent1_ref and parent2_ref are empty
        writeln!(file, "CELL,A1,42,FORMULA,10,,").unwrap();
        file.flush().unwrap();

        let status = load_spreadsheet(&mut sheet, filename);
        assert_eq!(status, crate::spreadsheet::CommandStatus::CmdOk);

        // Clean up
        let _ = fs::remove_file(filename);
    }

    #[test]
    fn test_load_spreadsheet_formula_parent_refs_parse_fail() {
        let mut sheet = Spreadsheet::create(5, 5).unwrap();
        let filename = "test_formula_parent_refs_parse_fail.sheet";
        let mut file = File::create(filename).unwrap();
        // parent1_ref and parent2_ref are invalid (parse_cell_reference fails)
        writeln!(file, "CELL,A1,42,FORMULA,10,INVALID1,INVALID2").unwrap();
        file.flush().unwrap();

        let status = load_spreadsheet(&mut sheet, filename);
        assert_eq!(status, crate::spreadsheet::CommandStatus::CmdOk);

        // Clean up
        let _ = fs::remove_file(filename);
    }

    #[test]
    fn test_load_spreadsheet_formula_parent_refs_out_of_bounds() {
        let mut sheet = Spreadsheet::create(5, 5).unwrap();
        let filename = "test_formula_parent_refs_out_of_bounds.sheet";
        let mut file = File::create(filename).unwrap();
        // parent1_ref and parent2_ref are valid format but out of bounds
        writeln!(file, "CELL,A1,42,FORMULA,10,Z10,Y20").unwrap(); // Z10 and Y20 are out of 5x5
        file.flush().unwrap();

        let status = load_spreadsheet(&mut sheet, filename);
        assert_eq!(status, crate::spreadsheet::CommandStatus::CmdOk);

        // Clean up
        let _ = fs::remove_file(filename);
    }
}
