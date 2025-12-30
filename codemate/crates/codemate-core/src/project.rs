//! Project and module detection.
//!
//! Detects project boundaries by scanning for marker files like Cargo.toml, package.json, etc.

use crate::chunk::{Language, Module, ProjectType};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Detects project and module boundaries in a codebase.
pub struct ProjectDetector {
    /// Detected modules indexed by path
    modules: HashMap<String, Module>,
    /// Root path being indexed
    root_path: PathBuf,
}

impl ProjectDetector {
    /// Create a new project detector for the given root path.
    pub fn new(root_path: &Path) -> Self {
        Self {
            modules: HashMap::new(),
            root_path: root_path.to_path_buf(),
        }
    }

    /// Set detected modules.
    pub fn set_modules(&mut self, modules: Vec<Module>) {
        for module in modules {
            self.modules.insert(module.id.clone(), module);
        }
    }

    /// Scan the directory tree and detect all modules.
    pub fn detect_modules(&mut self) -> Vec<Module> {
        self.scan_directory(&self.root_path.clone(), None);
        self.modules.values().cloned().collect()
    }
    
    /// Scan a directory for project markers.
    fn scan_directory(&mut self, dir: &Path, parent_id: Option<String>) {
        let rel_path = self.relative_path(dir);
        
        // Determine standardized ID: root or path components joined by ::
        let current_id = if rel_path.is_empty() {
            "root".to_string()
        } else {
            rel_path.replace('/', "::").replace('\\', "::")
        };

        // 1. Detect if this is a project module
        let detected = if let Some(module) = self.detect_rust_project(dir) {
            Some(module)
        } else if let Some(module) = self.detect_python_project(dir) {
            Some(module)
        } else if let Some(module) = self.detect_node_project(dir) {
            Some(module)
        } else if let Some(module) = self.detect_go_project(dir) {
            Some(module)
        } else if let Some(module) = self.detect_java_project(dir) {
            Some(module)
        } else if let Some(module) = self.detect_terraform_project(dir) {
            Some(module)
        } else {
            None
        };

        let mut module = if let Some(mut m) = detected {
            m.id = current_id.clone();
            m
        } else {
            let name = if rel_path.is_empty() {
                "root".to_string()
            } else {
                self.dir_name(dir).unwrap_or_else(|| "dir".to_string())
            };
            let mut m = Module::new(name, rel_path.clone(), Language::Unknown, ProjectType::Directory);
            m.id = current_id.clone();
            m
        };

        if let Some(ref pid) = parent_id {
            if pid != &module.id {
                module = module.with_parent(pid.clone());
            }
        }

        self.modules.insert(module.id.clone(), module);
        let next_parent_id = Some(current_id);
        
        // Recursively scan subdirectories
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && !self.should_skip_dir(&path) {
                    self.scan_directory(&path, next_parent_id.clone());
                }
            }
        }
    }

    /// Check for Rust project (Cargo.toml) or sub-module (mod.rs, lib.rs).
    fn detect_rust_project(&self, dir: &Path) -> Option<Module> {
        let cargo_toml = dir.join("Cargo.toml");
        if cargo_toml.exists() {
            let content = std::fs::read_to_string(&cargo_toml).ok()?;
            
            // Check if it's a workspace
            let project_type = if content.contains("[workspace]") {
                ProjectType::Workspace
            } else {
                ProjectType::Crate
            };

            // Extract package name from Cargo.toml
            let name = self.extract_toml_value(&content, "name")
                .or_else(|| self.dir_name(dir))?;

            let rel_path = self.relative_path(dir);
            Some(Module::new(name, rel_path, Language::Rust, project_type))
        } else if dir.join("mod.rs").exists() || dir.join("lib.rs").exists() {
            // Sub-module
            let name = self.dir_name(dir)?;
            let rel_path = self.relative_path(dir);
            Some(Module::new(name, rel_path, Language::Rust, ProjectType::Directory))
        } else {
            None
        }
    }

    /// Check for Python project (pyproject.toml, setup.py, __init__.py).
    fn detect_python_project(&self, dir: &Path) -> Option<Module> {
        let markers = ["pyproject.toml", "setup.py", "setup.cfg"];
        
        for marker in markers {
            if dir.join(marker).exists() {
                let name = self.dir_name(dir)?;
                let rel_path = self.relative_path(dir);
                return Some(Module::new(name, rel_path, Language::Python, ProjectType::Package));
            }
        }
        
        // Check for __init__.py (Python package/sub-module)
        if dir.join("__init__.py").exists() {
            let name = self.dir_name(dir)?;
            let rel_path = self.relative_path(dir);
            return Some(Module::new(name, rel_path, Language::Python, ProjectType::Directory));
        }
        
        None
    }

    /// Check for Node.js/TypeScript project (package.json).
    fn detect_node_project(&self, dir: &Path) -> Option<Module> {
        let package_json = dir.join("package.json");
        if package_json.exists() {
            let content = std::fs::read_to_string(&package_json).ok()?;
            
            // Extract name from package.json
            let name = self.extract_json_value(&content, "name")
                .or_else(|| self.dir_name(dir))?;

            // Detect TypeScript vs JavaScript
            let language = if dir.join("tsconfig.json").exists() {
                Language::TypeScript
            } else {
                Language::JavaScript
            };

            let rel_path = self.relative_path(dir);
            Some(Module::new(name, rel_path, language, ProjectType::NpmPackage))
        } else {
            None
        }
    }

    /// Check for Go project (go.mod).
    fn detect_go_project(&self, dir: &Path) -> Option<Module> {
        let go_mod = dir.join("go.mod");
        if go_mod.exists() {
            let content = std::fs::read_to_string(&go_mod).ok()?;
            
            // Extract module name from go.mod
            let name = content.lines()
                .find(|line| line.starts_with("module "))
                .map(|line| line.trim_start_matches("module ").trim().to_string())
                .or_else(|| self.dir_name(dir))?;

            let rel_path = self.relative_path(dir);
            Some(Module::new(name, rel_path, Language::Go, ProjectType::GoModule))
        } else {
            None
        }
    }

    /// Check for Java project (pom.xml, build.gradle).
    fn detect_java_project(&self, dir: &Path) -> Option<Module> {
        let markers = ["pom.xml", "build.gradle", "build.gradle.kts"];
        
        for marker in markers {
            if dir.join(marker).exists() {
                let name = self.dir_name(dir)?;
                let rel_path = self.relative_path(dir);
                return Some(Module::new(name, rel_path, Language::Java, ProjectType::JavaProject));
            }
        }
        None
    }

    /// Check for Terraform project (*.tf files).
    fn detect_terraform_project(&self, dir: &Path) -> Option<Module> {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Some(ext) = entry.path().extension() {
                    if ext == "tf" {
                        let name = self.dir_name(dir)?;
                        let rel_path = self.relative_path(dir);
                        return Some(Module::new(name, rel_path, Language::Hcl, ProjectType::TerraformModule));
                    }
                }
            }
        }
        None
    }

    /// Get directory name as String.
    fn dir_name(&self, dir: &Path) -> Option<String> {
        dir.file_name()?.to_str().map(|s| s.to_string())
    }

    /// Get relative path from root.
    fn relative_path(&self, path: &Path) -> String {
        path.strip_prefix(&self.root_path)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string()
    }

    /// Check if directory should be skipped.
    fn should_skip_dir(&self, path: &Path) -> bool {
        let name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        
        if name.starts_with('.') && name != "." && name != ".." {
            return true;
        }

        matches!(name, 
            "node_modules" | "target" | ".git" | "__pycache__" | 
            "venv" | ".venv" | "vendor" | "dist" | "build" | ".terraform"
        )
    }

    /// Extract value from TOML content (simple regex-free parser).
    fn extract_toml_value(&self, content: &str, key: &str) -> Option<String> {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with(&format!("{} =", key)) || trimmed.starts_with(&format!("{}=", key)) {
                // Extract value between quotes
                if let Some(start) = trimmed.find('"') {
                    if let Some(end) = trimmed[start + 1..].find('"') {
                        return Some(trimmed[start + 1..start + 1 + end].to_string());
                    }
                }
            }
        }
        None
    }

    /// Extract value from JSON content (simple regex-free parser).
    fn extract_json_value(&self, content: &str, key: &str) -> Option<String> {
        let pattern = format!("\"{}\"", key);
        for line in content.lines() {
            if line.contains(&pattern) {
                // Find the value after the colon
                if let Some(colon_pos) = line.find(':') {
                    let value_part = &line[colon_pos + 1..];
                    if let Some(start) = value_part.find('"') {
                        if let Some(end) = value_part[start + 1..].find('"') {
                            return Some(value_part[start + 1..start + 1 + end].to_string());
                        }
                    }
                }
            }
        }
        None
    }

    /// Find the module that contains a given file path.
    pub fn find_module_for_file(&self, file_path: &Path) -> Option<&Module> {
        let rel_path = file_path.strip_prefix(&self.root_path).ok()?;
        
        // Find the deepest matching module
        let mut best_match: Option<&Module> = None;
        let mut best_depth = 0;

        for module in self.modules.values() {
            let module_path = Path::new(&module.path);
            if rel_path.starts_with(module_path) || module.path.is_empty() {
                let depth = module.path.matches('/').count() + module.path.matches('\\').count();
                if depth >= best_depth {
                    best_depth = depth;
                    best_match = Some(module);
                }
            }
        }

        best_match
    }

    /// Get module ID for a file path.
    pub fn get_module_id_for_file(&self, file_path: &Path) -> Option<String> {
        self.find_module_for_file(file_path).map(|m| m.id.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_detect_rust_crate() {
        let temp_dir = TempDir::new().unwrap();
        let crate_dir = temp_dir.path().join("my_crate");
        fs::create_dir(&crate_dir).unwrap();
        fs::write(crate_dir.join("Cargo.toml"), r#"
[package]
name = "my_crate"
version = "0.1.0"
"#).unwrap();

        let mut detector = ProjectDetector::new(temp_dir.path());
        let modules = detector.detect_modules();

        assert_eq!(modules.len(), 1);
        assert_eq!(modules[0].name, "my_crate");
        assert_eq!(modules[0].project_type, ProjectType::Crate);
        assert_eq!(modules[0].language, Language::Rust);
    }

    #[test]
    fn test_detect_rust_workspace() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("Cargo.toml"), r#"
[workspace]
members = ["crates/*"]
"#).unwrap();

        let mut detector = ProjectDetector::new(temp_dir.path());
        let modules = detector.detect_modules();

        assert_eq!(modules.len(), 1);
        assert_eq!(modules[0].project_type, ProjectType::Workspace);
    }

    #[test]
    fn test_detect_node_project() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("package.json"), r#"
{
  "name": "my-app",
  "version": "1.0.0"
}
"#).unwrap();

        let mut detector = ProjectDetector::new(temp_dir.path());
        let modules = detector.detect_modules();

        assert_eq!(modules.len(), 1);
        assert_eq!(modules[0].name, "my-app");
        assert_eq!(modules[0].project_type, ProjectType::NpmPackage);
    }

    #[test]
    fn test_find_module_for_file() {
        let temp_dir = TempDir::new().unwrap();
        let crate_dir = temp_dir.path().join("crates").join("my_crate");
        fs::create_dir_all(&crate_dir).unwrap();
        fs::write(crate_dir.join("Cargo.toml"), r#"
[package]
name = "my_crate"
"#).unwrap();
        fs::create_dir(crate_dir.join("src")).unwrap();
        fs::write(crate_dir.join("src/lib.rs"), "// code").unwrap();

        let mut detector = ProjectDetector::new(temp_dir.path());
        detector.detect_modules();

        let file_path = crate_dir.join("src/lib.rs");
        let module_id = detector.get_module_id_for_file(&file_path);
        
        assert!(module_id.is_some());
        assert!(module_id.unwrap().contains("my_crate"));
    }
}
