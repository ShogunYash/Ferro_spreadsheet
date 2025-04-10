use crate::cell::Cell;
use crate::spreadsheet::Spreadsheet;
use crate::linked_list::Node;
use crate::evaluator::{get_cell_from_key, get_key};

pub fn remove_child(parent: &Cell, key: i32) {
    parent.children.remove(key);
}

pub fn add_child(parent: &Cell, row: i16, col:i16, cols:i16){
    let key = get_key(row, col, cols);
    parent.children.prepend(key);
}

pub fn get_key_row_col(key: i32, cols: i16, row: &mut i16, col: &mut i16) {
    *row = (key / cols as i32) as i16;
    *col = (key % cols as i32) as i16;
}

pub fn add_children(sheet:&Spreadsheet, cell1: i32, cell2:i32 , formula:i16 , row :i16, col:i16)   {
    let rem = formula %10 as i16;
    if formula== -1 {
        return;
    }   
    if rem == 0{
        let ref_cell1 = get_cell_from_key(sheet , cell1);
        let ref_cell2= get_cell_from_key(sheet,cell2);
        let cols=sheet.cols;
        add_child(ref_cell1, row, col, cols);
        add_child(ref_cell2, row, col, cols);
    }
    else if rem == 2{
        let ref_cell1 = get_cell_from_key(sheet , cell1);
        let cols=sheet.cols;
        add_child(ref_cell1, row, col, cols);
    }
    else if rem == 3{
        let ref_cell2 = get_cell_from_key(sheet , cell2);
        let cols=sheet.cols;
        add_child(ref_cell2, row, col, cols);
    }
    else {
        let start_row = (cell1/(sheet.cols as i32)) as i16;
        let start_col = (cell1%(sheet.cols as i32) )as i16;
        let end_row = (cell2/(sheet.cols as i32)) as i16;
        let end_col = (cell2%(sheet.cols as i32)) as i16;
        for i in start_row..=end_row {
            for j in start_col..=end_col {
                let ref_cell= sheet.get_cell(i,j);
                add_child(ref_cell, row, col, sheet.cols);
            }
        }
    }
}

pub fn remove_all_parents(sheet: &mut Spreadsheet, row: i16, col: i16){
    let child = sheet.get_cell(row, col);
    if (child.formula == -1){
        return;
    }
    let rem = (child.formula%10) as i16;
    if child.formula <= 9 && child.formula >= 5 {//is a range struct really required
        let start_row = (child.parent1/(sheet.cols as i32)) as i16;
        let start_col = (child.parent1%(sheet.cols as i32)) as i16;
        let end_row = (child.parent2/(sheet.cols as i32)) as i16;
        let end_col = (child.parent2%(sheet.cols as i32)) as i16;
        for i in start_row..=end_row {
            for j in start_col..=end_col {
                    let ref_cell = sheet.get_cell(i,j);
                    let key = get_key(i,j, sheet.cols);
                    remove_child(ref_cell, key);
            }
        }
    }
    else if rem == 0 {
        let mut row: i16 = 0;
        let mut col: i16 = 0;
        let cols: i16 = sheet.cols;
        get_key_row_col(child.parent1, cols, &mut row, &mut col);
        let ref_cell1 = sheet.get_cell(row, col);
        remove_child(ref_cell1, child.parent1);
        get_key_row_col(child.parent2, cols, &mut row, &mut col);
        let ref_cell2 = sheet.get_cell(row, col);
        remove_child(ref_cell2, child.parent2);
    }
    else if rem == 2 {
        let mut row: i16 = 0;
        let mut col: i16 = 0;
        let cols: i16 = sheet.cols;
        get_key_row_col(child.parent1, cols, &mut row, &mut col);
        let ref_cell1 = sheet.get_cell(row, col);
        remove_child(ref_cell1, child.parent1);
    }
    else if rem == 3{
        let mut row: i16 = 0;
        let mut col: i16 = 0;
        let cols: i16 = sheet.cols;
        get_key_row_col(child.parent2, cols, &mut row, &mut col);
        let ref_cell2 = sheet.get_cell(row, col);
        remove_child(ref_cell2, child.parent2);
    }
}

// Convert the linked list to a vector for testing or debugging
pub fn add_linkedlist_stack(stack: &mut Vec<i32>, head: &Node) {
    let mut current = head;
    loop {
        if current.key == -1 {
            break; // Stop if we reach the end of the linked list
        }
        stack.push(current.key);
        current = current.next.as_ref().unwrap(); // Move to the next node
    }
}

pub fn detect_cycle(sheet: &Spreadsheet, parent1: i32, parent2: i32, formula: i16, targetkey: i32) -> bool {
    let mut visited = vec![false; (sheet.rows * sheet.cols) as usize];  // Added vis to reduce computation
    let mut stack = vec![(targetkey)];
    let for_rem = formula % 10;
    if for_rem == 0 {
        visited[parent1 as usize] = true;
        visited[parent2 as usize] = true;
    }
    else if for_rem == 2{
        visited[parent1 as usize] = true;
    }
    else if for_rem == 3{
        visited[parent2 as usize] = true;
    }
    else {
        let cols = sheet.cols;
        let mut start_row = 0;
        let mut start_col = 0;
        get_key_row_col(parent1, cols, &mut start_row, &mut start_col);
        let mut end_row = 0;
        let mut end_col = 0;
        get_key_row_col(parent2, cols, &mut end_row, &mut end_col);
        for i in start_row..=end_row {
            for j in start_col..=end_col {
                visited[((i as i32 )* (sheet.cols as i32) + (j as i32)) as usize] = true;
            }
        }
    }
    while let Some(key) = stack.pop() {
        if visited[key as usize] {
            return true;
        }
        visited[key as usize] = true;
        let cell = get_cell_from_key(sheet, key);
        add_linkedlist_stack(&mut stack, &cell.children);
    }
    false
}