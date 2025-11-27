use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Represents a tsconfig.json file
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // Fields used for deserialization and future features
pub struct TsConfig {
    pub compiler_options: Option<CompilerOptions>,
    pub include: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
    pub files: Option<Vec<String>>,
    #[serde(rename = "extends")]
    pub extends_path: Option<String>,
    pub references: Option<Vec<ProjectReference>>,
}

/// Compiler options from tsconfig.json
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CompilerOptions {
    pub target: Option<String>,
    pub module: Option<String>,
    pub module_resolution: Option<String>,
    pub base_url: Option<String>,
    pub paths: Option<HashMap<String, Vec<String>>>,
    pub root_dir: Option<String>,
    pub root_dirs: Option<Vec<String>>,
    pub out_dir: Option<String>,
    pub declaration: Option<bool>,
    pub declaration_dir: Option<String>,
    pub strict: Option<bool>,
    pub no_implicit_any: Option<bool>,
    pub strict_null_checks: Option<bool>,
    pub strict_function_types: Option<bool>,
    pub strict_bind_call_apply: Option<bool>,
    pub strict_property_initialization: Option<bool>,
    pub no_implicit_this: Option<bool>,
    pub always_strict: Option<bool>,
    pub no_unused_locals: Option<bool>,
    pub no_unused_parameters: Option<bool>,
    pub no_implicit_returns: Option<bool>,
    pub no_fallthrough_cases_in_switch: Option<bool>,
    pub es_module_interop: Option<bool>,
    pub allow_synthetic_default_imports: Option<bool>,
    pub skip_lib_check: Option<bool>,
    pub force_consistent_casing_in_file_names: Option<bool>,
    pub resolve_json_module: Option<bool>,
    pub isolated_modules: Option<bool>,
    pub jsx: Option<String>,
    pub jsx_factory: Option<String>,
    pub jsx_fragment_factory: Option<String>,
    pub jsx_import_source: Option<String>,
    pub lib: Option<Vec<String>>,
    pub types: Option<Vec<String>>,
    pub type_roots: Option<Vec<String>>,
}

/// Project reference
#[derive(Debug, Clone, Deserialize)]
pub struct ProjectReference {
    pub path: String,
    pub prepend: Option<bool>,
}

impl TsConfig {
    /// Load a tsconfig.json file
    #[allow(dead_code)] // Reserved for project system integration
    pub fn load(path: &Path) -> Result<Self, TsConfigError> {
        let content =
            std::fs::read_to_string(path).map_err(|e| TsConfigError::IoError(e.to_string()))?;

        // Remove comments (simple approach - doesn't handle all cases)
        let content = remove_json_comments(&content);

        let config: TsConfig =
            serde_json::from_str(&content).map_err(|e| TsConfigError::ParseError(e.to_string()))?;

        // Handle extends
        if let Some(ref extends) = config.extends_path {
            let base_dir = path.parent().unwrap_or(Path::new("."));
            let extends_path = base_dir.join(extends);

            // Add .json if not present
            let extends_path = if extends_path.extension().is_none() {
                extends_path.with_extension("json")
            } else {
                extends_path
            };

            if extends_path.exists() {
                let base_config = Self::load(&extends_path)?;
                return Ok(config.merge_with_base(base_config));
            }
        }

        Ok(config)
    }

    /// Merge this config with a base config (for extends)
    fn merge_with_base(self, base: TsConfig) -> Self {
        TsConfig {
            compiler_options: match (self.compiler_options, base.compiler_options) {
                (Some(child), Some(parent)) => Some(child.merge_with_base(parent)),
                (Some(child), None) => Some(child),
                (None, Some(parent)) => Some(parent),
                (None, None) => None,
            },
            include: self.include.or(base.include),
            exclude: self.exclude.or(base.exclude),
            files: self.files.or(base.files),
            extends_path: None, // Already resolved
            references: self.references.or(base.references),
        }
    }
}

