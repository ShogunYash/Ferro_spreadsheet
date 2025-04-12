// Cell identifier (row, column)
type CellId = (u16, u16);

pub struct Cell {
    // Original fields
    pub value: CellValue,
    pub formula: i16,
    
    // Store parent cells directly in the Cell struct
    // Since there are at most 2 parents, we can store them inline
    pub parent1: Option<CellId>, // Store first parent directly
    pub parent2: Option<CellId>, // Store second parent directly
    
    // For reverse lookup (what cells depend on this cell)
    // Using Option<NonZeroU16> as index into DependencyTable
    pub dependents_index: Option<NonZeroU16>, // 2 bytes when Some
    
    // Dirty flag to mark cells needing recalculation
    pub is_dirty: bool, // 1 byte
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

    // Add a new dependent collection and return its index
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

    // Add a single dependent
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

    // Remove a dependent
    pub fn remove_dep(&mut self, index: NonZeroU16, dep: CellId) -> Option<NonZeroU16> {
        let deps = self.get_deps_mut(index);
        if let Some(pos) = deps.iter().position(|&id| id == dep) {
            deps.swap_remove(pos);
            
            // If the dependent list is now empty, free the index
            if deps.is_empty() {
                self.free_indices.push((index.get() - 1) as u16);
                return None;
            }
        }
        Some(index)
    }
}
pub struct Spreadsheet {
    cells: HashMap<CellId, Cell>,
    dependent_table: DependencyTable,
    
    // Queue for cells that need recalculation
    evaluation_queue: VecDeque<CellId>,
}

impl Spreadsheet {
    pub fn new() -> Self {
        Self {
            cells: HashMap::new(),
            dependent_table: DependencyTable::new(),
            evaluation_queue: VecDeque::new(),
        }
    }

    // Set a cell's value and mark dependent cells as dirty
    pub fn set_cell_value(&mut self, cell_id: CellId, value: CellValue) {
        // Update the cell value
        if let Some(cell) = self.cells.get_mut(&cell_id) {
            let value_changed = cell.value != value;
            cell.value = value;
            
            // Only propagate changes if the value actually changed
            if value_changed {
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
            }
        } else {
            // Cell doesn't exist yet, create it
            let new_cell = Cell {
                // Initialize other fields
                parent1: None,
                parent2: None,
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
        // Validate the number of dependencies
        assert!(dependencies.len() <= 2, "A cell can have at most 2 parents");
        
        // Get or create the cell
        let cell = self.cells.entry(cell_id).or_insert_with(|| Cell {
            parent1: None,
            parent2: None,
            dependents_index: None,
            value: CellValue::Empty, // Default value
            formula: 0,
            is_dirty: true,
        });
        
        // Update formula
        cell.formula = formula;
        
        // Remove this cell from old parents' dependents lists
        if let Some(old_parent) = cell.parent1 {
            if let Some(parent_cell) = self.cells.get_mut(&old_parent) {
                if let Some(deps_idx) = parent_cell.dependents_index {
                    parent_cell.dependents_index = self.dependent_table.remove_dep(deps_idx, cell_id);
                }
            }
        }
        
        if let Some(old_parent) = cell.parent2 {
            if let Some(parent_cell) = self.cells.get_mut(&old_parent) {
                if let Some(deps_idx) = parent_cell.dependents_index {
                    parent_cell.dependents_index = self.dependent_table.remove_dep(deps_idx, cell_id);
                }
            }
        }
        
        // Clear old parents
        cell.parent1 = None;
        cell.parent2 = None;
        
        // Add new parents
        for (i, &dep_id) in dependencies.iter().enumerate() {
            if i == 0 {
                cell.parent1 = Some(dep_id);
            } else if i == 1 {
                cell.parent2 = Some(dep_id);
            }
            
            // Add this cell to the parent's dependents list
            let parent_cell = self.cells.entry(dep_id).or_insert_with(|| Cell {
                parent1: None,
                parent2: None,
                dependents_index: None,
                value: CellValue::Empty,
                formula: 0,
                is_dirty: false,
            });
            
            parent_cell.dependents_index = Some(
                self.dependent_table.add_single_dep(parent_cell.dependents_index, cell_id)
            );
        }
        
        // Mark cell as dirty and add to evaluation queue
        cell.is_dirty = true;
        self.evaluation_queue.push_back(cell_id);
    }

    // Recalculate all dirty cells
    pub fn recalculate(&mut self) {
        // Process cells in the evaluation queue
        while let Some(cell_id) = self.evaluation_queue.pop_front() {
            // Skip if cell is not dirty anymore
            if let Some(cell) = self.cells.get(&cell_id) {
                if !cell.is_dirty {
                    continue;
                }
            } else {
                continue;
            }
            
            // Check if parents are all up-to-date
            let mut parents_dirty = false;
            if let Some(cell) = self.cells.get(&cell_id) {
                // Check first parent
                if let Some(parent_id) = cell.parent1 {
                    if let Some(parent_cell) = self.cells.get(&parent_id) {
                        if parent_cell.is_dirty {
                            parents_dirty = true;
                        }
                    }
                }
                
                // Check second parent
                if !parents_dirty && let Some(parent_id) = cell.parent2 {
                    if let Some(parent_cell) = self.cells.get(&parent_id) {
                        if parent_cell.is_dirty {
                            parents_dirty = true;
                        }
                    }
                }
            }
            
            // If parents are dirty, move this cell to the back of the queue
            if parents_dirty {
                self.evaluation_queue.push_back(cell_id);
                continue;
            }
            
            // All parents are clean, evaluate this cell
            if let Some(cell) = self.cells.get_mut(&cell_id) {
                // Get parent values for evaluation
                let parent1_value = cell.parent1.and_then(|id| 
                    self.cells.get(&id).map(|c| c.value.clone())
                );
                
                let parent2_value = cell.parent2.and_then(|id| 
                    self.cells.get(&id).map(|c| c.value.clone())
                );
                
                // Evaluate the formula based on parent values
                let new_value = self.evaluate_formula(
                    cell_id, 
                    cell.formula, 
                    parent1_value, 
                    parent2_value
                );
                
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
    fn evaluate_formula(
        &self, 
        cell_id: CellId, 
        formula: i16, 
        parent1_value: Option<CellValue>, 
        parent2_value: Option<CellValue>
    ) -> CellValue {
        // This is where you implement your formula evaluation logic
        // Formula code can directly use parent1_value and parent2_value
        CellValue::Number(0.0) // Placeholder
    }
}