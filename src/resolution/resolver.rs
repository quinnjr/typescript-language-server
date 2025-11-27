use std::path::{Path, PathBuf};

use super::node_modules::resolve_node_module;
use super::tsconfig::TsConfig;

/// The result of module resolution
#[derive(Debug, Clone)]
pub struct ResolvedModule {
    /// The resolved file path
    pub path: PathBuf,
    /// Whether this is an external package (from node_modules)
    pub is_external: bool,
    /// The original module specifier
    pub specifier: String,
}

/// Module resolution mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleResolution {
    /// Node.js CommonJS resolution
    Node,
    /// Node.js ESM resolution (Node16/NodeNext)
    NodeNext,
    /// Bundler resolution (similar to webpack/vite)
    Bundler,
    /// Classic TypeScript resolution (deprecated)
    Classic,
}

impl Default for ModuleResolution {
    fn default() -> Self {
        Self::Node
    }
}

/// Module resolver
pub struct ModuleResolver {
    /// Resolution mode
    pub mode: ModuleResolution,
    /// Base directory for resolution
    pub base_dir: PathBuf,
    /// Path mappings from tsconfig
    pub path_mappings: Vec<(String, Vec<String>)>,
    /// Base URL from tsconfig
    pub base_url: Option<PathBuf>,
}

impl ModuleResolver {
    pub fn new(base_dir: PathBuf) -> Self {
        Self {
            mode: ModuleResolution::default(),
            base_dir,
            path_mappings: Vec::new(),
            base_url: None,
        }
    }

    /// Create a resolver with tsconfig settings
    pub fn with_tsconfig(base_dir: PathBuf, tsconfig: &TsConfig) -> Self {
        let mut resolver = Self::new(base_dir.clone());

        if let Some(ref compiler_options) = tsconfig.compiler_options {
            // Set module resolution mode
            if let Some(ref module_resolution) = compiler_options.module_resolution {
                resolver.mode = match module_resolution.to_lowercase().as_str() {
                    "node" | "node10" => ModuleResolution::Node,
                    "node16" | "nodenext" => ModuleResolution::NodeNext,
                    "bundler" => ModuleResolution::Bundler,
                    "classic" => ModuleResolution::Classic,
                    _ => ModuleResolution::Node,
                };
            }

            // Set base URL
            if let Some(ref base_url) = compiler_options.base_url {
                resolver.base_url = Some(base_dir.join(base_url));
            }

            // Set path mappings
            if let Some(ref paths) = compiler_options.paths {
                resolver.path_mappings =
                    paths.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            }
        }

        resolver
    }

    /// Resolve a module specifier from a source file
    pub fn resolve(&self, specifier: &str, from_file: &Path) -> Option<ResolvedModule> {
        let from_dir = from_file.parent().unwrap_or(Path::new("."));

        // Try path mappings first
        if let Some(resolved) = self.resolve_with_path_mappings(specifier) {
            return Some(resolved);
        }

        // Check if it's a relative import
        if specifier.starts_with("./") || specifier.starts_with("../") {
            return self.resolve_relative(specifier, from_dir);
        }

        // Check if it's an absolute import (with baseUrl)
        if let Some(ref base_url) = self.base_url {
            if let Some(resolved) = self.resolve_from_base_url(specifier, base_url) {
                return Some(resolved);
            }
        }

        // Try to resolve from node_modules
        self.resolve_node_module(specifier, from_dir)
    }

    /// Resolve using path mappings
    fn resolve_with_path_mappings(&self, specifier: &str) -> Option<ResolvedModule> {
        for (pattern, targets) in &self.path_mappings {
            if let Some(matched) = match_path_pattern(pattern, specifier) {
                for target in targets {
                    let resolved_target = target.replace('*', &matched);
                    let base = self.base_url.as_ref().unwrap_or(&self.base_dir);
                    let full_path = base.join(&resolved_target);

                    if let Some(resolved) = self.try_resolve_file(&full_path) {
                        return Some(ResolvedModule {
                            path: resolved,
                            is_external: false,
                            specifier: specifier.to_string(),
                        });
                    }
                }
            }
        }
        None
    }

    /// Resolve a relative import
    fn resolve_relative(&self, specifier: &str, from_dir: &Path) -> Option<ResolvedModule> {
        let target_path = from_dir.join(specifier);

        self.try_resolve_file(&target_path)
            .map(|path| ResolvedModule {
                path,
                is_external: false,
                specifier: specifier.to_string(),
            })
    }

    /// Resolve from base URL
    fn resolve_from_base_url(&self, specifier: &str, base_url: &Path) -> Option<ResolvedModule> {
        let target_path = base_url.join(specifier);

        self.try_resolve_file(&target_path)
            .map(|path| ResolvedModule {
                path,
                is_external: false,
                specifier: specifier.to_string(),
            })
    }

    /// Resolve from node_modules
    fn resolve_node_module(&self, specifier: &str, from_dir: &Path) -> Option<ResolvedModule> {
        resolve_node_module(specifier, from_dir).map(|path| ResolvedModule {
            path,
            is_external: true,
            specifier: specifier.to_string(),
        })
    }

    /// Try to resolve a file path, handling extensions
    fn try_resolve_file(&self, path: &Path) -> Option<PathBuf> {
        // If the path already has an extension and exists, use it
        if path.exists() && path.is_file() {
            return Some(path.to_path_buf());
        }

        // Try adding extensions
        let extensions = [
            ".ts", ".tsx", ".d.ts", ".js", ".jsx", ".mts", ".mjs", ".cts", ".cjs",
        ];

        for ext in extensions {
            let with_ext = path.with_extension(ext.trim_start_matches('.'));
            if with_ext.exists() && with_ext.is_file() {
                return Some(with_ext);
            }
        }

        // Try as directory with index file
        if path.is_dir() {
            for ext in extensions {
                let index = path.join(format!("index{}", ext));
                if index.exists() && index.is_file() {
                    return Some(index);
                }
            }
        }

        // Try with /index added
        let as_dir = path.to_path_buf();
        for ext in extensions {
            let index = as_dir.join(format!("index{}", ext));
            if index.exists() && index.is_file() {
                return Some(index);
            }
        }

        None
    }
}

/// Match a path pattern with a specifier
/// Patterns can contain a single `*` wildcard
fn match_path_pattern(pattern: &str, specifier: &str) -> Option<String> {
    if pattern.contains('*') {
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            let prefix = parts[0];
            let suffix = parts[1];

            if specifier.starts_with(prefix) && specifier.ends_with(suffix) {
                let matched_len = specifier.len() - prefix.len() - suffix.len();
                let matched = &specifier[prefix.len()..prefix.len() + matched_len];
                return Some(matched.to_string());
            }
        }
    } else if pattern == specifier {
        return Some(String::new());
    }
    None
}

impl Default for ModuleResolver {
    fn default() -> Self {
        Self::new(PathBuf::from("."))
    }
}
