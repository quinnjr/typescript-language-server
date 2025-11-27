use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Represents the dependency graph of files in a project
#[derive(Debug, Default)]
pub struct FileGraph {
    /// Files that each file imports
    imports: HashMap<PathBuf, HashSet<PathBuf>>,
    /// Files that import each file (reverse mapping)
    importers: HashMap<PathBuf, HashSet<PathBuf>>,
}

impl FileGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a file to the graph
    pub fn add_file(&mut self, path: PathBuf) {
        self.imports.entry(path.clone()).or_default();
        self.importers.entry(path).or_default();
    }

    /// Remove a file from the graph
    pub fn remove_file(&mut self, path: &Path) {
        // Remove from imports
        if let Some(imported) = self.imports.remove(path) {
            // Remove this file from the importers of each imported file
            for imported_path in imported {
                if let Some(importers) = self.importers.get_mut(&imported_path) {
                    importers.remove(path);
                }
            }
        }

        // Remove from importers
        if let Some(importers) = self.importers.remove(path) {
            // Remove this file from the imports of each importer
            for importer_path in importers {
                if let Some(imports) = self.imports.get_mut(&importer_path) {
                    imports.remove(path);
                }
            }
        }
    }

    /// Add an import relationship
    pub fn add_import(&mut self, from: &Path, to: &Path) {
        self.imports
            .entry(from.to_path_buf())
            .or_default()
            .insert(to.to_path_buf());

        self.importers
            .entry(to.to_path_buf())
            .or_default()
            .insert(from.to_path_buf());
    }

    /// Remove an import relationship
    pub fn remove_import(&mut self, from: &Path, to: &Path) {
        if let Some(imports) = self.imports.get_mut(from) {
            imports.remove(to);
        }

        if let Some(importers) = self.importers.get_mut(to) {
            importers.remove(from);
        }
    }

    /// Get all files that a file imports
    pub fn get_imports(&self, path: &Path) -> Option<&HashSet<PathBuf>> {
        self.imports.get(path)
    }

    /// Get all files that import a given file
    pub fn get_importers(&self, path: &Path) -> Option<&HashSet<PathBuf>> {
        self.importers.get(path)
    }

    /// Get all files that would be affected by a change to the given file
    /// (all files that directly or indirectly import this file)
    pub fn get_affected_files(&self, changed_path: &Path) -> HashSet<PathBuf> {
        let mut affected = HashSet::new();
        let mut to_visit = vec![changed_path.to_path_buf()];

        while let Some(current) = to_visit.pop() {
            if affected.contains(&current) {
                continue;
            }
            affected.insert(current.clone());

            if let Some(importers) = self.importers.get(&current) {
                for importer in importers {
                    if !affected.contains(importer) {
                        to_visit.push(importer.clone());
                    }
                }
            }
        }

        affected
    }

    /// Get all dependencies of a file (all files it directly or indirectly imports)
    pub fn get_dependencies(&self, path: &Path) -> HashSet<PathBuf> {
        let mut deps = HashSet::new();
        let mut to_visit = vec![path.to_path_buf()];

        while let Some(current) = to_visit.pop() {
            if deps.contains(&current) {
                continue;
            }
            deps.insert(current.clone());

            if let Some(imports) = self.imports.get(&current) {
                for import in imports {
                    if !deps.contains(import) {
                        to_visit.push(import.clone());
                    }
                }
            }
        }

        deps
    }

    /// Check if the graph has a cycle involving the given file
    pub fn has_cycle(&self, start: &Path) -> bool {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        self.has_cycle_util(start, &mut visited, &mut rec_stack)
    }

    fn has_cycle_util(
        &self,
        path: &Path,
        visited: &mut HashSet<PathBuf>,
        rec_stack: &mut HashSet<PathBuf>,
    ) -> bool {
        let path_buf = path.to_path_buf();

        if rec_stack.contains(&path_buf) {
            return true;
        }

        if visited.contains(&path_buf) {
            return false;
        }

        visited.insert(path_buf.clone());
        rec_stack.insert(path_buf.clone());

        if let Some(imports) = self.imports.get(path) {
            for import in imports {
                if self.has_cycle_util(import, visited, rec_stack) {
                    return true;
                }
            }
        }

        rec_stack.remove(&path_buf);
        false
    }

    /// Clear all import relationships for a file (used before re-analyzing)
    pub fn clear_imports(&mut self, path: &Path) {
        if let Some(imports) = self.imports.remove(path) {
            for imported in imports {
                if let Some(importers) = self.importers.get_mut(&imported) {
                    importers.remove(path);
                }
            }
        }
        self.imports.insert(path.to_path_buf(), HashSet::new());
    }
}
