//! Module resolution implementation
//! Reserved for cross-file navigation features

#![allow(dead_code)]

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_resolver_new() {
        let resolver = ModuleResolver::new(PathBuf::from("/test"));
        assert_eq!(resolver.base_dir, PathBuf::from("/test"));
        assert_eq!(resolver.mode, ModuleResolution::Node);
        assert!(resolver.path_mappings.is_empty());
        assert!(resolver.base_url.is_none());
    }

    #[test]
    fn test_module_resolver_default() {
        let resolver = ModuleResolver::default();
        assert_eq!(resolver.base_dir, PathBuf::from("."));
        assert_eq!(resolver.mode, ModuleResolution::Node);
    }

    #[test]
    fn test_module_resolution_default() {
        let mode = ModuleResolution::default();
        assert_eq!(mode, ModuleResolution::Node);
    }

    #[test]
    fn test_module_resolution_variants() {
        assert_eq!(ModuleResolution::Node, ModuleResolution::Node);
        assert_eq!(ModuleResolution::NodeNext, ModuleResolution::NodeNext);
        assert_eq!(ModuleResolution::Bundler, ModuleResolution::Bundler);
        assert_eq!(ModuleResolution::Classic, ModuleResolution::Classic);
    }

    #[test]
    fn test_with_tsconfig_node_resolution() {
        let tsconfig = TsConfig {
            compiler_options: Some(crate::resolution::tsconfig::CompilerOptions {
                module_resolution: Some("node".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };

        let resolver = ModuleResolver::with_tsconfig(PathBuf::from("/test"), &tsconfig);
        assert_eq!(resolver.mode, ModuleResolution::Node);
    }

    #[test]
    fn test_with_tsconfig_node16_resolution() {
        let tsconfig = TsConfig {
            compiler_options: Some(crate::resolution::tsconfig::CompilerOptions {
                module_resolution: Some("node16".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };

        let resolver = ModuleResolver::with_tsconfig(PathBuf::from("/test"), &tsconfig);
        assert_eq!(resolver.mode, ModuleResolution::NodeNext);
    }

    #[test]
    fn test_with_tsconfig_bundler_resolution() {
        let tsconfig = TsConfig {
            compiler_options: Some(crate::resolution::tsconfig::CompilerOptions {
                module_resolution: Some("bundler".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };

        let resolver = ModuleResolver::with_tsconfig(PathBuf::from("/test"), &tsconfig);
        assert_eq!(resolver.mode, ModuleResolution::Bundler);
    }

    #[test]
    fn test_with_tsconfig_classic_resolution() {
        let tsconfig = TsConfig {
            compiler_options: Some(crate::resolution::tsconfig::CompilerOptions {
                module_resolution: Some("classic".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };

        let resolver = ModuleResolver::with_tsconfig(PathBuf::from("/test"), &tsconfig);
        assert_eq!(resolver.mode, ModuleResolution::Classic);
    }

    #[test]
    fn test_with_tsconfig_base_url() {
        let tsconfig = TsConfig {
            compiler_options: Some(crate::resolution::tsconfig::CompilerOptions {
                base_url: Some("src".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };

        let resolver = ModuleResolver::with_tsconfig(PathBuf::from("/test"), &tsconfig);
        assert_eq!(resolver.base_url, Some(PathBuf::from("/test/src")));
    }

    #[test]
    fn test_with_tsconfig_path_mappings() {
        let mut paths = std::collections::HashMap::new();
        paths.insert("@/*".to_string(), vec!["src/*".to_string()]);

        let tsconfig = TsConfig {
            compiler_options: Some(crate::resolution::tsconfig::CompilerOptions {
                paths: Some(paths),
                ..Default::default()
            }),
            ..Default::default()
        };

        let resolver = ModuleResolver::with_tsconfig(PathBuf::from("/test"), &tsconfig);
        assert_eq!(resolver.path_mappings.len(), 1);
    }

    #[test]
    fn test_match_path_pattern_exact() {
        let result = match_path_pattern("lodash", "lodash");
        assert_eq!(result, Some(String::new()));
    }

    #[test]
    fn test_match_path_pattern_no_match() {
        let result = match_path_pattern("lodash", "underscore");
        assert_eq!(result, None);
    }

    #[test]
    fn test_match_path_pattern_wildcard() {
        let result = match_path_pattern("@/*", "@/components/Button");
        assert_eq!(result, Some("components/Button".to_string()));
    }

    #[test]
    fn test_match_path_pattern_prefix_suffix() {
        let result = match_path_pattern("@app/*-module", "@app/auth-module");
        assert_eq!(result, Some("auth".to_string()));
    }

    #[test]
    fn test_resolved_module_fields() {
        let module = ResolvedModule {
            path: PathBuf::from("/test/utils.ts"),
            is_external: false,
            specifier: "./utils".to_string(),
        };

        assert_eq!(module.path, PathBuf::from("/test/utils.ts"));
        assert!(!module.is_external);
        assert_eq!(module.specifier, "./utils");
    }

    #[test]
    fn test_resolved_module_external() {
        let module = ResolvedModule {
            path: PathBuf::from("/node_modules/lodash/index.js"),
            is_external: true,
            specifier: "lodash".to_string(),
        };

        assert!(module.is_external);
    }

    #[test]
    fn test_resolve_relative_not_found() {
        let resolver = ModuleResolver::new(PathBuf::from("/test"));
        let from_file = PathBuf::from("/test/src/main.ts");

        let result = resolver.resolve("./nonexistent", &from_file);
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_node_module_not_found() {
        let resolver = ModuleResolver::new(PathBuf::from("/test"));
        let from_file = PathBuf::from("/test/src/main.ts");

        let result = resolver.resolve("nonexistent-package", &from_file);
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_with_base_url_not_found() {
        let mut resolver = ModuleResolver::new(PathBuf::from("/test"));
        resolver.base_url = Some(PathBuf::from("/test/src"));

        let from_file = PathBuf::from("/test/src/main.ts");
        let result = resolver.resolve("nonexistent", &from_file);
        assert!(result.is_none());
    }

    #[test]
    fn test_try_resolve_file_nonexistent() {
        let resolver = ModuleResolver::new(PathBuf::from("/test"));
        let result = resolver.try_resolve_file(&PathBuf::from("/nonexistent/file.ts"));
        assert!(result.is_none());
    }

    #[test]
    fn test_module_resolution_clone() {
        let mode = ModuleResolution::Node;
        let cloned = mode;
        assert_eq!(mode, cloned);
    }

    #[test]
    fn test_resolved_module_clone() {
        let module = ResolvedModule {
            path: PathBuf::from("/test/utils.ts"),
            is_external: false,
            specifier: "./utils".to_string(),
        };

        let cloned = module.clone();
        assert_eq!(module.path, cloned.path);
        assert_eq!(module.is_external, cloned.is_external);
        assert_eq!(module.specifier, cloned.specifier);
    }
}
