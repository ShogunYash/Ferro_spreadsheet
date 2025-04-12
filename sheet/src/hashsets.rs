use smallvec::SmallVec;
use std::collections::{HashMap, HashSet, VecDeque};
use std::num::NonZeroU32;

// Cell identifier (row, column)
type CellId = (u16, u16);

pub struct Cell {
    // ... your existing fields
    pub value: CellValue,
    pub formula: i16,
    
    // For dependency tracking (each cell tracks what it depends on)
    // Using Option<NonZeroU16> as index into DependencyTable
    pub dependencies_index: Option<NonZeroU16>, // 2 bytes when Some
    
    // For reverse lookup (what cells depend on this cell)
    // Using Option<NonZeroU16> as index into DependencyTable
    pub dependents_index: Option<NonZeroU16>,   // 2 bytes when Some
    
    // Dirty flag to mark cells needing recalculation
    pub is_dirty: bool,                        // 1 byte
}


pub struct DependencyTable {
    // Maps from index to actual cell IDs
    entries: Vec<SmallVec<[CellId; 4]>>,
    // Track freed indices for reuse
    free_indices: Vec<u16>,
}

impl DependencyTable {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            free_indices: Vec::new(),
        }
    }

    // Add a new dependency collection and return its index
    pub fn add_deps(&mut self, deps: SmallVec<[CellId; 4]>) -> NonZeroU16 {
        if let Some(reused_index) = self.free_indices.pop() {
            self.entries[reused_index as usize] = deps;
            NonZeroU16::new(reused_index + 1).unwrap()
        } else {
            let index = self.entries.len();
            self.entries.push(deps);
            assert!(index < u16::MAX as usize);
            NonZeroU16::new((index + 1) as u16).unwrap()
        }
    }

    pub fn get_deps(&self, index: NonZeroU16) -> &SmallVec<[CellId; 4]> {
        &self.entries[(index.get() - 1) as usize]
    }

    pub fn get_deps_mut(&mut self, index: NonZeroU16) -> &mut SmallVec<[CellId; 4]> {
        &mut self.entries[(index.get() - 1) as usize]
    }

    // Add a single dependency
    pub fn add_single_dep(&mut self, index: Option<NonZeroU16>, dep: CellId) -> NonZeroU16 {
        match index {
            Some(idx) => {
                let deps = self.get_deps_mut(idx);
                if !deps.contains(&dep) {
                    deps.push(dep);
                }
                idx
            }
            None => {
                let mut deps = SmallVec::new();
                deps.push(dep);
                self.add_deps(deps)
            }
        }
    }
}
pub struct Spreadsheet {
    cells: HashMap<CellId, Cell>,
    dependency_table: DependencyTable,
    dependent_table: DependencyTable,
    
    // Queue for cells that need recalculation
    evaluation_queue: VecDeque<CellId>,
}

impl Spreadsheet {
    pub fn new() -> Self {
        Self {
            cells: HashMap::new(),
            dependency_table: DependencyTable::new(),
            dependent_table: DependencyTable::new(),
            evaluation_queue: VecDeque::new(),
        }
    }

    // Set a cell's value and mark dependent cells as dirty
    pub fn set_cell_value(&mut self, cell_id: CellId, value: CellValue) {
        // Update the cell value
        if let Some(cell) = self.cells.get_mut(&cell_id) {
            cell.value = value;
            
            // Mark all dependent cells as dirty and add to evaluation queue
            if let Some(idx) = cell.dependents_index {
                let dependents = self.dependent_table.get_deps(idx);
                for &dependent_id in dependents {
                    if let Some(dependent_cell) = self.cells.get_mut(&dependent_id) {
                        dependent_cell.is_dirty = true;
                        self.evaluation_queue.push_back(dependent_id);
                    }
                }
            }
        } else {
            // Cell doesn't exist yet, create it
            let mut new_cell = Cell {
                // Initialize other fields
                dependencies_index: None,
                dependents_index: None,
                value,
                formula: 0, // Default formula
                is_dirty: false,
            };
            
            self.cells.insert(cell_id, new_cell);
        }
    }

