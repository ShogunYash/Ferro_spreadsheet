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

// pub fn visualize_cell_relationships(
//     spreadsheet: &Spreadsheet,
//     row: i16,
//     col: i16,
// ) -> CommandStatus {
//     // ...move the entire body of the function from spreadsheet.rs here, replacing `spreadsheet` with `spreadsheet`...
//     if row < 0 || row >= spreadsheet.rows || col < 0 || col >= spreadsheet.cols {
//         return CommandStatus::CmdInvalidCell;
//     }

//     // Get the cell key
//     let cell_key = spreadsheet.get_key(row, col);

//     // Create a directed graph for visualization
//     let mut graph = DiGraph::<String, &str>::new();
//     let mut node_indices = HashMap::new();

//     // Function to get formatted cell name
//     let get_cell_label = |key: i32| -> String {
//         let (r, c) = spreadsheet.get_row_col(key);
//         let col_name = spreadsheet.get_column_name(c);
//         format!(
//             "{}{} ({})",
//             col_name,
//             r + 1,
//             match spreadsheet.grid[key as usize] {
//                 CellValue::Integer(val) => val.to_string(),
//                 CellValue::Error => "ERROR".to_string(),
//             }
//         )
//     };

//     // Add the target cell to the graph
//     let target_label = get_cell_label(cell_key);
//     let target_node = graph.add_node(target_label.clone());
//     node_indices.insert(cell_key, target_node);

//     // Helper function to process relationships
//     fn process_relationships(
//         spreadsheet: &Spreadsheet,
//         start_key: i32,
//         is_parent_direction: bool,
//         processed: &mut HashSet<i32>,
//         depth_limit: usize,
//         graph: &mut DiGraph<String, &str>,
//         node_indices: &mut HashMap<i32, NodeIndex>,
//         get_cell_label: &dyn Fn(i32) -> String,
//     ) {
//         if !processed.insert(start_key) {
//             return; // Already processed this cell
//         }

//         // Add appropriate relationships based on direction
//         if is_parent_direction {
//             // Add parents - traverse up the dependency tree
//             if let Some(meta) = spreadsheet.cell_meta.get(&start_key) {
//                 for parent_key in [meta.parent1, meta.parent2].iter().filter(|&&k| k >= 0) {
//                     // Create parent node if it doesn't exist
//                     let parent_idx = if let Some(&idx) = node_indices.get(parent_key) {
//                         idx
//                     } else {
//                         let parent_label = get_cell_label(*parent_key);
//                         let idx = graph.add_node(parent_label);
//                         node_indices.insert(*parent_key, idx);
//                         idx
//                     };

//                     // Add edge from parent to child
//                     let child_idx = node_indices[&start_key];
//                     graph.add_edge(parent_idx, child_idx, "depends on");

//                     // Recurse for this parent (up to the depth limit)
//                     if processed.len() < depth_limit {
//                         process_relationships(
//                             spreadsheet,
//                             *parent_key,
//                             true,
//                             processed,
//                             depth_limit,
//                             graph,
//                             node_indices,
//                             get_cell_label,
//                         );
//                     }
//                 }
//             }
//         } else {
//             // Add children - traverse down the dependency tree
//             if let Some(children) = spreadsheet.get_cell_children(start_key) {
//                 for &child_key in children {
//                     // Create child node if it doesn't exist
//                     let child_idx = if let Some(&idx) = node_indices.get(&child_key) {
//                         idx
//                     } else {
//                         let child_label = get_cell_label(child_key);
//                         let idx = graph.add_node(child_label);
//                         node_indices.insert(child_key, idx);
//                         idx
//                     };

//                     // Add edge from parent to child
//                     let parent_idx = node_indices[&start_key];
//                     graph.add_edge(parent_idx, child_idx, "used by");

//                     // Recurse for this child (up to the depth limit)
//                     if processed.len() < depth_limit {
//                         process_relationships(
//                             spreadsheet,
//                             child_key,
//                             false,
//                             processed,
//                             depth_limit,
//                             graph,
//                             node_indices,
//                             get_cell_label,
//                         );
//                     }
//                 }
//             }
//         }
//     }

//     // Process parents (upward traversal)
//     let mut processed = HashSet::new();
//     // Mark target cell as processed
//     process_relationships(
//         spreadsheet,
//         cell_key,
//         true,
//         &mut processed,
//         20,
//         &mut graph,
//         &mut node_indices,
//         &get_cell_label,
//     );

//     // Process children (downward traversal)
//     let mut processed = HashSet::new();
//     // Mark target cell as processed
//     process_relationships(
//         spreadsheet,
//         cell_key,
//         false,
//         &mut processed,
//         20,
//         &mut graph,
//         &mut node_indices,
//         &get_cell_label,
//     );

//     // Generate DOT format
//     let dot = Dot::with_config(&graph, &[Config::EdgeNoLabel]);

//     // Save to temp file
//     let temp_file = format!("cell_{}_{}_relationships.dot", row, col);
//     let mut file = match File::create(&temp_file) {
//         Ok(file) => file,
//         Err(e) => {
//             eprintln!("Failed to create dot file: {}", e);
//             return CommandStatus::CmdOk;
//         }
//     };