impl CompilerOptions {
    fn merge_with_base(self, base: CompilerOptions) -> Self {
        CompilerOptions {
            target: self.target.or(base.target),
            module: self.module.or(base.module),
            module_resolution: self.module_resolution.or(base.module_resolution),
            base_url: self.base_url.or(base.base_url),
            paths: match (self.paths, base.paths) {
                (Some(child), Some(parent)) => {
                    let mut merged = parent;
                    merged.extend(child);
                    Some(merged)
                }
                (Some(child), None) => Some(child),
                (None, Some(parent)) => Some(parent),
                (None, None) => None,
            },
            root_dir: self.root_dir.or(base.root_dir),
            root_dirs: self.root_dirs.or(base.root_dirs),
            out_dir: self.out_dir.or(base.out_dir),
            declaration: self.declaration.or(base.declaration),
            declaration_dir: self.declaration_dir.or(base.declaration_dir),
            strict: self.strict.or(base.strict),
            no_implicit_any: self.no_implicit_any.or(base.no_implicit_any),
            strict_null_checks: self.strict_null_checks.or(base.strict_null_checks),
            strict_function_types: self.strict_function_types.or(base.strict_function_types),
            strict_bind_call_apply: self.strict_bind_call_apply.or(base.strict_bind_call_apply),
            strict_property_initialization: self
                .strict_property_initialization
                .or(base.strict_property_initialization),
            no_implicit_this: self.no_implicit_this.or(base.no_implicit_this),
            always_strict: self.always_strict.or(base.always_strict),
            no_unused_locals: self.no_unused_locals.or(base.no_unused_locals),
            no_unused_parameters: self.no_unused_parameters.or(base.no_unused_parameters),
            no_implicit_returns: self.no_implicit_returns.or(base.no_implicit_returns),
            no_fallthrough_cases_in_switch: self
                .no_fallthrough_cases_in_switch
                .or(base.no_fallthrough_cases_in_switch),
            es_module_interop: self.es_module_interop.or(base.es_module_interop),
            allow_synthetic_default_imports: self
                .allow_synthetic_default_imports
                .or(base.allow_synthetic_default_imports),
            skip_lib_check: self.skip_lib_check.or(base.skip_lib_check),
            force_consistent_casing_in_file_names: self
                .force_consistent_casing_in_file_names
                .or(base.force_consistent_casing_in_file_names),
            resolve_json_module: self.resolve_json_module.or(base.resolve_json_module),
            isolated_modules: self.isolated_modules.or(base.isolated_modules),
            jsx: self.jsx.or(base.jsx),
            jsx_factory: self.jsx_factory.or(base.jsx_factory),
            jsx_fragment_factory: self.jsx_fragment_factory.or(base.jsx_fragment_factory),
            jsx_import_source: self.jsx_import_source.or(base.jsx_import_source),
            lib: self.lib.or(base.lib),
            types: self.types.or(base.types),
            type_roots: self.type_roots.or(base.type_roots),
        }
    }
}

/// Errors that can occur when loading a tsconfig
#[derive(Debug)]
pub enum TsConfigError {
    IoError(String),
    ParseError(String),
}

impl std::fmt::Display for TsConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TsConfigError::IoError(e) => write!(f, "IO error: {}", e),
            TsConfigError::ParseError(e) => write!(f, "Parse error: {}", e),
        }
    }
}

impl std::error::Error for TsConfigError {}

