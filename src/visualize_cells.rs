//! Visualization of cell relationships in the spreadsheet.
//!
//! Generates a graph of cell dependencies and saves it as a DOT file, optionally rendering it as an image.

use crate::cell::CellValue;
use crate::spreadsheet::{CommandStatus, Spreadsheet};
// use petgraph::{
//     dot::{Config, Dot},
//     graph::{DiGraph, NodeIndex},
// };
use petgraph::{
    dot::{Config, Dot},
    graph::DiGraph,
};
// use std::collections::{HashMap, HashSet};
use std::collections::HashMap;
use std::{fs::File, io::Write, process::Command};

/// Visualizes the relationships of a specified cell, including direct and range-based parents and children.
///
/// This function creates a directed graph representing the cell's dependencies and dependents, saves it as a DOT file,
/// and attempts to render it as a PNG image using Graphviz. It also prints a textual representation of the relationships
/// to the console, including direct parents, range-based parents, direct children, and range-based children. The graph
/// includes nodes for the target cell, its parents, children, and any ranges it is part of, with labeled edges indicating
/// the nature of the relationships (e.g., "depends on", "used by", "part of range used by").
///
/// # Arguments
///
/// * `spreadsheet` - A reference to the `Spreadsheet` containing the cell data and relationships.
/// * `row` - The zero-based row index of the target cell.
/// * `col` - The zero-based column index of the target cell.
///
/// # Returns
///
/// * `CommandStatus::CmdOk` - If the visualization is successful or encounters non-critical errors (e.g., Graphviz not found).
/// * `CommandStatus::CmdInvalidCell` - If the specified row or column is out of bounds for the spreadsheet.
///
/// # Side Effects
///
/// * Creates a DOT file named `cell_{row}_{col}_relationships.dot` in the current directory.
/// * Attempts to open the PNG file with the default image viewer on the user's operating system (Windows, macOS, or Linux).
/// * Prints the textual representation of the cell's relationships to the console.
pub fn visualize_cell_relationships(
    spreadsheet: &Spreadsheet,
    row: i16,
    col: i16,
) -> CommandStatus {
    if row < 0 || row >= spreadsheet.rows || col < 0 || col >= spreadsheet.cols {
        return CommandStatus::InvalidCell;
    }

    // Get the cell key
    let cell_key = spreadsheet.get_key(row, col);

    // Create a directed graph for visualization
    let mut graph = DiGraph::<String, &str>::new();
    let mut node_indices = HashMap::new();

    // Function to get formatted cell name
    let get_cell_label = |key: i32| -> String {
        let (r, c) = spreadsheet.get_row_col(key);
        let col_name = spreadsheet.get_column_name(c);
        format!(
            "{}{} ({})",
            col_name,
            r + 1,
            match spreadsheet.grid[key as usize] {
                CellValue::Integer(val) => val.to_string(),
                CellValue::Error => "ERROR".to_string(),
            }
        )
    };

    // Add the target cell to the graph
    let target_label = get_cell_label(cell_key);
    let target_node = graph.add_node(target_label.clone());
    node_indices.insert(cell_key, target_node);

    // Process immediate parents (one level up)
    if let Some(meta) = spreadsheet.cell_meta.get(&cell_key) {
        // Check if this is a range-based formula (formula types 5-9)
        if (5..=9).contains(&meta.formula) {
            // For range formulas, parent1 is the start cell key and parent2 is the end cell key
            let start_key = meta.parent1;
            let end_key = meta.parent2;

            if start_key >= 0 && end_key >= 0 {
                // Get the row and column for range display
                let (start_r, start_c) = spreadsheet.get_row_col(start_key);
                let (end_r, end_c) = spreadsheet.get_row_col(end_key);

                // Create a special range node to represent the range
                let range_name = format!(
                    "Range {}{}:{}{}",
                    spreadsheet.get_column_name(start_c),
                    start_r + 1,
                    spreadsheet.get_column_name(end_c),
                    end_r + 1
                );

                let range_node = graph.add_node(range_name);

                // Add edge from range to the target cell
                graph.add_edge(range_node, target_node, "provides data to");
            }
        } else {
            // For normal formula types, handle individual parents
            for parent_key in [meta.parent1, meta.parent2].iter().filter(|&&k| k >= 0) {
                // Create parent node if it doesn't exist
                let parent_idx = if let Some(&idx) = node_indices.get(parent_key) {
                    idx
                } else {
                    let parent_label = get_cell_label(*parent_key);
                    let idx = graph.add_node(parent_label);
                    node_indices.insert(*parent_key, idx);
                    idx
                };

                // Add edge from parent to child
                let child_idx = node_indices[&cell_key];
                graph.add_edge(parent_idx, child_idx, "depends on");
            }
        }
    }

    // Process immediate children (one level down)
    if let Some(children) = spreadsheet.get_cell_children(cell_key) {
        for &child_key in children {
            // Create child node if it doesn't exist
            let child_idx = if let Some(&idx) = node_indices.get(&child_key) {
                idx
            } else {
                let child_label = get_cell_label(child_key);
                let idx = graph.add_node(child_label);
                node_indices.insert(child_key, idx);
                idx
            };

            // Add edge from parent to child
            let parent_idx = node_indices[&cell_key];
            graph.add_edge(parent_idx, child_idx, "used by");
        }
    }

    // Process range-based children (one level down)
    // Find range relationships where this cell is within the range
    for rc in &spreadsheet.range_children {
        if spreadsheet.is_cell_in_range(cell_key, rc.start_key, rc.end_key) {
            let child_key = rc.child_key;

            // Create child node if it doesn't exist
            let child_idx = if let Some(&idx) = node_indices.get(&child_key) {
                idx
            } else {
                let child_label = get_cell_label(child_key);
                let idx = graph.add_node(child_label);
                node_indices.insert(child_key, idx);
                idx
            };

            // Add edge from current cell to range-dependent child
            let parent_idx = node_indices[&cell_key];
            graph.add_edge(parent_idx, child_idx, "part of range used by");
        }
    }

    // Generate DOT format
    let dot = Dot::with_config(&graph, &[Config::EdgeNoLabel]);

    // Save to temp file
    let temp_file = format!("cell_{}_{}_relationships.dot", row, col);
    let mut file = match File::create(&temp_file) {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Failed to create dot file: {}", e);
            return CommandStatus::CmdOk;
        }
    };

    if let Err(e) = writeln!(file, "{:?}", dot) {
        eprintln!("Failed to write to dot file: {}", e);
        return CommandStatus::CmdOk;
    }

    println!("Cell relationships saved to {}", temp_file);

    // Attempt to render with Graphviz if available
    let output_file = format!("cell_{}_{}_relationships.png", row, col);
    match Command::new("dot")
        .args(["-Tpng", &temp_file, "-o", &output_file])
        .output()
    {
        Ok(_) => {
            println!("Cell relationship diagram generated as {}", output_file);
        }
        Err(_) => {
            println!("Graphviz not found. You can manually convert the .dot file to an image.");
            println!("For instance: dot -Tpng {} -o {}", temp_file, output_file);
        }
    }

    // Print textual representation of the relationships
    println!("\nCell {}{}:", spreadsheet.get_column_name(col), row + 1);

    // Show parents
    println!("  Parents:");
    let mut has_parents = false;

    // Check for range-based or direct parents
    if let Some(meta) = spreadsheet.cell_meta.get(&cell_key) {
        if (5..=9).contains(&meta.formula) {
            // Range-based parents
            has_parents = true;
            let start_key = meta.parent1;
            let end_key = meta.parent2;

            if start_key >= 0 && end_key >= 0 {
                let (start_r, start_c) = spreadsheet.get_row_col(start_key);
                let (end_r, end_c) = spreadsheet.get_row_col(end_key);

                // Determine the function type
                let function_name = match meta.formula {
                    5 => "SUM",
                    6 => "AVG",
                    7 => "MIN",
                    8 => "MAX",
                    9 => "STDEV",
                    _ => "UNKNOWN",
                };

                println!(
                    "    - Range function: {} on range {}{}:{}{}",
                    function_name,
                    spreadsheet.get_column_name(start_c),
                    start_r + 1,
                    spreadsheet.get_column_name(end_c),
                    end_r + 1
                );
            }
        } else {
            // Direct parent cells
            for parent_key in [meta.parent1, meta.parent2].iter().filter(|&&k| k >= 0) {
                has_parents = true;
                let (r, c) = spreadsheet.get_row_col(*parent_key);
                println!(
                    "    - {}{}: {}",
                    spreadsheet.get_column_name(c),
                    r + 1,
                    match spreadsheet.grid[*parent_key as usize] {
                        CellValue::Integer(val) => val.to_string(),
                        CellValue::Error => "ERROR".to_string(),
                    }
                );
            }
        }
    }

    if !has_parents {
        println!("    (none)");
    }

    // Show children
    println!("  Children:");
    let mut has_children = false;

    // Direct child cells
    if let Some(children) = spreadsheet.get_cell_children(cell_key) {
        if !children.is_empty() {
            for &child_key in children {
                has_children = true;
                let (r, c) = spreadsheet.get_row_col(child_key);
                println!(
                    "    - {}{}: {}",
                    spreadsheet.get_column_name(c),
                    r + 1,
                    match spreadsheet.grid[child_key as usize] {
                        CellValue::Integer(val) => val.to_string(),
                        CellValue::Error => "ERROR".to_string(),
                    }
                );
            }
        }
    }

    // Range-based children - cells that depend on a range which includes this cell
    for rc in &spreadsheet.range_children {
        if spreadsheet.is_cell_in_range(cell_key, rc.start_key, rc.end_key) {
            has_children = true;
            let (r, c) = spreadsheet.get_row_col(rc.child_key);
            println!(
                "    - {}{} (via range): {}",
                spreadsheet.get_column_name(c),
                r + 1,
                match spreadsheet.grid[rc.child_key as usize] {
                    CellValue::Integer(val) => val.to_string(),
                    CellValue::Error => "ERROR".to_string(),
                }
            );
        }
    }

    if !has_children {
        println!("    (none)");
    }

    CommandStatus::CmdOk
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
    fn test_visualize_cell_invalid() {
        let sheet = create_test_spreadsheet(5, 5);
        assert_eq!(
            visualize_cell_relationships(&sheet, 5, 5),
            CommandStatus::InvalidCell
        );
    }

    #[test]
    fn test_visualize_cell_with_parents() {
        let mut sheet = create_test_spreadsheet(5, 5);
        sheet.get_cell_meta(1, 1).parent1 = sheet.get_key(0, 0);
        assert_eq!(
            visualize_cell_relationships(&sheet, 1, 1),
            CommandStatus::CmdOk
        );
    }

    #[test]
    fn test_visualize_cell_with_children() {
        let mut sheet = create_test_spreadsheet(5, 5);
        sheet.add_child(&sheet.get_key(0, 0), &sheet.get_key(1, 1));
        assert_eq!(
            visualize_cell_relationships(&sheet, 0, 0),
            CommandStatus::CmdOk
        );
    }

    #[test]
    fn test_visualize_range_child() {
        let mut sheet = create_test_spreadsheet(5, 5);
        sheet.add_range_child(
            sheet.get_key(0, 0),
            sheet.get_key(1, 1),
            sheet.get_key(2, 2),
        );
        assert_eq!(
            visualize_cell_relationships(&sheet, 2, 2),
            CommandStatus::CmdOk
        );
    }

    #[test]
    fn test_visualize_cell_with_multiple_parents() {
        let mut sheet = create_test_spreadsheet(5, 5);
        sheet.get_cell_meta(1, 1).parent1 = sheet.get_key(0, 0);
        sheet.get_cell_meta(1, 1).parent2 = sheet.get_key(0, 1);
        assert_eq!(
            visualize_cell_relationships(&sheet, 1, 1),
            CommandStatus::CmdOk
        );
    }

    #[test]
    fn test_visualize_cell_no_relationships() {
        let sheet = create_test_spreadsheet(5, 5);
        assert_eq!(
            visualize_cell_relationships(&sheet, 0, 0),
            CommandStatus::CmdOk
        );
    }

    #[test]
    fn test_visualize_range_parent() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let cell_key = sheet.get_key(2, 2);
        sheet.range_children.push(crate::spreadsheet::RangeChild {
            start_key: sheet.get_key(0, 0),
            end_key: sheet.get_key(1, 1),
            child_key: cell_key,
        });
        assert_eq!(
            visualize_cell_relationships(&sheet, 2, 2),
            CommandStatus::CmdOk
        );
    }

    #[test]
    fn test_visualize_cell_in_range_child() {
        let mut sheet = create_test_spreadsheet(5, 5);
        let cell_key = sheet.get_key(0, 0);
        sheet.range_children.push(crate::spreadsheet::RangeChild {
            start_key: cell_key,
            end_key: sheet.get_key(1, 1),
            child_key: sheet.get_key(2, 2),
        });
        assert_eq!(
            visualize_cell_relationships(&sheet, 0, 0),
            CommandStatus::CmdOk
        );
    }

    #[test]
    fn test_visualize_cell_with_large_integer() {
        let mut sheet = create_test_spreadsheet(5, 5);
        *sheet.get_mut_cell(0, 0) = CellValue::Integer(1_000_000);
        assert_eq!(
            visualize_cell_relationships(&sheet, 0, 0),
            CommandStatus::CmdOk
        );
    }

    #[test]
    fn test_visualize_cell_negative_coordinates() {
        let sheet = create_test_spreadsheet(5, 5);
        assert_eq!(
            visualize_cell_relationships(&sheet, -1, 0),
            CommandStatus::InvalidCell
        );
    }
}
