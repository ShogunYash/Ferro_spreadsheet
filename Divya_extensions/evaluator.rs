use crate::spreadsheet::{Spreadsheet, CommandStatus, MAX_DISPLAY};
use crate::cell::{CellValue, parse_cell_reference};
use crate::formula::{parse_range, Range, eval_max, eval_min, sum_value, eval_variance, eval_avg};
use crate::graph::{add_children, remove_all_parents};
use crate::reevaluate_topo::{toposort_reval_detect_cycle, sleep_fn};
use std::collections::HashSet;

fn resolve_cell_reference(sheet: &Spreadsheet, s: &str) -> Result<(i16, i16), CommandStatus> {
    if let Some(range) = sheet.named_ranges.get(s) {
        if range.start_row == range.end_row && range.start_col == range.end_col {
            Ok((range.start_row, range.start_col))
        } else {
            Err(CommandStatus::CmdUnrecognized)
        }
    } else {
        parse_cell_reference(sheet, s)
    }
}

fn get_formula_string(sheet: &Spreadsheet, row: i16, col: i16) -> String {
    let meta = sheet.get_cell_meta_ref(row, col);
    if meta.formula == -1 {
        return "No formula".to_string();
    }
    let rem = meta.formula % 10;
    let msb = meta.formula / 10;
    let parent1 = meta.parent1;
    let parent2 = meta.parent2;

    match rem {
        0 => {
            let (left, right) = if parent1 >= 0 && parent2 >= 0 {
                // Check if parent1 and parent2 are literals (not cell references)
                let is_literal1 = parent1 < (sheet.rows as i32 * sheet.cols as i32) && parent1 >= 0;
                let is_literal2 = parent2 < (sheet.rows as i32 * sheet.cols as i32) && parent2 >= 0;

                if is_literal1 && is_literal2 {
                    (parent1.to_string(), parent2.to_string())
                } else if is_literal1 {
                    let (right_row, right_col) = sheet.get_row_col(parent2);
                    let right_name = sheet.get_cell_name(right_row, right_col);
                    (parent1.to_string(), right_name)
                } else if is_literal2 {
                    let (left_row, left_col) = sheet.get_row_col(parent1);
                    let left_name = sheet.get_cell_name(left_row, left_col);
                    (left_name, parent2.to_string())
                } else {
                    let (left_row, left_col) = sheet.get_row_col(parent1);
                    let (right_row, right_col) = sheet.get_row_col(parent2);
                    let left_name = sheet.get_cell_name(left_row, left_col);
                    let right_name = sheet.get_cell_name(right_row, right_col);
                    (left_name, right_name)
                }
            } else {
                return "Invalid formula".to_string();
            };
            match msb {
                1 => format!("{} + {}", left, right),
                2 => format!("{} - {}", left, right),
                4 => format!("{} * {}", left, right),
                3 => format!("{} / {}", left, right),
                _ => "Unknown operation".to_string(),
            }
        }
        2 => {
            let (left, right) = if parent1 >= 0 {
                let (left_row, left_col) = sheet.get_row_col(parent1);
                let left_name = sheet.get_cell_name(left_row, left_col);
                (left_name, parent2.to_string())
            } else {
                return "Invalid formula".to_string();
            };
            match msb {
                1 => format!("{} + {}", left, right),
                2 => format!("{} - {}", left, right),
                4 => format!("{} * {}", left, right),
                3 => format!("{} / {}", left, right),
                10 => format!("SLEEP({})", left),
                _ => "Unknown operation".to_string(),
            }
        }
        3 => {
            let (left, right) = if parent2 >= 0 {
                let (right_row, right_col) = sheet.get_row_col(parent2);
                let right_name = sheet.get_cell_name(right_row, right_col);
                (parent1.to_string(), right_name)
            } else {
                return "Invalid formula".to_string();
            };
            match msb {
                1 => format!("{} + {}", left, right),
                2 => format!("{} - {}", left, right),
                4 => format!("{} * {}", left, right),
                3 => format!("{} / {}", left, right),
                _ => "Unknown operation".to_string(),
            }
        }
        5 => {
            let (start_row, start_col) = sheet.get_row_col(parent1);
            let (end_row, end_col) = sheet.get_row_col(parent2);
            let start_name = sheet.get_cell_name(start_row, start_col);
            let end_name = sheet.get_cell_name(end_row, end_col);
            format!("SUM({}:{})", start_name, end_name)
        }
        6 => {
            let (start_row, start_col) = sheet.get_row_col(parent1);
            let (end_row, end_col) = sheet.get_row_col(parent2);
            let start_name = sheet.get_cell_name(start_row, start_col);
            let end_name = sheet.get_cell_name(end_row, end_col);
            format!("AVG({}:{})", start_name, end_name)
        }
        7 => {
            let (start_row, start_col) = sheet.get_row_col(parent1);
            let (end_row, end_col) = sheet.get_row_col(parent2);
            let start_name = sheet.get_cell_name(start_row, start_col);
            let end_name = sheet.get_cell_name(end_row, end_col);
            format!("MIN({}:{})", start_name, end_name)
        }
        8 => {
            let (start_row, start_col) = sheet.get_row_col(parent1);
            let (end_row, end_col) = sheet.get_row_col(parent2);
            let start_name = sheet.get_cell_name(start_row, start_col);
            let end_name = sheet.get_cell_name(end_row, end_col);
            format!("MAX({}:{})", start_name, end_name)
        }
        9 => {
            let (start_row, start_col) = sheet.get_row_col(parent1);
            let (end_row, end_col) = sheet.get_row_col(parent2);
            let start_name = sheet.get_cell_name(start_row, start_col);
            let end_name = sheet.get_cell_name(end_row, end_col);
            format!("STDEV({}:{})", start_name, end_name)
        }
        _ => {
            if meta.formula == 82 {
                let (left_row, left_col) = sheet.get_row_col(parent1);
                let left_name = sheet.get_cell_name(left_row, left_col);
                left_name
            } else if meta.formula == 102 {
                let (left_row, left_col) = sheet.get_row_col(parent1);
                let left_name = sheet.get_cell_name(left_row, left_col);
                format!("SLEEP({})", left_name)
            } else {
                "Unknown formula".to_string()
            }
        }
    }
}

