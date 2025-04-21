use crate::spreadsheet::Spreadsheet;

pub fn add_children(sheet: &mut Spreadsheet, cell1: i32, cell2: i32, formula: i16, row: i16, col: i16) {
    if formula == -1 {
        return;
    }
    let rem = formula % 10;
    let child_key = sheet.get_key(row, col);
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
        // For range operations, use the optimized range_children structure
        sheet.add_range_child(cell1, cell2, child_key);
    }
}

pub fn remove_all_parents(sheet: &mut Spreadsheet, row: i16, col: i16) {
    // This removes the child row, col from its parent cells
    let child_key = sheet.get_key(row, col);
    
    let meta = match sheet.cell_meta.get(&child_key) {
        Some(meta) => meta,
        None => return, // No metadata, no parents to remove
    };
    
    if meta.formula == -1 {
        return;
    }
    
    let rem: i16 = (meta.formula % 10) as i16;
    
    if rem >= 5 && rem <= 9 {
        // Use the optimized range_children removal for range operations
        sheet.remove_range_child(child_key);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spreadsheet::Spreadsheet;

    fn create_test_spreadsheet(rows: i16, cols: i16) -> Spreadsheet {
        Spreadsheet::create(rows, cols).unwrap()
    }

    #[test]
    fn test_add_children_both_parents() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let parent1 = sheet.get_key(0, 0);
        let parent2 = sheet.get_key(0, 1);
        let child = sheet.get_key(1, 1);
        add_children(&mut sheet, parent1, parent2, 0, 1, 1); // Formula type 0
        assert!(sheet.get_cell_children(parent1).unwrap().contains(&child));
        assert!(sheet.get_cell_children(parent2).unwrap().contains(&child));
    }

    #[test]
    fn test_add_children_single_parent() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let parent1 = sheet.get_key(0, 0);
        let child = sheet.get_key(1, 1);
        add_children(&mut sheet, parent1, -1, 2, 1, 1); // Formula type 2
        assert!(sheet.get_cell_children(parent1).unwrap().contains(&child));
        assert!(sheet.get_cell_children(-1).is_none());
    }

    #[test]
    fn test_add_children_range() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let parent1 = sheet.get_key(0, 0);
        let parent2 = sheet.get_key(1, 1);
        add_children(&mut sheet, parent1, parent2, 5, 2, 2); // Formula type 5 (SUM)
    }

    #[test]
    fn test_remove_all_parents_no_meta() {
        let mut sheet = create_test_spreadsheet(5, 5);
        remove_all_parents(&mut sheet, 1, 1);
        assert!(!sheet.cell_meta.contains_key(&sheet.get_key(1, 1)));
    }

    #[test]
    fn test_remove_all_parents_single_parent() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let parent1 = sheet.get_key(0, 0);
        let child = sheet.get_key(1, 1);
        add_children(&mut sheet, parent1, -1, 2, 1, 1); 
        let meta = sheet.get_cell_meta(1, 1); 
        meta.parent1 = parent1;
        meta.formula = 2; 
        remove_all_parents(&mut sheet, 1, 1);
        let children = sheet.get_cell_children(parent1);
        assert!(children.is_none() || !children.unwrap().contains(&child));
    }

    #[test]
    fn test_remove_all_parents_range() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let parent1 = sheet.get_key(0, 0);
        let parent2 = sheet.get_key(1, 1);
        add_children(&mut sheet, parent1, parent2, 5, 2, 2); // adds range child
        let meta = sheet.get_cell_meta(2, 2); 
        meta.parent1 = parent1;
        meta.parent2 = parent2;
        meta.formula = 5;
        remove_all_parents(&mut sheet, 2, 2);
    }

    #[test]
    fn test_add_children_invalid_formula() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let parent1 = sheet.get_key(0, 0);
        add_children(&mut sheet, parent1, -1, -1, 1, 1);
        let child_key = sheet.get_key(1, 1);
        assert!(sheet.get_cell_children(parent1).is_none());
    }

    #[test]
    fn test_remove_all_parents_no_formula() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let meta = sheet.get_cell_meta(1, 1);
        meta.formula = -1;
        remove_all_parents(&mut sheet, 1, 1);
        assert!(sheet.range_children.is_empty());
    }
    }