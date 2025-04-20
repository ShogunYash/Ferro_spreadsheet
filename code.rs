
pub fn handle_command(
    sheet: &mut Spreadsheet,
    trimmed: &str,
    sleep_time: &mut f64,
) -> CommandStatus {    
    // Fast path for single-character commands to avoid string comparisons
    if trimmed.len() == 1 {
        match trimmed.as_bytes()[0] {
            b'w' | b'a' | b's' | b'd' => {
                // We've already validated it's one byte, so this is safe
                let direction = trimmed.chars().next().unwrap();
                sheet.scroll_viewport(direction);
                return CommandStatus::CmdOk;
            },
            b'q' => return CommandStatus::CmdOk, // Handle quit command if needed
            _ => {}
        }
    }
    
    // Use match for special commands for better branch prediction
    match trimmed {
        "disable_output" => {
            sheet.output_enabled = false;
            return CommandStatus::CmdOk;
        },
        "enable_output" => {
            sheet.output_enabled = true;
            return CommandStatus::CmdOk;
        },
        _ => {}
    }
    
    // Check for cell dependency visualization command
    if trimmed.starts_with("visualize ") {
        let cell_ref = &trimmed[10..]; // Skip "visualize " prefix
        match parse_cell_reference(sheet, cell_ref) {
            Ok((row, col)) => {
                return sheet.visualize_cell_relationships(row, col);
            },
            Err(status) => {
                return status;
            }
        }
    }
    
    // Check for scroll_to command with byte-based comparison
    if trimmed.len() > 10 && &trimmed.as_bytes()[..9] == b"scroll_to" && trimmed.as_bytes()[9] == b' ' {
        let cell_ref = &trimmed[10..];
        return sheet.scroll_to_cell(cell_ref);
    }
    
    // Check for cell assignment using byte search for '='
    let bytes = trimmed.as_bytes();
    let mut eq_pos = None;
    
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'=' {
            eq_pos = Some(i);
            break;
        }
    }
    
    if let Some(pos) = eq_pos {
        // Use slice operations which are more efficient than split_at
        let cell_ref = trimmed[..pos].trim();
        let expr = trimmed[pos+1..].trim();
        
        // Parse the cell reference with direct result handling
        return match parse_cell_reference(sheet, cell_ref) {
            Ok((row, col)) => {
                // All bounds checks in one condition
                set_cell_value(sheet, row, col, expr, sleep_time)
            },
            Err(status) => status,
        };
    }
    // No recognized command
    CommandStatus::CmdUnrecognized
}
pub struct RangeChild {
    pub start_key: i32,       // Range start cell key
    pub end_key: i32,         // Range end cell key
    pub child_key: i32,       // Child cell key
}

#[derive(Debug, PartialEq)]
pub enum CommandStatus {
    CmdOk,
    CmdUnrecognized,
    CmdCircularRef,
    CmdInvalidCell,
}

// Modified CellMeta to remove children (they're now stored separately)
#[derive(Debug, Clone)]
pub struct CellMeta {
    pub formula: i16,
    pub parent1: i32,
    pub parent2: i32,
}

impl CellMeta {
    pub fn new() -> Self {
        CellMeta {
            formula: -1,
            parent1: -1,
            parent2: -1,
        }
    }
}

impl Default for CellMeta {
    fn default() -> Self {
        CellMeta {
            formula: -1,
            parent1: -1,
            parent2: -1,
        }
    }
}

// Spreadsheet structure with HashMap of boxed HashSets for children
pub struct Spreadsheet {
    pub grid: Vec<CellValue>,                                // Vector of CellValues (contiguous in memory)
    pub children: HashMap<i32, Box<HashSet<i32>>>,           // Map from cell key to boxed HashSet of children
    pub range_children: Vec<RangeChild>,                     // Vector of range-based child relationships
    pub cell_meta: HashMap<i32, CellMeta>,                   // Map from cell key to metadata
    pub rows: i16,
    pub cols: i16,
    pub viewport_row: i16,
    pub viewport_col: i16,
    pub output_enabled: bool,
}