pub fn handle_sleep(
    sheet: &mut Spreadsheet,
    row: i16,
    col: i16,
    expr: &str,
    sleep_time: &mut f64,
) -> CommandStatus {
    let cell_key = sheet.get_key(row, col);
    if let Ok((target_row, target_col)) = resolve_cell_reference(sheet, expr) {
        let pkey = sheet.get_key(target_row, target_col);
        if row == target_row && col == target_col {
            return CommandStatus::CmdCircularRef;
        }
        remove_all_parents(sheet, row, col);
        let meta = sheet.get_cell_meta(row, col);
        meta.parent1 = pkey;
        meta.parent2 = -1;
        meta.formula = 102;
        add_children(sheet, pkey, -1, 102, row, col);
        let parent_value = sheet.get_cell(target_row, target_col);
        if let CellValue::Integer(val) = parent_value {
            sleep_fn(sheet, row, col, *val, sleep_time);
        } else {
            *sheet.get_mut_cell(row, col) = CellValue::Error;
        }
    } else if let Ok(val) = expr.parse::<i32>() {
        remove_all_parents(sheet, row, col);
        sheet.cell_meta.remove(&cell_key);
        sleep_fn(sheet, row, col, val, sleep_time);
    } else {
        return CommandStatus::CmdUnrecognized;
    }
    sheet.set_last_edited(row, col);
    CommandStatus::CmdOk
}

