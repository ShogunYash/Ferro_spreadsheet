// Optimized add_children function for better performance with large ranges
pub fn add_children(sheet: &mut Spreadsheet, parent1: i32, parent2: i32, formula: i16, row: i16, col: i16) {
    let child_key = sheet.get_key(row, col);
    
    // Fast path: If no valid parents, nothing to do
    if parent1 < 0 && parent2 < 0 {
        return;
    }
    
    // Handle parent1 (single cell reference)
    if parent1 >= 0 {
        sheet.add_child(&parent1, &child_key);
    }
    
    // Handle parent2 (could be the end of a range)
    if parent2 >= 0 && parent2 != parent1 {
        // For range-based formulas (MIN/MAX/SUM/AVG/STDEV)
        if formula >= 5 && formula <= 9 {
            // This is a range-based formula (parent1 is start, parent2 is end)
            let (start_row, start_col) = sheet.get_row_col(parent1);
            let (end_row, end_col) = sheet.get_row_col(parent2);
            
            // Pre-allocate a vector to hold all keys in the range to avoid repeated map lookups
            let approx_capacity = ((end_row - start_row + 1) * (end_col - start_col + 1)) as usize;
            
            // Use a more efficient approach for very large ranges
            if approx_capacity > 1000 {
                // For extremely large ranges, use a bulk approach
                // Pre-allocate a HashMap with capacity for all cells in the range
                let mut keys_to_add = Vec::with_capacity(min(approx_capacity, 10000));
                
                // Generate all keys in the range (limit to a reasonable batch size)
                for r in start_row..=end_row {
                    for c in start_col..=end_col {
                        if keys_to_add.len() >= 10000 {
                            // Process in batches to avoid excessive memory usage
                            for &parent_key in &keys_to_add {
                                sheet.add_child(&parent_key, &child_key);
                            }
                            keys_to_add.clear();
                        }
                        
                        let parent_key: i32 = sheet.get_key(r, c);
                        keys_to_add.push(parent_key);
                    }
                }
                
                // Process any remaining keys
                for &parent_key in &keys_to_add {
                    sheet.add_child(&parent_key, &child_key);
                }
            } else {
                // For smaller ranges, use direct iteration
                for r in start_row..=end_row {
                    for c in start_col..=end_col {
                        let parent_key = sheet.get_key(r, c);
                        sheet.add_child(&parent_key, &child_key);
                    }
                }
            }
        } else {
            // Non-range formula, just add parent2
            sheet.add_child(&parent2, &child_key);
        }
    }
}