//! Workspace management for multi-project setups
//! Reserved for workspace features

#![allow(dead_code)]

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_workspace_new() {
        let root = PathBuf::from("/test/workspace");
        let workspace = Workspace::new(root.clone());

        assert_eq!(workspace.root, root);
    }

    #[test]
    fn test_add_project() {
        let mut workspace = Workspace::new(PathBuf::from("/test"));
        let config_path = PathBuf::from("/test/project/tsconfig.json");
        let project = Project::new(PathBuf::from("/test/project"));

        workspace.add_project(config_path.clone(), project);

        assert!(workspace.get_project(&config_path).is_some());
    }

    #[test]
    fn test_remove_project() {
        let mut workspace = Workspace::new(PathBuf::from("/test"));
        let config_path = PathBuf::from("/test/project/tsconfig.json");
        let project = Project::new(PathBuf::from("/test/project"));

        workspace.add_project(config_path.clone(), project);
        workspace.remove_project(&config_path);

        assert!(workspace.get_project(&config_path).is_none());
    }

    #[test]
    fn test_get_projects() {
        let mut workspace = Workspace::new(PathBuf::from("/test"));

        workspace.add_project(
            PathBuf::from("/test/a/tsconfig.json"),
            Project::new(PathBuf::from("/test/a")),
        );
        workspace.add_project(
            PathBuf::from("/test/b/tsconfig.json"),
            Project::new(PathBuf::from("/test/b")),
        );

        let projects: Vec<_> = workspace.get_projects().collect();
        assert_eq!(projects.len(), 2);
    }

    #[test]
    fn test_project_for_file() {
        let mut workspace = Workspace::new(PathBuf::from("/test"));

        workspace.add_project(
            PathBuf::from("/test/project/tsconfig.json"),
            Project::new(PathBuf::from("/test/project")),
        );

        let file = PathBuf::from("/test/project/src/main.ts");
        let project = workspace.project_for_file(&file);

        assert!(project.is_some());
        assert_eq!(project.unwrap().root, PathBuf::from("/test/project"));
    }

    #[test]
    fn test_project_for_file_nested() {
        let mut workspace = Workspace::new(PathBuf::from("/test"));

        // Add two projects, one nested inside the other
        workspace.add_project(
            PathBuf::from("/test/project/tsconfig.json"),
            Project::new(PathBuf::from("/test/project")),
        );
        workspace.add_project(
            PathBuf::from("/test/project/packages/lib/tsconfig.json"),
            Project::new(PathBuf::from("/test/project/packages/lib")),
        );

        // File in nested project should match the nested project
        let file = PathBuf::from("/test/project/packages/lib/src/index.ts");
        let project = workspace.project_for_file(&file);

        assert!(project.is_some());
        assert_eq!(
            project.unwrap().root,
            PathBuf::from("/test/project/packages/lib")
        );
    }

    #[test]
    fn test_project_for_file_not_found() {
        let workspace = Workspace::new(PathBuf::from("/test"));

        let file = PathBuf::from("/other/project/main.ts");
        let project = workspace.project_for_file(&file);

        assert!(project.is_none());
    }

    #[test]
    fn test_project_for_file_mut() {
        let mut workspace = Workspace::new(PathBuf::from("/test"));

        workspace.add_project(
            PathBuf::from("/test/project/tsconfig.json"),
            Project::new(PathBuf::from("/test/project")),
        );

        let file = PathBuf::from("/test/project/src/main.ts");
        let project = workspace.project_for_file_mut(&file);

        assert!(project.is_some());
        // Add a file to verify mutability
        project.unwrap().add_file(file.clone());
    }

    #[test]
    fn test_discover_projects_creates_default() {
        let temp_dir = TempDir::new().unwrap();

        let mut workspace = Workspace::new(temp_dir.path().to_path_buf());
        let result = workspace.discover_projects();

        assert!(result.is_ok());
        // Should create a default project when no tsconfig found
        assert!(workspace.get_projects().next().is_some());
    }

    #[test]
    fn test_discover_projects_finds_tsconfig() {
        let temp_dir = TempDir::new().unwrap();

        // Create a tsconfig.json
        fs::write(
            temp_dir.path().join("tsconfig.json"),
            r#"{"compilerOptions": {}}"#,
        )
        .unwrap();

        let mut workspace = Workspace::new(temp_dir.path().to_path_buf());
        let result = workspace.discover_projects();

        assert!(result.is_ok());
    }

    #[test]
    fn test_discover_projects_nested() {
        let temp_dir = TempDir::new().unwrap();

        // Create nested tsconfig files
        let pkg_dir = temp_dir.path().join("packages").join("lib");
        fs::create_dir_all(&pkg_dir).unwrap();

        fs::write(
            temp_dir.path().join("tsconfig.json"),
            r#"{"compilerOptions": {}}"#,
        )
        .unwrap();

        fs::write(pkg_dir.join("tsconfig.json"), r#"{"compilerOptions": {}}"#).unwrap();

        let mut workspace = Workspace::new(temp_dir.path().to_path_buf());
        let result = workspace.discover_projects();

        assert!(result.is_ok());
        // Should find both tsconfig files
        let project_count = workspace.get_projects().count();
        assert!(project_count >= 1);
    }

    #[test]
    fn test_find_tsconfig_files_empty() {
        let temp_dir = TempDir::new().unwrap();

        let configs = find_tsconfig_files(temp_dir.path());
        assert!(configs.is_empty());
    }

    #[test]
    fn test_find_tsconfig_files_single() {
        let temp_dir = TempDir::new().unwrap();

        fs::write(
            temp_dir.path().join("tsconfig.json"),
            r#"{"compilerOptions": {}}"#,
        )
        .unwrap();

        let configs = find_tsconfig_files(temp_dir.path());
        assert_eq!(configs.len(), 1);
    }

    #[test]
    fn test_find_tsconfig_files_custom_name() {
        let temp_dir = TempDir::new().unwrap();

        fs::write(
            temp_dir.path().join("custom.tsconfig.json"),
            r#"{"compilerOptions": {}}"#,
        )
        .unwrap();

        let configs = find_tsconfig_files(temp_dir.path());
        assert_eq!(configs.len(), 1);
    }

    #[test]
    fn test_find_tsconfig_skips_node_modules() {
        let temp_dir = TempDir::new().unwrap();
        let node_modules = temp_dir.path().join("node_modules");
        fs::create_dir(&node_modules).unwrap();

        fs::write(
            node_modules.join("tsconfig.json"),
            r#"{"compilerOptions": {}}"#,
        )
        .unwrap();

        let configs = find_tsconfig_files(temp_dir.path());
        assert!(configs.is_empty());
    }

    #[test]
    fn test_find_tsconfig_skips_hidden() {
        let temp_dir = TempDir::new().unwrap();
        let hidden = temp_dir.path().join(".hidden");
        fs::create_dir(&hidden).unwrap();

        fs::write(hidden.join("tsconfig.json"), r#"{"compilerOptions": {}}"#).unwrap();

        let configs = find_tsconfig_files(temp_dir.path());
        assert!(configs.is_empty());
    }

    #[test]
    fn test_get_project_nonexistent() {
        let workspace = Workspace::new(PathBuf::from("/test"));
        let config_path = PathBuf::from("/nonexistent/tsconfig.json");

        assert!(workspace.get_project(&config_path).is_none());
    }
}