//     if let Err(e) = writeln!(file, "{:?}", dot) {
//         eprintln!("Failed to write to dot file: {}", e);
//         return CommandStatus::CmdOk;
//     }

//     println!("Cell relationships saved to {}", temp_file);

//     // Attempt to render with Graphviz if available
//     let output_file = format!("cell_{}_{}_relationships.png", row, col);
//     match Command::new("dot")
//         .args(["-Tpng", &temp_file, "-o", &output_file])
//         .output()
//     {
//         Ok(_) => {
//             println!("Cell relationship diagram generated as {}", output_file);
//             // Try to open the image with the default viewer
//             #[cfg(target_os = "windows")]
//             let _ = Command::new("cmd").args(["/C", &output_file]).spawn();

//             #[cfg(target_os = "macos")]
//             let _ = Command::new("open").arg(&output_file).spawn();

//             #[cfg(target_os = "linux")]
//             let _ = Command::new("xdg-open").arg(&output_file).spawn();
//         }
//         Err(_) => {
//             println!("Graphviz not found. You can manually convert the .dot file to an image.");
//             println!("For instance: dot -Tpng {} -o {}", temp_file, output_file);
//         }
//     }

//     // Print textual representation of the relationships
//     println!("\nCell {}{}:", spreadsheet.get_column_name(col), row + 1);

//     // Show parents
//     if let Some(meta) = spreadsheet.cell_meta.get(&cell_key) {
//         println!("  Parents:");
//         let mut has_parents = false;

//         for parent_key in [meta.parent1, meta.parent2].iter().filter(|&&k| k >= 0) {
//             has_parents = true;
//             let (r, c) = spreadsheet.get_row_col(*parent_key);
//             println!(
//                 "    - {}{}: {}",
//                 spreadsheet.get_column_name(c),
//                 r + 1,
//                 match spreadsheet.grid[*parent_key as usize] {
//                     CellValue::Integer(val) => val.to_string(),
//                     CellValue::Error => "ERROR".to_string(),
//                 }
//             );
//         }

//         if !has_parents {
//             println!("    (none)");
//         }
//     }

//     // Show children
//     println!("  Children:");
//     if let Some(children) = spreadsheet.get_cell_children(cell_key) {
//         if !children.is_empty() {
//             for &child_key in children {
//                 let (r, c) = spreadsheet.get_row_col(child_key);
//                 println!(
//                     "    - {}{}: {}",
//                     spreadsheet.get_column_name(c),
//                     r + 1,
//                     match spreadsheet.grid[child_key as usize] {
//                         CellValue::Integer(val) => val.to_string(),
//                         CellValue::Error => "ERROR".to_string(),
//                     }
//                 );
//             }
//         } else {
//             println!("    (none)");
//         }
//     } else {
//         println!("    (none)");
//     }

//     CommandStatus::CmdOk
// }
pub fn visualize_cell_relationships(
    spreadsheet: &Spreadsheet,
    row: i16,
    col: i16,
) -> CommandStatus {
    if row < 0 || row >= spreadsheet.rows || col < 0 || col >= spreadsheet.cols {
        return CommandStatus::CmdInvalidCell;
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
    
    // Process range-based parents (one level up)
    // Find range relationships where this cell is the child
    for rc in &spreadsheet.range_children {
        if rc.child_key == cell_key {
            // Create a special node to represent the range
            let (start_row, start_col) = spreadsheet.get_row_col(rc.start_key);
            let (end_row, end_col) = spreadsheet.get_row_col(rc.end_key);
            let range_label = format!(
                "Range {}{}:{}{}", 
                spreadsheet.get_column_name(start_col),
                start_row + 1,
                spreadsheet.get_column_name(end_col),
                end_row + 1
            );
            
            let range_node = graph.add_node(range_label);
            
            // Add an edge from the range to the child
            graph.add_edge(range_node, node_indices[&cell_key], "range depends on");
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
            // Try to open the image with the default viewer
            #[cfg(target_os = "windows")]
            let _ = Command::new("cmd").args(["/C", &output_file]).spawn();

            #[cfg(target_os = "macos")]
            let _ = Command::new("open").arg(&output_file).spawn();

            #[cfg(target_os = "linux")]
            let _ = Command::new("xdg-open").arg(&output_file).spawn();
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

    // Direct parent cells
    if let Some(meta) = spreadsheet.cell_meta.get(&cell_key) {
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
    
    // Range-based parents
    for rc in &spreadsheet.range_children {
        if rc.child_key == cell_key {
            has_parents = true;
            let (start_row, start_col) = spreadsheet.get_row_col(rc.start_key);
            let (end_row, end_col) = spreadsheet.get_row_col(rc.end_key);
            println!(
                "    - Range {}{}:{}{}",
                spreadsheet.get_column_name(start_col),
                start_row + 1,
                spreadsheet.get_column_name(end_col),
                end_row + 1
            );
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