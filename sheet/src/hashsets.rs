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