pub fn evaluate_arithmetic(
    sheet: &mut Spreadsheet,
    row: i16,
    col: i16,
    expr: &str,
) -> CommandStatus {
    let cell_key = sheet.get_key(row, col);
    if let Ok(number) = expr.parse::<i32>() {
        remove_all_parents(sheet, row, col);
        sheet.cell_meta.remove(&cell_key);
        *sheet.get_mut_cell(row, col) = CellValue::Integer(number);
        sheet.set_last_edited(row, col);
        return CommandStatus::CmdOk;
    }
    let mut all_alnum = true;
    for &b in expr.as_bytes() {
        if !(b.is_ascii_alphanumeric() || b == b'_') {
            all_alnum = false;
            break;
        }
    }
    if all_alnum {
        if let Ok((target_row, target_col)) = resolve_cell_reference(sheet, expr) {
            let ref_cell_key = sheet.get_key(target_row, target_col);
            remove_all_parents(sheet, row, col);
            let meta = sheet.get_cell_meta(row, col);
            meta.parent1 = ref_cell_key;
            meta.parent2 = -1;
            meta.formula = 82;
            add_children(sheet, ref_cell_key, -1, 82, row, col);
            *sheet.get_mut_cell(row, col) = match sheet.get_cell(target_row, target_col) {
                CellValue::Integer(val) => CellValue::Integer(*val),
                _ => CellValue::Error,
            };
            sheet.set_last_edited(row, col);
            return CommandStatus::CmdOk;
        }
    }
    let bytes = expr.as_bytes();
    let mut op_idx = 0;
    let mut op = 0u8;
    for i in 1..bytes.len() {
        match bytes[i] {
            b'+' | b'-' | b'*' | b'/' => {
                op = bytes[i];
                op_idx = i;
                break;
            }
            _ => {}
        }
    }
    if op_idx == 0 {
        return CommandStatus::CmdUnrecognized;
    }
    let left = &expr[..op_idx];
    let right = &expr[op_idx + 1..];
    if left.is_empty() || right.is_empty() {
        return CommandStatus::CmdUnrecognized;
    }
    let mut left_val = 0;
    let mut right_val = 0;
    let mut left_is_cell = false;
    let mut right_is_cell = false;
    let mut error_found = false;
    let mut left_cell_key = -1;
    let mut right_cell_key = -1;
    if let Ok(num) = left.parse::<i32>() {
        left_val = num;
    } else if let Ok((left_row, left_col)) = resolve_cell_reference(sheet, left) {
        left_is_cell = true;
        left_cell_key = sheet.get_key(left_row, left_col);
        match sheet.get_cell(left_row, left_col) {
            CellValue::Integer(val) => left_val = *val,
            _ => error_found = true,
        }
    } else {
        return CommandStatus::CmdUnrecognized;
    }
    if let Ok(num) = right.parse::<i32>() {
        right_val = num;
    } else if let Ok((right_row, right_col)) = resolve_cell_reference(sheet, right) {
        right_is_cell = true;
        right_cell_key = sheet.get_key(right_row, right_col);
        match sheet.get_cell(right_row, right_col) {
            CellValue::Integer(val) => right_val = *val,
            _ => error_found = true,
        }
    } else {
        return CommandStatus::CmdUnrecognized;
    }
    remove_all_parents(sheet, row, col);
    let mut formula_type = match op {
        b'+' => 10,
        b'-' => 20,
        b'*' => 40,
        b'/' => 30,
        _ => unreachable!(),
    };
    if left_is_cell && right_is_cell {
        formula_type += 0;
    } else if left_is_cell {
        formula_type += 2;
    } else if right_is_cell {
        formula_type += 3;
    }
    let meta = sheet.get_cell_meta(row, col);
    meta.formula = formula_type;
    meta.parent1 = if left_is_cell { left_cell_key } else { left_val };
    meta.parent2 = if right_is_cell { right_cell_key } else { right_val };
    if left_is_cell && right_is_cell {
        add_children(sheet, left_cell_key, right_cell_key, formula_type, row, col);
    } else if left_is_cell {
        add_children(sheet, left_cell_key, -1, formula_type, row, col);
    } else if right_is_cell {
        add_children(sheet, -1, right_cell_key, formula_type, row, col);
    }
    let cell = sheet.get_mut_cell(row, col);
    if error_found {
        *cell = CellValue::Error;
    } else {
        match op {
            b'+' => *cell = CellValue::Integer(left_val + right_val),
            b'-' => *cell = CellValue::Integer(left_val - right_val),
            b'*' => *cell = CellValue::Integer(left_val * right_val),
            b'/' => {
                if right_val == 0 {
                    *cell = CellValue::Error;
                } else {
                    *cell = CellValue::Integer(left_val / right_val);
                }
            }
            _ => unreachable!(),
        }
    }
    sheet.set_last_edited(row, col);
    CommandStatus::CmdOk
}

