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

    /// Get the number of files in the project
    pub fn file_count(&self) -> usize {
        self.files.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_project_new() {
        let root = PathBuf::from("/test/project");
        let project = Project::new(root.clone());

        assert_eq!(project.root, root);
        assert!(project.config_path.is_none());
        assert!(project.config.is_none());
        assert!(project.files.is_empty());
    }

    #[test]
    fn test_add_file() {
        let mut project = Project::new(PathBuf::from("/test"));
        let file = PathBuf::from("/test/main.ts");

        project.add_file(file.clone());

        assert!(project.contains_file(&file));
        assert_eq!(project.file_count(), 1);
    }

    #[test]
    fn test_remove_file() {
        let mut project = Project::new(PathBuf::from("/test"));
        let file = PathBuf::from("/test/main.ts");

        project.add_file(file.clone());
        project.remove_file(&file);

        assert!(!project.contains_file(&file));
        assert_eq!(project.file_count(), 0);
    }

    #[test]
    fn test_contains_file() {
        let mut project = Project::new(PathBuf::from("/test"));
        let file1 = PathBuf::from("/test/main.ts");
        let file2 = PathBuf::from("/test/other.ts");

        project.add_file(file1.clone());

        assert!(project.contains_file(&file1));
        assert!(!project.contains_file(&file2));
    }

    #[test]
    fn test_get_files() {
        let mut project = Project::new(PathBuf::from("/test"));
        project.add_file(PathBuf::from("/test/a.ts"));
        project.add_file(PathBuf::from("/test/b.ts"));
        project.add_file(PathBuf::from("/test/c.ts"));

        let files: Vec<_> = project.get_files().collect();
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn test_from_tsconfig_not_found() {
        let result = Project::from_tsconfig(PathBuf::from("/nonexistent/tsconfig.json"));
        assert!(result.is_err());
    }

    #[test]
    fn test_from_tsconfig_valid() {
        let temp_dir = TempDir::new().unwrap();
        let tsconfig_path = temp_dir.path().join("tsconfig.json");

        fs::write(
            &tsconfig_path,
            r#"{
                "compilerOptions": {
                    "target": "ES2020",
                    "module": "ESNext"
                }
            }"#,
        )
        .unwrap();

        let result = Project::from_tsconfig(tsconfig_path);
        assert!(result.is_ok());

        let project = result.unwrap();
        assert!(project.config.is_some());
    }

    #[test]
    fn test_from_tsconfig_with_include() {
        let temp_dir = TempDir::new().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();

        // Create test files
        fs::write(src_dir.join("main.ts"), "const x = 1;").unwrap();
        fs::write(src_dir.join("utils.ts"), "export const y = 2;").unwrap();

        let tsconfig_path = temp_dir.path().join("tsconfig.json");
        fs::write(
            &tsconfig_path,
            r#"{
                "compilerOptions": {
                    "target": "ES2020"
                },
                "include": ["src/**/*"]
            }"#,
        )
        .unwrap();

        let result = Project::from_tsconfig(tsconfig_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_discover_default_files() {
        let temp_dir = TempDir::new().unwrap();

        // Create test files
        fs::write(temp_dir.path().join("main.ts"), "const x = 1;").unwrap();
        fs::write(temp_dir.path().join("utils.tsx"), "const y = 2;").unwrap();
        fs::write(temp_dir.path().join("readme.md"), "# README").unwrap();

        let tsconfig_path = temp_dir.path().join("tsconfig.json");
        fs::write(
            &tsconfig_path,
            r#"{
                "compilerOptions": {
                    "target": "ES2020"
                },
                "include": []
            }"#,
        )
        .unwrap();

        let result = Project::from_tsconfig(tsconfig_path);
        assert!(result.is_ok());

        let project = result.unwrap();
        // Should discover .ts and .tsx files but not .md
        let ts_files: Vec<_> = project
            .get_files()
            .filter(|p| {
                p.extension()
                    .map_or(false, |ext| ext == "ts" || ext == "tsx")
            })
            .collect();

        assert_eq!(ts_files.len(), 2);
    }

    #[test]
    fn test_discover_skips_node_modules() {
        let temp_dir = TempDir::new().unwrap();
        let node_modules = temp_dir.path().join("node_modules");
        fs::create_dir(&node_modules).unwrap();

        fs::write(temp_dir.path().join("main.ts"), "const x = 1;").unwrap();
        fs::write(node_modules.join("lodash.ts"), "export {};").unwrap();

        let tsconfig_path = temp_dir.path().join("tsconfig.json");
        fs::write(
            &tsconfig_path,
            r#"{
                "compilerOptions": {}
            }"#,
        )
        .unwrap();

        let result = Project::from_tsconfig(tsconfig_path);
        assert!(result.is_ok());

        let project = result.unwrap();
        // Should not include files from node_modules
        for file in project.get_files() {
            assert!(!file.to_string_lossy().contains("node_modules"));
        }
    }

    #[test]
    fn test_discover_skips_hidden_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let hidden_dir = temp_dir.path().join(".hidden");
        fs::create_dir(&hidden_dir).unwrap();

        fs::write(temp_dir.path().join("main.ts"), "const x = 1;").unwrap();
        fs::write(hidden_dir.join("secret.ts"), "export {};").unwrap();

        let tsconfig_path = temp_dir.path().join("tsconfig.json");
        fs::write(
            &tsconfig_path,
            r#"{
                "compilerOptions": {}
            }"#,
        )
        .unwrap();

        let result = Project::from_tsconfig(tsconfig_path);
        assert!(result.is_ok());

        let project = result.unwrap();
        for file in project.get_files() {
            assert!(!file.to_string_lossy().contains(".hidden"));
        }
    }

    #[test]
    fn test_file_graph_integration() {
        let mut project = Project::new(PathBuf::from("/test"));
        let main = PathBuf::from("/test/main.ts");
        let utils = PathBuf::from("/test/utils.ts");

        project.add_file(main.clone());
        project.add_file(utils.clone());
        project.file_graph.add_import(&main, &utils);

        // Removing a file should also update the graph
        project.remove_file(&main);
        assert!(!project.contains_file(&main));
    }

    #[test]
    fn test_wildcard_pattern() {
        let temp_dir = TempDir::new().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();

        fs::write(src_dir.join("main.ts"), "const x = 1;").unwrap();

        let tsconfig_path = temp_dir.path().join("tsconfig.json");
        fs::write(
            &tsconfig_path,
            r#"{
                "compilerOptions": {},
                "include": ["src/*"]
            }"#,
        )
        .unwrap();

        let result = Project::from_tsconfig(tsconfig_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_specific_file_pattern() {
        let temp_dir = TempDir::new().unwrap();
        let main_file = temp_dir.path().join("main.ts");
        fs::write(&main_file, "const x = 1;").unwrap();

        let tsconfig_path = temp_dir.path().join("tsconfig.json");
        fs::write(
            &tsconfig_path,
            r#"{
                "compilerOptions": {},
                "include": ["main.ts"]
            }"#,
        )
        .unwrap();

        let result = Project::from_tsconfig(tsconfig_path);
        assert!(result.is_ok());

        let project = result.unwrap();
        assert!(project.contains_file(&main_file));
    }
}
