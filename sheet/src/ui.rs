use crate::spreadsheet::Spreadsheet;

const MAX_DISPLAY_ROWS: usize = 10;
const MAX_DISPLAY_COLS: usize = 10;

pub fn display_sheet(sheet: &Spreadsheet) {
    let view_row = sheet.get_view_row();
    let view_col = sheet.get_view_col();
    
    // Calculate actual number of rows and columns to display
    let display_rows = std::cmp::min(MAX_DISPLAY_ROWS, sheet.get_rows() - view_row);
    let display_cols = std::cmp::min(MAX_DISPLAY_COLS, sheet.get_cols() - view_col);
    
    if display_rows == 0 || display_cols == 0 {
        return;
    }
    
    // Print column headers
    print!("  "); // Space for row headers
    for c in 0..display_cols {
        let col_idx = view_col + c;
        print_column_header(col_idx);
        print!(" ");
    }
    println!();
    
    // Print rows
    for r in 0..display_rows {
        let row_idx = view_row + r;
        
        // Print row header (1-indexed)
        print!("{} ", row_idx + 1);
        
        // Print cells
        for c in 0..display_cols {
            let col_idx = view_col + c;
            
            if let Some(cell) = sheet.get_cell(row_idx, col_idx) {
                print!("{} ", cell);
            } else {
                print!("? "); // Should not happen
            }
        }
        println!();
    }
}

// Helper function to print column headers (A, B, ..., Z, AA, AB, ...)
fn print_column_header(col_idx: usize) {
    let mut col_val = col_idx + 1; // Convert to 1-based for calculation
    let mut col_str = String::new();
    
    while col_val > 0 {
        let remainder = (col_val - 1) % 26;
        col_str.insert(0, (b'A' + remainder as u8) as char);
        col_val = (col_val - remainder) / 26;
        if col_val == 1 && remainder == 0 {
            break;
        }
    }
    
    print!("{}", col_str);
}