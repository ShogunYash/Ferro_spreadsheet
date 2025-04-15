use crate::spreadsheet::Spreadsheet;

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

// pub fn detect_cycle(sheet: &Spreadsheet, parent1: i32, parent2: i32, formula: i16, target_key: i32) -> bool {
//     let rem = formula % 10;
    
//     let mut visited = HashSet::with_capacity(32);
//     let mut stack = Vec::with_capacity(32); // Pre-allocate with initial size
//     stack.push(target_key);
    
//     // Extract coordinates for range operations if needed
//     let mut start_row = 0;
//     let mut start_col = 0;
//     let mut end_row = 0;
//     let mut end_col = 0;
    
//     if rem >= 5 {
//         let coords = sheet.get_row_col(parent1);
//         start_row = coords.0;
//         start_col = coords.1;
        
//         let coords = sheet.get_row_col(parent2);
//         end_row = coords.0;
//         end_col = coords.1;
//     }
    
//     while let Some(key) = stack.pop() {
//         // Skip if already visited
//         if !visited.insert(key) {
//             continue;
//         }
//         // Check conditions based on formula type
//         if rem == 0 && (parent1 == key || parent2 == key) {
//             return true;
//         }
//         else if rem == 2 && parent1 == key {
//             return true;
//         }
//         else if rem == 3 && parent2 == key {
//             return true;
//         }
//         else if rem >= 5 {
//             let (row, col) = sheet.get_row_col(key);
//             if start_row <= row && row <= end_row && start_col <= col && col <= end_col {
//                 return true;
//             }
//         }
        
//         // Add children to stack for processing - updated for separate children HashMap
//         if let Some(children) = sheet.get_cell_children(key) {
//             // Check if we need to resize the stack capacity
//             let required_capacity = stack.len() + children.len();
//             if required_capacity > stack.capacity() {
//                 // Calculate needed capacity (at least double, but enough for all children)
//                 let mut new_capacity = stack.capacity() * 2;
//                 while new_capacity < required_capacity {
//                     new_capacity *= 2;
//                 }
//                 stack.reserve(new_capacity - stack.capacity());
//             }
            
//             // Add all children to the stack
//             for &child in children {
//                 stack.push(child);
//             }
//         }
//     }
//     false
// }

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::cell::CellValue;
    
//     #[test]
//     fn test_remove_all_parents() {
//         let mut sheet = Spreadsheet::create(5, 5).unwrap();
        
//         // Set up parent-child relationships
//         let meta = sheet.get_cell_meta(0, 0);
//         meta.parent1 = sheet.get_key(1, 1);
//         meta.parent2 = sheet.get_key(2, 2);
        
//         add_children(&mut sheet, meta.parent1, meta.parent2, 5, 0, 0);
        
//         // Verify children are set up correctly
//         assert!(sheet.get_cell_children(meta.parent1).unwrap().contains(&sheet.get_key(0, 0)));
//         assert!(sheet.get_cell_children(meta.parent2).unwrap().contains(&sheet.get_key(0, 0)));
        
//         // Remove all parents
//         remove_all_parents(&mut sheet, 0, 0);
        
//         // Verify children are removed
//         assert!(sheet.get_cell_children(meta.parent1).is_none() || 
//                !sheet.get_cell_children(meta.parent1).unwrap().contains(&sheet.get_key(0, 0)));
        
//         assert!(sheet.get_cell_children(meta.parent2).is_none() || 
//                !sheet.get_cell_children(meta.parent2).unwrap().contains(&sheet.get_key(0, 0)));
//     }
    
//     #[test]
//     fn test_add_children() {
//         let mut sheet = Spreadsheet::create(5, 5).unwrap();
        
//         // Add a child with single parent
//         add_children(&mut sheet, sheet.get_key(1, 1), -1, 82, 0, 0);
//         assert!(sheet.get_cell_children(sheet.get_key(1, 1)).unwrap().contains(&sheet.get_key(0, 0)));
        
//         // Add a child with range parents (SUM formula)
//         add_children(&mut sheet, sheet.get_key(2, 2), sheet.get_key(3, 3), 5, 0, 1);
        
//         // Verify all cells in the range have the child
//         for r in 2..=3 {
//             for c in 2..=3 {
//                 assert!(sheet.get_cell_children(sheet.get_key(r, c)).unwrap().contains(&sheet.get_key(0, 1)));
//             }
//         }
//     }
    
//     #[test]
//     fn test_detect_cycle() {
//         let mut sheet = Spreadsheet::create(5, 5).unwrap();
        
//         // Set up a dependency chain: A1 -> B1 -> C1
//         *sheet.get_mut_cell(0, 0) = CellValue::Integer(1);  // A1 = 1
//         *sheet.get_mut_cell(0, 1) = CellValue::Integer(2);  // B1 = 2
        
//         let a1_key = sheet.get_key(0, 0);
//         let b1_key = sheet.get_key(0, 1);
//         let c1_key = sheet.get_key(0, 2);
        
//         // B1 depends on A1
//         add_children(&mut sheet, a1_key, -1, 82, 0, 1);
        
//         // C1 depends on B1
//         add_children(&mut sheet, b1_key, -1, 82, 0, 2);
        
//         // No cycle yet
//         assert!(!detect_cycle(&sheet, a1_key, -1, 82, c1_key));
        
//         // This would create a cycle: A1 depends on C1
//         assert!(detect_cycle(&sheet, c1_key, -1, 82, a1_key));
//     }
// }