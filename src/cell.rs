use crate::spreadsheet::{CommandStatus, Spreadsheet};

/// Cell value representation
#[derive(Debug, Clone, PartialEq)]
pub enum CellValue {
    Integer(i32),
    Error,
}

pub fn parse_cell_reference(
    sheet: &Spreadsheet,
    cell_ref: &str,
) -> Result<(i16, i16), CommandStatus> {
    let cell_ref = cell_ref.as_bytes();
    if cell_ref.is_empty() {
        return Err(CommandStatus::CmdUnrecognized);
    }

    // Find column/row split point in one pass
    let mut split_idx = 0;
    let mut col_length = 0;

    while split_idx < cell_ref.len() && cell_ref[split_idx] >= b'A' && cell_ref[split_idx] <= b'Z' {
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
    let col_name =
        std::str::from_utf8(&cell_ref[0..split_idx]).map_err(|_| CommandStatus::CmdUnrecognized)?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spreadsheet::{CommandStatus, Spreadsheet};

    fn create_test_spreadsheet(rows: i16, cols: i16) -> Spreadsheet {
        Spreadsheet::create(rows, cols).unwrap()
    }

    #[test]
    fn test_parse_cell_reference_valid() {
        let sheet = create_test_spreadsheet(10, 10);
        assert_eq!(parse_cell_reference(&sheet, "A1"), Ok((0, 0)));
        assert_eq!(parse_cell_reference(&sheet, "B2"), Ok((1, 1)));
        assert_eq!(parse_cell_reference(&sheet, "AA10"), Ok((9, 26)));
        assert_eq!(parse_cell_reference(&sheet, "ZZZ999"), Ok((998, 18277)));
    }

    #[test]
    fn test_parse_cell_reference_invalid() {
        let sheet = create_test_spreadsheet(10, 10);
        assert_eq!(
            parse_cell_reference(&sheet, "1A"),
            Err(CommandStatus::CmdUnrecognized)
        );
        assert_eq!(
            parse_cell_reference(&sheet, "A"),
            Err(CommandStatus::CmdUnrecognized)
        );
        assert_eq!(
            parse_cell_reference(&sheet, "A1B"),
            Err(CommandStatus::CmdUnrecognized)
        );
        assert_eq!(
            parse_cell_reference(&sheet, "AAAA1"),
            Err(CommandStatus::CmdUnrecognized)
        );
        assert_eq!(
            parse_cell_reference(&sheet, ""),
            Err(CommandStatus::CmdUnrecognized)
        );
    }

    #[test]
    fn test_parse_cell_reference_bounds() {
        let sheet = create_test_spreadsheet(10, 10);
        assert_eq!(
            parse_cell_reference(&sheet, "A1000"),
            Err(CommandStatus::CmdUnrecognized)
        );
        assert_eq!(
            parse_cell_reference(&sheet, "ZZZ1000"),
            Err(CommandStatus::CmdUnrecognized)
        );
        assert_eq!(
            parse_cell_reference(&sheet, "A0"),
            Err(CommandStatus::CmdUnrecognized)
        );
    }

    #[test]
    fn test_cell_value_equality() {
        assert_eq!(CellValue::Integer(42), CellValue::Integer(42));
        assert_eq!(CellValue::Error, CellValue::Error);
        assert_ne!(CellValue::Integer(42), CellValue::Error);
    }
}