/// Remove C-style comments from JSON
fn remove_json_comments(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut in_string = false;

    while let Some(c) = chars.next() {
        if in_string {
            result.push(c);
            if c == '"' {
                in_string = false;
            } else if c == '\\' {
                // Skip escaped character
                if let Some(next) = chars.next() {
                    result.push(next);
                }
            }
        } else {
            match c {
                '"' => {
                    in_string = true;
                    result.push(c);
                }
                '/' => {
                    if chars.peek() == Some(&'/') {
                        // Line comment - skip until end of line
                        chars.next();
                        for c in chars.by_ref() {
                            if c == '\n' {
                                result.push('\n');
                                break;
                            }
                        }
                    } else if chars.peek() == Some(&'*') {
                        // Block comment - skip until */
                        chars.next();
                        while let Some(c) = chars.next() {
                            if c == '*' && chars.peek() == Some(&'/') {
                                chars.next();
                                break;
                            }
                        }
                    } else {
                        result.push(c);
                    }
                }
                _ => result.push(c),
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_json_comments_line() {
        let input = r#"{
            // This is a comment
            "key": "value"
        }"#;
        let result = remove_json_comments(input);
        assert!(!result.contains("//"));
        assert!(result.contains("\"key\": \"value\""));
    }

    #[test]
    fn test_remove_json_comments_block() {
        let input = r#"{
            /* Block comment */
            "key": "value"
        }"#;
        let result = remove_json_comments(input);
        assert!(!result.contains("/*"));
        assert!(!result.contains("*/"));
        assert!(result.contains("\"key\": \"value\""));
    }

    #[test]
    fn test_remove_json_comments_in_string() {
        let input = r#"{"url": "http://example.com"}"#;
        let result = remove_json_comments(input);
        assert!(result.contains("http://example.com"));
    }

    #[test]
    fn test_remove_json_comments_no_comments() {
        let input = r#"{"key": "value"}"#;
        let result = remove_json_comments(input);
        assert_eq!(result, input);
    }

    #[test]
    fn test_tsconfig_default() {
        let config = TsConfig::default();
        assert!(config.compiler_options.is_none());
        assert!(config.include.is_none());
        assert!(config.exclude.is_none());
    }

    #[test]
    fn test_tsconfig_deserialize_simple() {
        let json = r#"{
            "compilerOptions": {
                "target": "ES2020",
                "module": "ESNext"
            }
        }"#;

        let config: TsConfig = serde_json::from_str(json).unwrap();
        let options = config.compiler_options.unwrap();
        assert_eq!(options.target, Some("ES2020".to_string()));
        assert_eq!(options.module, Some("ESNext".to_string()));
    }

    #[test]
    fn test_tsconfig_deserialize_with_include() {
        let json = r#"{
            "include": ["src/**/*"],
            "exclude": ["node_modules"]
        }"#;

        let config: TsConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.include, Some(vec!["src/**/*".to_string()]));
        assert_eq!(config.exclude, Some(vec!["node_modules".to_string()]));
    }

    #[test]
    fn test_tsconfig_deserialize_with_paths() {
        let json = r#"{
            "compilerOptions": {
                "baseUrl": ".",
                "paths": {
                    "@/*": ["src/*"]
                }
            }
        }"#;

        let config: TsConfig = serde_json::from_str(json).unwrap();
        let options = config.compiler_options.unwrap();
        assert_eq!(options.base_url, Some(".".to_string()));
        assert!(options.paths.is_some());
        let paths = options.paths.unwrap();
        assert!(paths.contains_key("@/*"));
    }

    #[test]
    fn test_tsconfig_deserialize_strict_options() {
        let json = r#"{
            "compilerOptions": {
                "strict": true,
                "noImplicitAny": true,
                "strictNullChecks": true
            }
        }"#;

        let config: TsConfig = serde_json::from_str(json).unwrap();
        let options = config.compiler_options.unwrap();
        assert_eq!(options.strict, Some(true));
        assert_eq!(options.no_implicit_any, Some(true));
        assert_eq!(options.strict_null_checks, Some(true));
    }

    #[test]
    fn test_tsconfig_deserialize_jsx() {
        let json = r#"{
            "compilerOptions": {
                "jsx": "react-jsx",
                "jsxImportSource": "react"
            }
        }"#;

        let config: TsConfig = serde_json::from_str(json).unwrap();
        let options = config.compiler_options.unwrap();
        assert_eq!(options.jsx, Some("react-jsx".to_string()));
        assert_eq!(options.jsx_import_source, Some("react".to_string()));
    }

    #[test]
    fn test_compiler_options_default() {
        let options = CompilerOptions::default();
        assert!(options.target.is_none());
        assert!(options.module.is_none());
        assert!(options.strict.is_none());
    }

    #[test]
    fn test_tsconfig_merge() {
        let base = TsConfig {
            compiler_options: Some(CompilerOptions {
                target: Some("ES2015".to_string()),
                strict: Some(true),
                ..Default::default()
            }),
            include: Some(vec!["src/**/*".to_string()]),
            ..Default::default()
        };

        let child = TsConfig {
            compiler_options: Some(CompilerOptions {
                target: Some("ES2020".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };

        let merged = child.merge_with_base(base);
        let options = merged.compiler_options.unwrap();

        // Child overrides target
        assert_eq!(options.target, Some("ES2020".to_string()));
        // Inherits strict from base
        assert_eq!(options.strict, Some(true));
        // Inherits include from base
        assert_eq!(merged.include, Some(vec!["src/**/*".to_string()]));
    }

    #[test]
    fn test_tsconfig_error_display() {
        let io_error = TsConfigError::IoError("File not found".to_string());
        assert!(io_error.to_string().contains("IO error"));

        let parse_error = TsConfigError::ParseError("Invalid JSON".to_string());
        assert!(parse_error.to_string().contains("Parse error"));
    }

    #[test]
    fn test_project_reference() {
        let json = r#"{
            "references": [
                { "path": "./packages/core" },
                { "path": "./packages/utils", "prepend": true }
            ]
        }"#;

        let config: TsConfig = serde_json::from_str(json).unwrap();
        let refs = config.references.unwrap();
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0].path, "./packages/core");
        assert!(refs[1].prepend.unwrap());
    }

    #[test]
    fn test_tsconfig_with_comments() {
        let json = r#"{
            // Compiler options
            "compilerOptions": {
                "target": "ES2020" /* inline comment */
            }
        }"#;

        let cleaned = remove_json_comments(json);
        let config: TsConfig = serde_json::from_str(&cleaned).unwrap();
        let options = config.compiler_options.unwrap();
        assert_eq!(options.target, Some("ES2020".to_string()));
    }
}
