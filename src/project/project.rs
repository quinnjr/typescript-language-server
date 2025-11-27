//! Project management
//! Reserved for multi-file project features

#![allow(dead_code)]

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::resolution::ModuleResolver;
use crate::resolution::tsconfig::TsConfig;

use super::FileGraph;

/// Represents a TypeScript project (usually corresponds to a tsconfig.json)
pub struct Project {
    /// Root directory of the project
    pub root: PathBuf,
    /// Path to tsconfig.json
    pub config_path: Option<PathBuf>,
    /// Parsed tsconfig
    pub config: Option<TsConfig>,
    /// Module resolver for this project
    pub resolver: ModuleResolver,
    /// Files included in this project
    pub files: HashSet<PathBuf>,
    /// File dependency graph
    pub file_graph: FileGraph,
}

impl Project {
    /// Create a new project from a directory
    pub fn new(root: PathBuf) -> Self {
        Self {
            root: root.clone(),
            config_path: None,
            config: None,
            resolver: ModuleResolver::new(root),
            files: HashSet::new(),
            file_graph: FileGraph::new(),
        }
    }

    /// Create a project from a tsconfig.json file
    pub fn from_tsconfig(config_path: PathBuf) -> Result<Self, String> {
        let root = config_path.parent().unwrap_or(Path::new(".")).to_path_buf();

        let config = TsConfig::load(&config_path).map_err(|e| e.to_string())?;

        let resolver = ModuleResolver::with_tsconfig(root.clone(), &config);

        let mut project = Self {
            root,
            config_path: Some(config_path),
            config: Some(config),
            resolver,
            files: HashSet::new(),
            file_graph: FileGraph::new(),
        };

        // Discover project files
        project.discover_files()?;

        Ok(project)
    }

    /// Discover files based on tsconfig include/exclude patterns
    fn discover_files(&mut self) -> Result<(), String> {
        // Clone patterns to avoid borrow issues
        let include_patterns: Vec<String> = match &self.config {
            Some(c) => c.include.clone().unwrap_or_default(),
            None => return Ok(()),
        };

        if include_patterns.is_empty() {
            // Default: include all .ts/.tsx files in the project
            self.discover_default_files()?;
        } else {
            // Use include patterns
            for pattern in &include_patterns {
                self.discover_files_by_pattern(pattern)?;
            }
        }

        Ok(())
    }

    /// Discover files with default pattern (all .ts/.tsx in project)
    fn discover_default_files(&mut self) -> Result<(), String> {
        let extensions = ["ts", "tsx", "mts", "cts"];

        if let Ok(entries) = std::fs::read_dir(&self.root) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if extensions.contains(&ext.to_string_lossy().as_ref()) {
                            self.files.insert(path);
                        }
                    }
                } else if path.is_dir() {
                    // Skip node_modules and hidden directories
                    if let Some(name) = path.file_name() {
                        let name = name.to_string_lossy();
                        if !name.starts_with('.') && name != "node_modules" {
                            self.discover_files_in_dir(&path, &extensions)?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Recursively discover files in a directory
    fn discover_files_in_dir(&mut self, dir: &Path, extensions: &[&str]) -> Result<(), String> {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if extensions.contains(&ext.to_string_lossy().as_ref()) {
                            self.files.insert(path);
                        }
                    }
                } else if path.is_dir() {
                    if let Some(name) = path.file_name() {
                        let name = name.to_string_lossy();
                        if !name.starts_with('.') && name != "node_modules" {
                            self.discover_files_in_dir(&path, extensions)?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Discover files by glob pattern
    fn discover_files_by_pattern(&mut self, pattern: &str) -> Result<(), String> {
        // Simple pattern handling - supports ** and *
        let pattern_path = self.root.join(pattern);
        let pattern_str = pattern_path.to_string_lossy();

        if pattern_str.contains("**") {
            // Recursive pattern
            let parts: Vec<&str> = pattern.split("**").collect();
            if let Some(base) = parts.first() {
                let base_path = self.root.join(base.trim_end_matches('/'));
                if base_path.is_dir() {
                    let extensions = ["ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs"];
                    self.discover_files_in_dir(&base_path, &extensions)?;
                }
            }
        } else if pattern_str.contains('*') {
            // Single-level wildcard
            if let Some(parent) = pattern_path.parent() {
                if parent.is_dir() {
                    if let Ok(entries) = std::fs::read_dir(parent) {
                        for entry in entries.filter_map(|e| e.ok()) {
                            let path = entry.path();
                            if path.is_file() {
                                // Simple matching - could be improved
                                self.files.insert(path);
                            }
                        }
                    }
                }
            }
        } else if pattern_path.is_file() {
            self.files.insert(pattern_path);
        }

        Ok(())
    }

    /// Add a file to the project
    pub fn add_file(&mut self, path: PathBuf) {
        self.files.insert(path);
    }

    /// Remove a file from the project
    pub fn remove_file(&mut self, path: &Path) {
        self.files.remove(path);
        self.file_graph.remove_file(path);
    }

    /// Check if a file is part of this project
    pub fn contains_file(&self, path: &Path) -> bool {
        self.files.contains(path)
    }

    /// Get all files in the project
    pub fn get_files(&self) -> impl Iterator<Item = &PathBuf> {
        self.files.iter()
    }
}
