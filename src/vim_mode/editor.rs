// vim_mode/editor.rs
use crate::spreadsheet::Spreadsheet;

// Define editor modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditorMode {
    Normal,
    Insert,
}

// Editor state structure
pub struct EditorState {
    pub mode: EditorMode,
    pub cursor_row: i16,
    pub cursor_col: i16,
    pub clipboard: Option<(i16, i16, String)>, // (row, col, value)
    pub should_quit: bool,
}

impl EditorState {
    pub fn new() -> Self {
        EditorState {
            mode: EditorMode::Normal,
            cursor_row: 0,  // Start at row 0 (first row)
            cursor_col: 0,  // Start at col 0 (first column)
            clipboard: None,
            should_quit: false,
        }
    }
    
    pub fn mode_display(&self) -> &'static str {
        match self.mode {
            EditorMode::Normal => "NORMAL",
            EditorMode::Insert => "INSERT",
        }
    }
    
    // Move cursor in the specified direction
    pub fn move_cursor(&mut self, direction: char, sheet: &Spreadsheet) {
        match direction {
            'h' => if self.cursor_col > 0 { self.cursor_col -= 1 },
            'j' => if self.cursor_row < sheet.rows - 1 { self.cursor_row += 1 },
            'u' => if self.cursor_row > 0 { self.cursor_row -= 1 },
            'l' => if self.cursor_col < sheet.cols - 1 { self.cursor_col += 1 },
            _ => {}
        }
        
        // Make sure cursor is within viewport boundaries
        self.ensure_viewport_contains_cursor(sheet);
    }
    
    // Adjust spreadsheet viewport if needed to show cursor
    fn ensure_viewport_contains_cursor(&self, sheet: &Spreadsheet) {
        // This is just a placeholder - the actual implementation would
        // depend on how your spreadsheet viewport is managed
        // If the spreadsheet already manages its own viewport based on
        // user commands, you might not need to modify it here
    }
    
    // Custom rendering function for vim mode
    pub fn render_spreadsheet(&self, sheet: &Spreadsheet) {
        // Clear screen
        print!("\x1B[2J\x1B[1;1H");
        
        // Let the spreadsheet render itself
        // We'll rely on the existing print_spreadsheet functionality,
        // but might extend it in the future to show cursor position
        sheet.print_spreadsheet();
        
        // Display current cell info
        let col_letter = (b'A' + self.cursor_col as u8) as char;
        println!("\nCursor at: {}{}", col_letter, self.cursor_row + 1);
        
        // Additional status info can be added here
    }
    
    // Convert cursor position to cell reference string (e.g., "A1")
    pub fn cursor_to_cell_ref(&self) -> String {
        let col_letter = (b'A' + self.cursor_col as u8) as char;
        format!("{}{}", col_letter, self.cursor_row + 1)
    }
}