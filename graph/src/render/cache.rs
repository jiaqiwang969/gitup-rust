use std::collections::HashMap;
use crate::layout::Row;

pub struct RenderCache {
    row_cache: HashMap<String, Vec<crate::render::Cell>>,
    dirty: bool,
}

impl RenderCache {
    pub fn new() -> Self {
        Self {
            row_cache: HashMap::new(),
            dirty: false,
        }
    }

    pub fn get(&self, commit_id: &str) -> Option<&Vec<crate::render::Cell>> {
        self.row_cache.get(commit_id)
    }

    pub fn insert(&mut self, commit_id: String, cells: Vec<crate::render::Cell>) {
        self.row_cache.insert(commit_id, cells);
    }

    pub fn invalidate(&mut self) {
        self.dirty = true;
        self.row_cache.clear();
    }
}