pub fn evaluate_formula(
    sheet: &mut Spreadsheet,
    row: i16,
    col: i16,
    expr: &str,
    sleep_time: &mut f64,
) -> CommandStatus {
    if expr.is_empty() {
        return CommandStatus::CmdUnrecognized;
    }
    let bytes = expr.as_bytes();
    let (is_formula, formula_type, prefix_len) = match bytes.get(0..3) {
        Some(b"AVG") if bytes.get(3) == Some(&b'(') => (true, 6, 4),
        Some(b"MIN") if bytes.get(3) == Some(&b'(') => (true, 7, 4),
        Some(b"MAX") if bytes.get(3) == Some(&b'(') => (true, 8, 4),
        Some(b"SUM") if bytes.get(3) == Some(&b'(') => (true, 5, 4),
        Some(b"SLE") if bytes.len() > 5 && bytes[3] == b'E' && bytes[4] == b'P' && bytes.get(5) == Some(&b'(') => {
            if !expr.ends_with(')') {
                return CommandStatus::CmdUnrecognized;
            }
            return handle_sleep(sheet, row, col, &expr[6..expr.len() - 1], sleep_time);
        }
        Some(b"STD") if bytes.len() > 5 && bytes[3] == b'E' && bytes[4] == b'V' && bytes.get(5) == Some(&b'(') => (true, 9, 6),
        _ => (false, -1, 0),
    };
    if is_formula {
        if !expr.ends_with(')') {
            return CommandStatus::CmdUnrecognized;
        }
        let range_str = &expr[prefix_len..expr.len() - 1];
        let range = if let Some(named_range) = sheet.named_ranges.get(range_str) {
            named_range.clone()
        } else {
            match parse_range(sheet, range_str) {
                Ok(r) => r,
                Err(status) => return status,
            }
        };
        let parent1 = sheet.get_key(range.start_row, range.start_col);
        let parent2 = sheet.get_key(range.end_row, range.end_col);
        remove_all_parents(sheet, row, col);
        let meta = sheet.get_cell_meta(row, col);
        meta.parent1 = parent1;
        meta.parent2 = parent2;
        meta.formula = formula_type;
        add_children(sheet, parent1, parent2, formula_type, row, col);
        let status = match formula_type {
            9 => eval_variance(sheet, row, col, parent1, parent2),
            8 => eval_max(sheet, row, col, parent1, parent2),
            7 => eval_min(sheet, row, col, parent1, parent2),
            6 => eval_avg(sheet, row, col, parent1, parent2),
            _ => sum_value(sheet, row, col, parent1, parent2),
        };
        sheet.set_last_edited(row, col);
        status
    } else {
        evaluate_arithmetic(sheet, row, col, expr)
    }
}

pub fn set_cell_value(
    sheet: &mut Spreadsheet,
    row: i16,
    col: i16,
    expr: &str,
    sleep_time: &mut f64,
) -> CommandStatus {
    if sheet.is_cell_locked(row, col) {
        return CommandStatus::CmdLockedCell;
    }
    let cell_key = sheet.get_key(row, col);
    let old_meta = sheet.cell_meta.get(&cell_key).cloned();
    let old_value = sheet.get_cell(row, col).clone();
    let status = evaluate_formula(sheet, row, col, expr, sleep_time);
    if let CommandStatus::CmdOk = status {
        let has_cycle = toposort_reval_detect_cycle(sheet, row, col, sleep_time);
        if has_cycle {
            remove_all_parents(sheet, row, col);
            *sheet.get_mut_cell(row, col) = old_value;
            if let Some(old) = old_meta {
                sheet.cell_meta.insert(cell_key, old.clone());
                add_children(sheet, old.parent1, old.parent2, old.formula, row, col);
            } else {
                sheet.cell_meta.remove(&cell_key);
            }
            return CommandStatus::CmdCircularRef;
        } else {
            sheet.cell_history.entry(cell_key).or_insert_with(Vec::new).push(old_value);
            sheet.set_last_edited(row, col);
        }
    }
    status
}