    // Update a cell's formula and dependencies
    pub fn set_cell_formula(&mut self, cell_id: CellId, formula: i16, dependencies: Vec<CellId>) {
        // Get or create the cell
        let cell = self.cells.entry(cell_id).or_insert_with(|| Cell {
            // Initialize other fields
            dependencies_index: None,
            dependents_index: None,
            value: CellValue::Empty, // Default value
            formula: 0,
            is_dirty: true,
        });
        
        // Update formula
        cell.formula = formula;
        
        // Clear old dependencies if they exist
        if let Some(old_deps_idx) = cell.dependencies_index {
            let old_deps = self.dependency_table.get_deps(old_deps_idx).clone();
            
            // Remove this cell from each dependency's dependents list
            for &dep_id in &old_deps {
                if let Some(dep_cell) = self.cells.get_mut(&dep_id) {
                    if let Some(dep_deps_idx) = dep_cell.dependents_index {
                        let dependents = self.dependent_table.get_deps_mut(dep_deps_idx);
                        if let Some(pos) = dependents.iter().position(|&id| id == cell_id) {
                            dependents.swap_remove(pos);
                        }
                    }
                }
            }
        }
        
        // Add new dependencies
        let mut deps_vec = SmallVec::new();
        for dep_id in &dependencies {
            deps_vec.push(*dep_id);
            
            // Add this cell to each dependency's dependents list
            let dep_cell = self.cells.entry(*dep_id).or_insert_with(|| Cell {
                // Initialize other fields
                dependencies_index: None,
                dependents_index: None,
                value: CellValue::Empty,
                formula: 0,
                is_dirty: false,
            });
            
            dep_cell.dependents_index = Some(
                self.dependent_table.add_single_dep(dep_cell.dependents_index, cell_id)
            );
        }
        
        // Set the cell's dependencies
        if !deps_vec.is_empty() {
            cell.dependencies_index = Some(self.dependency_table.add_deps(deps_vec));
        } else {
            cell.dependencies_index = None;
        }
        
        // Mark cell as dirty and add to evaluation queue
        cell.is_dirty = true;
        self.evaluation_queue.push_back(cell_id);
    }

    // Recalculate all dirty cells
    pub fn recalculate(&mut self) {
        // Process cells in the evaluation queue
        while let Some(cell_id) = self.evaluation_queue.pop_front() {
            // Skip if cell is not dirty anymore (might have been processed already)
            if let Some(cell) = self.cells.get(&cell_id) {
                if !cell.is_dirty {
                    continue;
                }
            } else {
                continue;
            }
            
            // Check if dependencies are all up-to-date
            let mut dependencies_dirty = false;
            if let Some(cell) = self.cells.get(&cell_id) {
                if let Some(deps_idx) = cell.dependencies_index {
                    let deps = self.dependency_table.get_deps(deps_idx);
                    for &dep_id in deps {
                        if let Some(dep_cell) = self.cells.get(&dep_id) {
                            if dep_cell.is_dirty {
                                dependencies_dirty = true;
                                break;
                            }
                        }
                    }
                }
            }
            
            // If dependencies are dirty, move this cell to the back of the queue
            if dependencies_dirty {
                self.evaluation_queue.push_back(cell_id);
                continue;
            }
            
            // All dependencies are clean, evaluate this cell
            if let Some(cell) = self.cells.get_mut(&cell_id) {
                // Evaluate the formula based on its dependencies
                let new_value = self.evaluate_formula(cell_id, cell.formula);
                
                // Update the cell's value
                let value_changed = cell.value != new_value;
                cell.value = new_value;
                cell.is_dirty = false;
                
                // If value changed, mark dependents as dirty
                if value_changed {
                    if let Some(deps_idx) = cell.dependents_index {
                        let dependents = self.dependent_table.get_deps(deps_idx);
                        for &dependent_id in dependents {
                            if let Some(dependent_cell) = self.cells.get_mut(&dependent_id) {
                                dependent_cell.is_dirty = true;
                                self.evaluation_queue.push_back(dependent_id);
                            }
                        }
                    }
                }
            }
        }
    }

    // Evaluate a cell's formula
    fn evaluate_formula(&self, cell_id: CellId, formula: i16) -> CellValue {
        // This is where you would implement your formula evaluation logic
        // For now, just a placeholder that returns a default value
        CellValue::Number(0.0)
    }
}