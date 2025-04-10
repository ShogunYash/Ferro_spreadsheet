use crate::cell::{Cell, CellValue, parse_cell_reference};
use crate::spreadsheet::{Spreadsheet, CommandStatus};
use crate::linked_list::Node;

pub fn remove_child(parent: &Cell, key: i32) {
    parent.children.remove_child(key);
}

pub fn add_child(parent: &Cell, row: i16, col:i16, cols:i16){
    let key = get_key(row, col, cols);
    parent.children.prepend(key);
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
        let start_row = cell1/sheet.cols;
        let start_col = cell1%sheet.cols;
        let end_row = cell2/sheet.cols; 
        let end_col = cell2%sheet.cols;
        for i in start_row..=end_row {
            for j in start_col..=end_col {
                let ref_cell = get_cell_from_key(sheet, i*sheet.cols+j);
                add_child(ref_cell, row, col, sheet.cols);
            }
        }
    }
}

pub fn remove_all_parents(sheet: &mut Spreadsheet, row: i16, col: i16){
    let child;
    if let Some(cell) = sheet.get_cell(row, col) {
        child = cell;
    }
    if (child.formula == -1){
        return;
    }
    let rem = (child.formula%10) as i16;
    if (child.formula <= 9 && child.formula >= 5){//is a range struct really required
        let start_row = child.parent1/sheet.cols;
        let start_col = child.parent1%sheet.cols;
        let end_row = child.parent2/sheet.cols; 
        let end_col = child.parent2%sheet.cols;
        for i in start_row..=end_row {
            for j in start_col..=end_col {
                if let Some(ref_cell) = sheet.get_cell(i,j){
                    remove_child(ref_cell, key);
                }
            }
        }
    }
    else if rem == 0 {
        let ref_cell1 = sheet.get_cell(child.parent1/sheet.cols, child.parent1%sheet.cols);
        let ref_cell2 = sheet.get_cell(child.parent2/sheet.cols, child.parent2%sheet.cols);
        if let Some(ref_cell1) = ref_cell1 {
            remove_child(ref_cell1, key);
        }
        if let Some(ref_cell2) = ref_cell2 {
            remove_child(ref_cell2, key);
        }
    }
    else if rem == 2 {
        let ref_cell1 = sheet.get_cell(child.parent1/sheet.cols, child.parent1%sheet.cols);
        if let Some(ref_cell1) = ref_cell1 {
            remove_child(ref_cell1, key);
        }
    }
    else if rem == 3{
        let ref_cell2 = sheet.get_cell(child.parent2/sheet.cols, child.parent2%sheet.cols);
        if let Some(ref_cell2) = ref_cell2 {
            remove_child(ref_cell2, key);
        }
    }
}

// Convert the linked list to a vector for testing or debugging
pub fn add_linkedList_stack(stack: &mut Vec<i32>, head: &Node) {
    let mut current = head;
    while let Some(node) = current {
        stack.push(node.key);
        current = &node.next;
    }
}

pub fn detect_cycle(sheet: &mut Spreadsheet, parent1: i32, parent2: i32, formula: i16, targetkey: i32) -> bool {
    let mut visited = vec![false; sheet.rows * sheet.cols];  // Added vis to reduce computation
    let mut stack = vec![(targetkey)];
    let for_rem = formula % 10;
    if for_rem == 0 {
        visited[parent1] = true;
        visited[parent2] = true;
    }
    else if for_rem == 2{
        visited[parent1] = true;
    }
    else if for_rem == 3{
        visited[parent2] = true;
    }
    else {
        let start_row = parent1 / sheet.cols;
        let start_col = parent1 % sheet.cols;
        let end_row = parent2 / sheet.cols;
        let end_col = parent2 % sheet.cols;
        for i in start_row..=end_row {
            for j in start_col..=end_col {
                visited[((i as i32 )* (sheet.cols as i32) + (j as i32)) as i32] = true;
            }
        }
    }
    while let Some(key) = stack.pop() {
        if visited[key] {
            return true;
        }
        visited[key] = true;
        let cell = get_cell_from_key(sheet, key);
        add_linkedList_stack(&mut stack, &cell.children);
    }
    false
}