fn set_cell_to_value(
    sheet: &mut Spreadsheet,
    row: i16,
    col: i16,
    value: CellValue,
    sleep_time: &mut f64,
) -> CommandStatus {
    if sheet.is_cell_locked(row, col) {
        return CommandStatus::CmdLockedCell;
    }
    let cell_key = sheet.get_key(row, col);
    remove_all_parents(sheet, row, col);
    sheet.cell_meta.remove(&cell_key);
    *sheet.get_mut_cell(row, col) = value;
    toposort_reval_detect_cycle(sheet, row, col, sleep_time);
    sheet.set_last_edited(row, col);
    CommandStatus::CmdOk
}

pub fn handle_command(
    sheet: &mut Spreadsheet,
    trimmed: &str,
    sleep_time: &mut f64,
) -> (CommandStatus, Option<(i32, HashSet<i32>, HashSet<i32>)>) {
    if trimmed.len() == 1 {
        match trimmed.as_bytes()[0] {
            b'w' | b'a' | b's' | b'd' => {
                let direction = trimmed.chars().next().unwrap();
                sheet.scroll_viewport(direction);
                return (CommandStatus::CmdOk, None);
            }
            b'q' => return (CommandStatus::CmdOk, None),
            _ => {}
        }
    }
    match trimmed {
        "disable_output" => {
            sheet.output_enabled = false;
            return (CommandStatus::CmdOk, None);
        }
        "enable_output" => {
            sheet.output_enabled = true;
            return (CommandStatus::CmdOk, None);
        }
        "last_edit" => {
            sheet.scroll_to_last_edited();
            return (CommandStatus::CmdOk, None);
        }
        _ => {}
    }
    if trimmed.starts_with("visualize ") {
        let cell_ref = &trimmed[10..];
        match parse_cell_reference(sheet, cell_ref) {
            Ok((row, col)) => return (sheet.visualize_cell_relationships(row, col), None),
            Err(status) => return (status, None),
        }
    }
    if trimmed.len() > 10 && &trimmed.as_bytes()[..9] == b"scroll_to" && trimmed.as_bytes()[9] == b' ' {
        let cell_ref = &trimmed[10..];
        return (sheet.scroll_to_cell(cell_ref), None);
    }
    if trimmed.starts_with("display ") {
        let num_str = trimmed.get(8..).unwrap_or("").trim();
        match num_str.parse::<i16>() {
            Ok(num) if num > 0 && num <= MAX_DISPLAY => {
                sheet.display_rows = num;
                sheet.display_cols = num;
                return (CommandStatus::CmdOk, None);
            }
            _ => return (CommandStatus::CmdUnrecognized, None),
        }
    }
    if trimmed.starts_with("lock_cell ") {
        let lock_target = trimmed.get(10..).unwrap_or("").trim();
        if lock_target.contains(':') {
            match parse_range(sheet, lock_target) {
                Ok(range) => {
                    sheet.lock_range(range);
                    return (CommandStatus::CmdOk, None);
                }
                Err(_) => return (CommandStatus::CmdUnrecognized, None),
            }
        } else {
            match resolve_cell_reference(sheet, lock_target) {
                Ok((row, col)) => {
                    let range = Range {
                        start_row: row,
                        start_col: col,
                        end_row: row,
                        end_col: col,
                    };
                    sheet.lock_range(range);
                    return (CommandStatus::CmdOk, None);
                }
                Err(status) => return (status, None),
            }
        }
    }
    if trimmed.starts_with("name ") {
        let parts: Vec<&str> = trimmed[5..].split_whitespace().collect();
        if parts.len() == 2 {
            let target = parts[0];
            let name = parts[1];
            if let Ok(range) = parse_range(sheet, target) {
                sheet.named_ranges.insert(name.to_string(), range);
                return (CommandStatus::CmdOk, None);
            } else if let Ok((row, col)) = parse_cell_reference(sheet, target) {
                let range = Range {
                    start_row: row,
                    start_col: col,
                    end_row: row,
                    end_col: col,
                };
                sheet.named_ranges.insert(name.to_string(), range);
                return (CommandStatus::CmdOk, None);
            }
        }
        return (CommandStatus::CmdUnrecognized, None);
    }
    if trimmed.starts_with("history ") {
        let cell_ref = trimmed[8..].trim();
        return match resolve_cell_reference(sheet, cell_ref) {
            Ok((row, col)) => {
                let cell_key = sheet.get_key(row, col);
                if let Some(history) = sheet.cell_history.get_mut(&cell_key) {
                    if let Some(prev_value) = history.pop() {
                        (set_cell_to_value(sheet, row, col, prev_value, sleep_time), None)
                    } else {
                        (CommandStatus::CmdOk, None)
                    }
                } else {
                    (CommandStatus::CmdOk, None)
                }
            }
            Err(status) => (status, None),
        };
    }
    if trimmed.starts_with("high_dep ") {
        let cell_ref = trimmed[9..].trim();
        match resolve_cell_reference(sheet, cell_ref) {
            Ok((row, col)) => {
                let target_key = sheet.get_key(row, col);
                let parents = sheet.get_parents(target_key);
                let children = sheet.get_children(target_key);
                return (CommandStatus::CmdOk, Some((target_key, parents, children)));
            }
            Err(status) => return (status, None),
        }
    }
    if trimmed.starts_with("formula ") {
        let cell_ref = trimmed[8..].trim();
        match resolve_cell_reference(sheet, cell_ref) {
            Ok((row, col)) => {
                let formula_str = get_formula_string(sheet, row, col);
                println!("{}", formula_str);
                return (CommandStatus::CmdOk, None);
            }
            Err(status) => return (status, None),
        }
    }
    let bytes = trimmed.as_bytes();
    let mut eq_pos = None;
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'=' {
            eq_pos = Some(i);
            break;
        }
    }
    if let Some(pos) = eq_pos {
        let cell_ref = trimmed[..pos].trim();
        let expr = trimmed[pos + 1..].trim();
        let (row, col) = match resolve_cell_reference(sheet, cell_ref) {
            Ok((r, c)) => (r, c),
            Err(status) => return (status, None),
        };
        return (set_cell_value(sheet, row, col, expr, sleep_time), None);
    }
    (CommandStatus::CmdUnrecognized, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::CellValue;
    use crate::spreadsheet::{CommandStatus, Spreadsheet};

    fn create_test_spreadsheet(rows: i16, cols: i16) -> Spreadsheet {
        Spreadsheet::create(rows, cols).unwrap()
    }

    #[test]
    fn test_handle_sleep_with_reference() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(5);
        let mut sleep_time = 0.0;
        assert_eq!(
            handle_sleep(&mut sheet, 1, 1, "A1", &mut sleep_time),
            CommandStatus::CmdOk
        );
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(5));
        assert_eq!(sleep_time, 5.0);
        let meta = sheet.cell_meta.get(&sheet.get_key(1, 1)).unwrap();
        assert_eq!(meta.formula, 102);
        assert_eq!(meta.parent1, sheet.get_key(0, 0));
        assert_eq!(sheet.last_edited, Some((1, 1)));
    }

    #[test]
    fn test_handle_sleep_with_literal() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let mut sleep_time = 0.0;
        assert_eq!(
            handle_sleep(&mut sheet, 1, 1, "3", &mut sleep_time),
            CommandStatus::CmdOk
        );
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(3));
        assert_eq!(sleep_time, 3.0);
        assert!(!sheet.cell_meta.contains_key(&sheet.get_key(1, 1)));
        assert_eq!(sheet.last_edited, Some((1, 1)));
    }

    #[test]
    fn test_evaluate_arithmetic_literal() {
        let mut sheet = create_test_spreadsheet(5, 5);
        assert_eq!(
            evaluate_arithmetic(&mut sheet, 0, 0, "42"),
            CommandStatus::CmdOk
        );
        assert_eq!(*sheet.get_cell(0, 0), CellValue::Integer(42));
        assert_eq!(sheet.last_edited, Some((0, 0)));
    }

    #[test]
    fn test_evaluate_arithmetic_cell_ref() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(10);
        assert_eq!(
            evaluate_arithmetic(&mut sheet, 1, 1, "A1"),
            CommandStatus::CmdOk
        );
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(10));
        assert_eq!(sheet.last_edited, Some((1, 1)));
    }

    #[test]
    fn test_evaluate_arithmetic_binary_add() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(5);
        assert_eq!(
            evaluate_arithmetic(&mut sheet, 1, 1, "A1+3"),
            CommandStatus::CmdOk
        );
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(8));
        assert_eq!(sheet.last_edited, Some((1, 1)));
    }

    #[test]
    fn test_evaluate_formula_sum() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(1);
        *sheet.get_mut_cell(0, 1) = CellValue::Integer(2);
        let mut sleep_time = 0.0;
        assert_eq!(
            evaluate_formula(&mut sheet, 1, 1, "SUM(A1:B1)", &mut sleep_time),
            CommandStatus::CmdOk
        );
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(3));
        assert_eq!(sheet.last_edited, Some((1, 1)));
    }

    #[test]
    fn test_handle_command_last_edit() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let mut sleep_time = 0.0;
        handle_command(&mut sheet, "B2=42", &mut sleep_time);
        assert_eq!(sheet.last_edited, Some((1, 1)));
        assert_eq!(
            handle_command(&mut sheet, "last_edit", &mut sleep_time).0,
            CommandStatus::CmdOk
        );
        assert_eq!(sheet.viewport_row, 1);
        assert_eq!(sheet.viewport_col, 1);
    }

    #[test]
    fn test_handle_command_display() {
        let mut sheet = create_test_spreadsheet(20, 20);
        let mut sleep_time = 0.0;
        assert_eq!(
            handle_command(&mut sheet, "display 5", &mut sleep_time).0,
            CommandStatus::CmdOk
        );
        assert_eq!(sheet.display_rows, 5);
        assert_eq!(sheet.display_cols, 5);
    }

    #[test]
    fn test_handle_command_lock_cell() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let mut sleep_time = 0.0;
        assert_eq!(
            handle_command(&mut sheet, "lock_cell B2", &mut sleep_time).0,
            CommandStatus::CmdOk
        );
        assert!(sheet.is_cell_locked(1, 1));
    }

    #[test]
    fn test_handle_command_history() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let mut sleep_time = 0.0;
        assert_eq!(
            handle_command(&mut sheet, "A2=2", &mut sleep_time).0,
            CommandStatus::CmdOk
        );
        assert_eq!(
            handle_command(&mut sheet, "A2=3", &mut sleep_time).0,
            CommandStatus::CmdOk
        );
        assert_eq!(
            handle_command(&mut sheet, "history A2", &mut sleep_time).0,
            CommandStatus::CmdOk
        );
        assert_eq!(*sheet.get_cell(1, 0), CellValue::Integer(2));
        assert_eq!(sheet.last_edited, Some((1, 0)));
    }

    #[test]
    fn test_evaluate_arithmetic_div_by_zero() {
        let mut sheet = create_test_spreadsheet(5, 5);
        assert_eq!(
            evaluate_arithmetic(&mut sheet, 0, 0, "5/0"),
            CommandStatus::CmdOk
        );
        assert_eq!(*sheet.get_cell(0, 0), CellValue::Error);
    }

    #[test]
    fn test_evaluate_formula_max() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(5);
        *sheet.get_mut_cell(0, 1) = CellValue::Integer(3);
        let mut sleep_time = 0.0;
        assert_eq!(
            evaluate_formula(&mut sheet, 1, 1, "MAX(A1:B1)", &mut sleep_time),
            CommandStatus::CmdOk
        );
        assert_eq!(*sheet.get_cell(1, 1), CellValue::Integer(5));
    }

    #[test]
    fn test_set_cell_value_circular_ref() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let mut sleep_time = 0.0;
        assert_eq!(
            set_cell_value(&mut sheet, 0, 0, "A1", &mut sleep_time),
            CommandStatus::CmdCircularRef
        );
        assert_eq!(*sheet.get_cell(0, 0), CellValue::Integer(0));
    }

    #[test]
    fn test_handle_command_scroll_to() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let mut sleep_time = 0.0;
        assert_eq!(
            handle_command(&mut sheet, "scroll_to B2", &mut sleep_time).0,
            CommandStatus::CmdOk
        );
        assert_eq!(sheet.viewport_row, 1);
        assert_eq!(sheet.viewport_col, 1);
    }
}