use crate::cell::Cell;
use crate::spreadsheet::Spreadsheet;
use crate::linked_list::Node;
use crate::evaluator::{get_cell_from_key, get_key};

pub fn remove_child(parent: &mut Cell, key: i32) {
    Node::remove(&mut parent.children, key);
}

pub fn add_child(parent: &mut Cell, row: i16, col:i16, cols:i16) {
    let key = get_key(row, col, cols);
    parent.children = Node::prepend(parent.children.clone(), key);
}

pub fn get_key_row_col(key: i32, cols: i16, row: &mut i16, col: &mut i16) {
    *row = (key / cols as i32) as i16;
    *col = (key % cols as i32) as i16;
}

pub fn add_children(sheet:&mut Spreadsheet, cell1: i32, cell2:i32 , formula:i16 , row :i16, col:i16)   {
    let rem = formula %10 as i16;
    let cols = sheet.cols;
    if formula== -1 {
        return;
    }   
    if rem == 0{
        let ref_cell1: &mut Cell = get_cell_from_key(sheet , cell1);
        add_child(ref_cell1, row, col, cols);
        let ref_cell2= get_cell_from_key(sheet,cell2);
        add_child(ref_cell2, row, col, cols);
    }
    else if rem == 2{
        let ref_cell1 = get_cell_from_key(sheet , cell1);
        let cols= cols;
        add_child(ref_cell1, row, col, cols);
    }
    else if rem == 3{
        let ref_cell2 = get_cell_from_key(sheet , cell2);
        let cols= cols;
        add_child(ref_cell2, row, col, cols);
    }
    else {
        let start_row = (cell1/(sheet.cols as i32)) as i16;
        let start_col = (cell1%(sheet.cols as i32) )as i16;
        let end_row = (cell2/(sheet.cols as i32)) as i16;
        let end_col = (cell2%(sheet.cols as i32)) as i16;
        for i in start_row..=end_row {
            for j in start_col..=end_col {
                let ref_cell= sheet.get_mut_cell(i, j);
                add_child(ref_cell, row, col, cols);
            }
        }
    }
}

pub fn remove_all_parents(sheet: &mut Spreadsheet, row: i16, col: i16){
    let cols = sheet.cols;
    let child = sheet.get_mut_cell(row, col);
    if child.formula == -1 {
        return;
    }
    let child_key = get_key(row, col, cols);
    let rem = (child.formula%10) as i16;
    if child.formula <= 9 && child.formula >= 5 {
        let start_row = (child.parent1/(cols as i32)) as i16;
        let start_col = (child.parent1%(cols as i32)) as i16;
        let end_row = (child.parent2/(cols as i32)) as i16;
        let end_col = (child.parent2%(cols as i32)) as i16;
        for i in start_row..=end_row {
            for j in start_col..=end_col {
                    let ref_cell = sheet.get_mut_cell(i, j);
                    remove_child(ref_cell, child_key);
            }
        }
    }
    else if rem == 0 {
        let mut row: i16 = 0;
        let mut col: i16 = 0;
        let parent1 = child.parent1;
        let parent2 = child.parent2;
        get_key_row_col(parent1, cols, &mut row, &mut col);
        let ref_cell1 = sheet.get_mut_cell(row, col);
        remove_child(ref_cell1, parent1);
        get_key_row_col(parent2, cols, &mut row, &mut col);
        let ref_cell2 = sheet.get_mut_cell(row, col);
        remove_child(ref_cell2, parent2);
    }
    else if rem == 2 {
        let mut row: i16 = 0;
        let mut col: i16 = 0;
        let parent1 = child.parent1;
        get_key_row_col(parent1, cols, &mut row, &mut col);
        let ref_cell1 = sheet.get_mut_cell(row, col);
        remove_child(ref_cell1, parent1);
    }
    else if rem == 3{
        let mut row: i16 = 0;
        let mut col: i16 = 0;
        let parent2 = child.parent2;
        get_key_row_col(parent2, cols, &mut row, &mut col);
        let ref_cell2 = sheet.get_mut_cell(row, col);
        remove_child(ref_cell2, parent2);
    }
}

// Convert the linked list to a vector for testing or debugging
pub fn add_linkedlist_stack(stack: &mut Vec<i32>, head: &Option<Box<Node>>) {
    let mut current = head;
    while let Some(node) = current {
        stack.push(node.key);
        current = &node.next;
    }
}

pub fn detect_cycle(sheet: &Spreadsheet, parent1: i32, parent2: i32, formula: i16, targetkey: i32) -> bool {
    let mut visited = vec![false; ((sheet.rows as i32) * (sheet.cols as i32)) as usize];  // Added vis to reduce computation
    let mut stack = vec![(targetkey)];
    let rem = formula % 10;
    let cols = sheet.cols;
    let mut row = 0;
    let mut col = 0;
    get_key_row_col(targetkey, cols, &mut row, &mut col);
    let mut start_row = 0;
    let mut start_col = 0;
    let mut end_row = 0;
    let mut end_col = 0;
    if rem >= 5 {
        get_key_row_col(parent1, cols, &mut start_row, &mut start_col);
        get_key_row_col(parent2, cols, &mut end_row, &mut end_col);
    }
    while let Some(key) = stack.pop() {
        // Base key is the parent then cycle is detected
        if visited[key as usize] {
            continue; // Skip if already visited;
        }
        if rem == 0 && (parent1 == key || parent2 == key){ 
            return true;
        }
        else if rem == 2 && parent1 == key {
            return true;
        }
        else if rem == 3 && parent2 == key {
            return true;
        }
        else if rem >= 5 && ( start_row <= row && row <= end_row) && (start_col <= col && col <= end_col) {
            return true;
        }
        visited[key as usize] = true;
        let cell = sheet.get_cell(row, col);
        if let Some(ref children) = cell.children {
            add_linkedlist_stack(&mut stack, &Some(children.clone()));
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_remove_child() {
        let mut cell = Cell::new();
        add_child(&mut cell, 0, 1, 5);
        assert!(cell.children.is_some());
        remove_child(&mut cell, get_key(0, 1, 5));
        assert!(cell.children.is_none());
    }

    #[test]
    fn test_add_children() {
        let mut sheet = Spreadsheet::create(5, 5).unwrap();
        add_children(&mut sheet, get_key(0, 0, 5), get_key(1, 1, 5), 5, 2, 2); // Range formula
        assert!(sheet.get_cell(0, 0).children.is_some());
    }

    // #[test]
    // fn test_detect_cycle() {
    //     let mut sheet = Spreadsheet::create(5, 5).unwrap();
    //     sheet.get_mut_cell(0, 0).parent1 = get_key(0, 1, 5);
    //     sheet.get_mut_cell(0, 1).parent1 = get_key(0, 0, 5);
    //     assert!(detect_cycle(&sheet, get_key(0, 1, 5), -1, 82, get_key(0, 0, 5)));
    // }
}