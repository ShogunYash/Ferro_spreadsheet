use crate::spreadsheet::Spreadsheet;
use std::collections::HashSet;

pub fn add_children(sheet: &mut Spreadsheet, cell1: i32, cell2: i32, formula: i16, row: i16, col: i16) {
    let rem = formula % 10;
    let child_key = sheet.get_key(row, col);
    if formula == -1 {
        return;
    }
    
    if rem == 0 {
        sheet.add_child(&cell1, &child_key);
        sheet.add_child(&cell2, &child_key);
    }
    else if rem == 2 {
        sheet.add_child(&cell1, &child_key);
    }
    else if rem == 3 {
        sheet.add_child(&cell2, &child_key);
    }
    else {
        let (start_row, start_col) = sheet.get_row_col(cell1);
        let (end_row, end_col) = sheet.get_row_col(cell2);
        for i in start_row..=end_row {
            for j in start_col..=end_col {
                let parent_key = sheet.get_key(i, j);
                sheet.add_child(&parent_key, &child_key);
            }
        }
    }
}

pub fn remove_all_parents(sheet: &mut Spreadsheet, row: i16, col: i16) {
    // This removes the child row, col from its parent cells
    let child_key = sheet.get_key(row, col);
    
    // Get metadata for this cell
    if !sheet.cell_meta.contains_key(&child_key) {
        return; // No metadata, no parents to remove
    }
    
    let meta = match sheet.cell_meta.get(&child_key) {
        Some(meta) => meta,
        None => return, // No metadata, no parents to remove
    };
    if meta.formula == -1 {
        return;
    }
    
    let rem = (meta.formula % 10) as i16;
    
    if rem >= 5 && rem <= 9 {
        let (start_row, start_col) = sheet.get_row_col(meta.parent1);
        let (end_row, end_col) = sheet.get_row_col(meta.parent2);

        for i in start_row..=end_row {
            for j in start_col..=end_col {
                let parent_key = sheet.get_key(i, j);
                sheet.remove_child(parent_key, child_key);
            }
        }
    }

    else if rem == 0 {
        let parent1= meta.parent1;
        let parent2 = meta.parent2;
        sheet.remove_child(parent1, child_key);
        sheet.remove_child(parent2, child_key);
    }
    else if rem == 2 {
        sheet.remove_child(meta.parent1, child_key);
    }
    else if rem == 3 {
        sheet.remove_child(meta.parent2, child_key);
    }
}

pub fn detect_cycle(sheet: &Spreadsheet, parent1: i32, parent2: i32, formula: i16, target_key: i32) -> bool {
    let rem = formula % 10;
    
    let mut visited = HashSet::with_capacity(32);
    let mut stack = Vec::with_capacity(32); // Pre-allocate with initial size
    stack.push(target_key);
    
    // Extract coordinates for range operations if needed
    let mut start_row = 0;
    let mut start_col = 0;
    let mut end_row = 0;
    let mut end_col = 0;
    
    if rem >= 5 {
        let coords = sheet.get_row_col(parent1);
        start_row = coords.0;
        start_col = coords.1;
        
        let coords = sheet.get_row_col(parent2);
        end_row = coords.0;
        end_col = coords.1;
    }
    
    while let Some(key) = stack.pop() {
        // Skip if already visited
        if !visited.insert(key) {
            continue;
        }
        // Check conditions based on formula type
        if rem == 0 && (parent1 == key || parent2 == key) {
            return true;
        }
        else if rem == 2 && parent1 == key {
            return true;
        }
        else if rem == 3 && parent2 == key {
            return true;
        }
        else if rem >= 5 {
            let (row, col) = sheet.get_row_col(key);
            if start_row <= row && row <= end_row && start_col <= col && col <= end_col {
                return true;
            }
        }
        
        // Add children to stack for processing - updated for separate children HashMap
        if let Some(children) = sheet.get_cell_children(key) {
            // Check if we need to resize the stack capacity
            let required_capacity = stack.len() + children.len();
            if required_capacity > stack.capacity() {
                // Calculate needed capacity (at least double, but enough for all children)
                let mut new_capacity = stack.capacity() * 2;
                while new_capacity < required_capacity {
                    new_capacity *= 2;
                }
                stack.reserve(new_capacity - stack.capacity());
            }
            
            // Add all children to the stack
            for &child in children {
                stack.push(child);
            }
        }
    }
    false
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_add_remove_child() {
//         let mut cell = Cell::new();
//         add_child(&mut cell, 0, 1, 5);
//         assert!(cell.children.is_some());
//         remove_child(&mut cell, get_key(0, 1, 5));
//         assert!(cell.children.is_none());
//     }

//     #[test]
//     fn test_add_children() {
//         let mut sheet = Spreadsheet::create(5, 5).unwrap();
//         add_children(&mut sheet, get_key(0, 0, 5), get_key(1, 1, 5), 5, 2, 2); // Range formula
//         assert!(sheet.get_cell(0, 0).children.is_some());
//     }

//     #[test]
//     fn test_detect_cycle() {
//         let mut sheet = Spreadsheet::create(5, 5).unwrap();
//         sheet.get_mut_cell(0, 0).parent1 = get_key(0, 1, 5);
//         sheet.get_mut_cell(0, 1).parent1 = get_key(0, 0, 5);
//         assert!(detect_cycle(&sheet, get_key(0, 1, 5), -1, 82, get_key(0, 0, 5)));
//     }
// }