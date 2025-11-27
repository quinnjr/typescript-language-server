use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::Project;

/// Manages multiple projects in a workspace
pub struct Workspace {
    /// Root directory of the workspace
    pub root: PathBuf,
    /// All projects in the workspace
    projects: HashMap<PathBuf, Project>,
}

impl Workspace {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            projects: HashMap::new(),
        }
    }

    /// Discover all projects (tsconfig.json files) in the workspace
    pub fn discover_projects(&mut self) -> Result<(), String> {
        let tsconfigs = find_tsconfig_files(&self.root);

        for config_path in tsconfigs {
            match Project::from_tsconfig(config_path.clone()) {
                Ok(project) => {
                    self.projects.insert(config_path, project);
                }
                Err(e) => {
                    // Log error but continue
                    eprintln!("Failed to load project from {:?}: {}", config_path, e);
                }
            }
        }

        // If no projects found, create a default one from workspace root
        if self.projects.is_empty() {
            self.projects
                .insert(self.root.clone(), Project::new(self.root.clone()));
        }

        Ok(())
    }

    /// Get the project that contains a given file
    pub fn project_for_file(&self, path: &Path) -> Option<&Project> {
        // Find the project whose root is closest to the file
        let mut best_project: Option<&Project> = None;
        let mut best_depth = 0;

        for project in self.projects.values() {
            if path.starts_with(&project.root) {
                let depth = project.root.components().count();
                if depth > best_depth {
                    best_depth = depth;
                    best_project = Some(project);
                }
            }
        }

        best_project
    }

    /// Get a mutable reference to the project that contains a given file
    pub fn project_for_file_mut(&mut self, path: &Path) -> Option<&mut Project> {
        // Find the project whose root is closest to the file
        let mut best_key: Option<PathBuf> = None;
        let mut best_depth = 0;

        for (key, project) in &self.projects {
            if path.starts_with(&project.root) {
                let depth = project.root.components().count();
                if depth > best_depth {
                    best_depth = depth;
                    best_key = Some(key.clone());
                }
            }
        }

        best_key.and_then(|key| self.projects.get_mut(&key))
    }

    /// Add a project to the workspace
    pub fn add_project(&mut self, config_path: PathBuf, project: Project) {
        self.projects.insert(config_path, project);
    }

    /// Remove a project from the workspace
    pub fn remove_project(&mut self, config_path: &Path) {
        self.projects.remove(config_path);
    }

    /// Get all projects in the workspace
    pub fn get_projects(&self) -> impl Iterator<Item = &Project> {
        self.projects.values()
    }

    /// Get a project by its config path
    pub fn get_project(&self, config_path: &Path) -> Option<&Project> {
        self.projects.get(config_path)
    }
}

/// Find all tsconfig.json files in a directory (recursively)
fn find_tsconfig_files(root: &Path) -> Vec<PathBuf> {
    let mut configs = Vec::new();

    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();

            if path.is_file() {
                if let Some(name) = path.file_name() {
                    let name = name.to_string_lossy();
                    if name == "tsconfig.json" || name.ends_with(".tsconfig.json") {
                        configs.push(path);
                    }
                }
            } else if path.is_dir() {
                // Skip node_modules and hidden directories
                if let Some(name) = path.file_name() {
                    let name = name.to_string_lossy();
                    if !name.starts_with('.') && name != "node_modules" {
                        configs.extend(find_tsconfig_files(&path));
                    }
                }
            }
        }
    }

    configs
}

