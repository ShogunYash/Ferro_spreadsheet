use crate::spreadsheet::Spreadsheet;
use crate::cell::CellValue;
use crate::formula::{eval_max, eval_min, sum_value, eval_variance, eval_avg};

pub fn sleep_fn(sheet: &mut Spreadsheet, row: i16, col: i16, value: i32, sleep_val: &mut f64) {
    *sheet.get_mut_cell(row, col) = CellValue::Integer(value);
    if value < 0 {
        return;
    }
    *sleep_val += value as f64;
}

pub fn reevaluate_formula(sheet: &mut Spreadsheet, row: i16, col: i16, sleep_val: &mut f64) {

    let cell_meta = sheet.get_cell_meta(row, col);
    let rem = cell_meta.formula % 10;
    let msb = cell_meta.formula / 10;
    let parent1 = cell_meta.parent1;
    let parent2 = cell_meta.parent2;

    match rem {
        0 => {
            let par1 = sheet.get_key_cell(parent1);
            let par2 = sheet.get_key_cell(parent2);
            if CellValue::Error == *par1 || CellValue::Error == *par2 {
                *sheet.get_mut_cell(row, col) = CellValue::Error;
                return;
            }
            if let CellValue::Integer(p1_value) = par1{
                if let CellValue::Integer(p2_value) = par2 {
                    match msb {
                        1 => {
                            *sheet.get_mut_cell(row, col) = CellValue::Integer(p1_value + p2_value);
                        }
                        2 => {
                            *sheet.get_mut_cell(row, col) = CellValue::Integer(p1_value - p2_value);
                        }
                        4 => {
                            *sheet.get_mut_cell(row, col) = CellValue::Integer(p1_value * p2_value);
                        }
                        _ => {
                            if *p2_value == 0 {
                                *sheet.get_mut_cell(row, col) = CellValue::Error;
                            } else {
                                *sheet.get_mut_cell(row, col) = CellValue::Integer(p1_value / p2_value);
                            }
                        }
                    }
                }
            }
        }
        2 => {
            let par1 = sheet.get_key_cell(parent1);
            if CellValue::Error == *par1 {
                *sheet.get_mut_cell(row, col) = CellValue::Error;
                return;
            }
            if let CellValue::Integer(p1_value) = par1 {
                match msb {
                    1 => {
                        *sheet.get_mut_cell(row, col) = CellValue::Integer(p1_value + parent2);
                    }
                    2 => {
                        *sheet.get_mut_cell(row, col) = CellValue::Integer(p1_value - parent2);
                    }
                    4 => {
                        *sheet.get_mut_cell(row, col) = CellValue::Integer(p1_value * parent2);
                    }
                    3 => {
                        if parent2 == 0 {
                            *sheet.get_mut_cell(row, col) = CellValue::Error;
                        } else {
                            *sheet.get_mut_cell(row, col) = CellValue::Integer(p1_value / parent2);
                        }
                    }
                    _ => {
                        sleep_fn(sheet, row, col, *p1_value, sleep_val);
                    }
                }
            }
        }
        3 => {
            let par2 = sheet.get_key_cell(parent2);
            if CellValue::Error == *par2 {
                *sheet.get_mut_cell(row, col) = CellValue::Error;
                return;
            }
            if let CellValue::Integer(p2_value) = par2 {
                match msb {
                    1 => {
                        *sheet.get_mut_cell(row, col) = CellValue::Integer(parent1 + p2_value);
                    }
                    2 => {
                        *sheet.get_mut_cell(row, col) = CellValue::Integer(parent1 - p2_value);
                    }
                    4 => {
                        *sheet.get_mut_cell(row, col) = CellValue::Integer(parent1 * p2_value);
                    }
                    _ => {
                        if *p2_value == 0 {
                            *sheet.get_mut_cell(row, col) = CellValue::Error;
                        } else {
                            *sheet.get_mut_cell(row, col) = CellValue::Integer(parent1 / p2_value);
                        }
                    }
                }
            }
        }
        5 => { sum_value(sheet, row, col, parent1, parent2); }
        6 => { eval_avg(sheet, row, col, parent1, parent2); }
        7 => { eval_min(sheet, row, col, parent1, parent2); }
        8 => { eval_max(sheet, row, col, parent1, parent2); }
        _ => { eval_variance(sheet, row, col, parent1, parent2); }
    }
}

pub fn toposort_reval_detect_cycle(sheet: &mut Spreadsheet, row: i16, col: i16, sleep_val: &mut f64) -> bool{
    
}