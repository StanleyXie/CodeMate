//! Chunk types for code elements.

use crate::ContentHash;
use serde::{Deserialize, Serialize};

/// Programming language of a chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Rust,
    Python,
    TypeScript,
    JavaScript,
    Go,
    Java,
    Hcl,
    Unknown,
}

impl Language {
    /// Detect language from file extension or name.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "rs" | "rust" => Language::Rust,
            "py" | "pyi" | "python" => Language::Python,
            "ts" | "tsx" | "typescript" => Language::TypeScript,
            "js" | "jsx" | "mjs" | "javascript" => Language::JavaScript,
            "go" | "golang" => Language::Go,
            "java" => Language::Java,
            "tf" | "tfvars" | "hcl" | "terraform" => Language::Hcl,
            _ => Language::Unknown,
        }
    }

    /// Detect language from file extension.
    pub fn from_extension(ext: &str) -> Self {
        Self::from_str(ext)
    }

    /// Get the language name as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::Python => "python",
            Language::TypeScript => "typescript",
            Language::JavaScript => "javascript",
            Language::Go => "go",
            Language::Java => "java",
            Language::Hcl => "hcl",
            Language::Unknown => "unknown",
        }
    }
}

/// Kind of code chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChunkKind {
    /// Function or method definition
    Function,
    /// Class definition
    Class,
    /// Struct definition
    Struct,
    /// Trait/interface definition
    Trait,
    /// Enum definition
    Enum,
    /// Module/namespace
    Module,
    /// Implementation block
    Impl,
    /// Top-level code block
    Block,
    /// Terraform/HCL resource
    Resource,
    /// Terraform/HCL data source
    DataSource,
    /// Terraform/HCL variable
    Variable,
    /// Terraform/HCL output
    Output,
}

/// A chunk of code with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    /// Content hash (unique identifier)
    pub content_hash: ContentHash,
    /// The actual source code
    pub content: String,
    /// Programming language
    pub language: Language,
    /// Kind of chunk
    pub kind: ChunkKind,
    /// Symbol name (function name, class name, etc.)
    pub symbol_name: Option<String>,
    /// Full signature if available
    pub signature: Option<String>,
    /// Docstring or comment
    pub docstring: Option<String>,
    /// Byte size
    pub byte_size: usize,
    /// Starting line (1-indexed)
    pub line_start: usize,
    /// Ending line (1-indexed)
    pub line_end: usize,
    /// Line count
    pub line_count: usize,
    /// Module ID (for project-level grouping)
    pub module_id: Option<String>,
}


impl Chunk {
    /// Create a new chunk from content.
    pub fn new(
        content: String,
        language: Language,
        kind: ChunkKind,
        symbol_name: Option<String>,
    ) -> Self {
        let byte_size = content.len();
        let line_count = content.lines().count();
        let content_hash = ContentHash::from_content(content.as_bytes());

        Self {
            content_hash,
            content,
            language,
            kind,
            symbol_name,
            signature: None,
            docstring: None,
            byte_size,
            line_start: 0,
            line_end: 0,
            line_count,
            module_id: None,
        }
    }


    /// Set the line range.
    pub fn with_line_range(mut self, start: usize, end: usize) -> Self {
        self.line_start = start;
        self.line_end = end;
        self
    }

    /// Set the signature.
    pub fn with_signature(mut self, signature: String) -> Self {
        self.signature = Some(signature);
        self
    }

    /// Set the docstring.
    pub fn with_docstring(mut self, docstring: String) -> Self {
        self.docstring = Some(docstring);
        self
    }

    /// Set the module ID.
    pub fn with_module_id(mut self, module_id: String) -> Self {
        self.module_id = Some(module_id);
        self
    }
}

/// Type of project/module for hierarchical organization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectType {
    /// Rust workspace (has [workspace] in Cargo.toml)
    Workspace,
    /// Rust crate (has Cargo.toml)
    Crate,
    /// Python package (has pyproject.toml or setup.py)
    Package,
    /// JavaScript/TypeScript project (has package.json)
    NpmPackage,
    /// Go module (has go.mod)
    GoModule,
    /// Java project (has pom.xml or build.gradle)
    JavaProject,
    /// Terraform root module
    TerraformModule,
    /// Generic directory-based module
    Directory,
}

impl ProjectType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProjectType::Workspace => "workspace",
            ProjectType::Crate => "crate",
            ProjectType::Package => "package",
            ProjectType::NpmPackage => "npm_package",
            ProjectType::GoModule => "go_module",
            ProjectType::JavaProject => "java_project",
            ProjectType::TerraformModule => "terraform_module",
            ProjectType::Directory => "directory",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "workspace" => ProjectType::Workspace,
            "crate" => ProjectType::Crate,
            "package" => ProjectType::Package,
            "npm_package" => ProjectType::NpmPackage,
            "go_module" => ProjectType::GoModule,
            "java_project" => ProjectType::JavaProject,
            "terraform_module" => ProjectType::TerraformModule,
            _ => ProjectType::Directory,
        }
    }
}

/// A module/project detected in the codebase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Module {
    /// Unique identifier (derived from path)
    pub id: String,
    /// Display name
    pub name: String,
    /// Path relative to index root
    pub path: String,
    /// Primary language
    pub language: Language,
    /// Type of project/module
    pub project_type: ProjectType,
    /// Parent module ID (for nested modules)
    pub parent_id: Option<String>,
}

impl Module {
    /// Create a new module.
    pub fn new(
        name: String,
        path: String,
        language: Language,
        project_type: ProjectType,
    ) -> Self {
        // Generate ID from path
        let id = path.replace('/', "::").replace('\\', "::");
        Self {
            id,
            name,
            path,
            language,
            project_type,
            parent_id: None,
        }
    }

    /// Set the parent module ID.
    pub fn with_parent(mut self, parent_id: String) -> Self {
        self.parent_id = Some(parent_id);
        self
    }
}

/// Location of a chunk in a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkLocation {
    /// Content hash of the chunk
    pub content_hash: ContentHash,
    /// File path relative to repository root
    pub file_path: String,
    /// Starting byte offset
    pub byte_start: usize,
    /// Ending byte offset
    pub byte_end: usize,
    /// Starting line (1-indexed)
    pub line_start: usize,
    /// Ending line (1-indexed)
    pub line_end: usize,
    /// Git commit hash where this location was recorded
    pub commit_hash: Option<String>,
    /// Author of the original code (from git blame)
    pub author: Option<String>,
    /// Timestamp when the code was last modified
    pub timestamp: Option<String>,
}

impl ChunkLocation {
    /// Create a new chunk location.
    pub fn new(
        content_hash: ContentHash,
        file_path: String,
        byte_start: usize,
        byte_end: usize,
        line_start: usize,
        line_end: usize,
    ) -> Self {
        Self {
            content_hash,
            file_path,
            byte_start,
            byte_end,
            line_start,
            line_end,
            commit_hash: None,
            author: None,
            timestamp: None,
        }
    }

    /// Set git commit info.
    pub fn with_commit(mut self, commit_hash: String) -> Self {
        self.commit_hash = Some(commit_hash);
        self
    }

    /// Set author info.
    pub fn with_author(mut self, author: String) -> Self {
        self.author = Some(author);
        self
    }

    /// Set timestamp.
    pub fn with_timestamp(mut self, timestamp: String) -> Self {
        self.timestamp = Some(timestamp);
        self
    }
}

/// Kind of relationship between code elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EdgeKind {
    /// Function or method call
    Calls,
    /// Module or file import
    Imports,
    /// Reference to a symbol
    References,
}

impl EdgeKind {
    /// Get the kind as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            EdgeKind::Calls => "calls",
            EdgeKind::Imports => "imports",
            EdgeKind::References => "references",
        }
    }
}

/// A directed relationship between two code elements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    /// Content hash of the source chunk
    pub source_hash: ContentHash,
    /// Query for the target (e.g., "GitRepository::open" or "std::path::Path")
    pub target_query: String,
    /// Kind of relationship
    pub kind: EdgeKind,
    /// Line number in the source file where this edge originates
    pub line_number: Option<usize>,
}

impl Edge {
    /// Create a new edge.
    pub fn new(source_hash: ContentHash, target_query: String, kind: EdgeKind) -> Self {
        Self {
            source_hash,
            target_query,
            kind,
            line_number: None,
        }
    }

    /// Set the line number.
    pub fn with_line(mut self, line: usize) -> Self {
        self.line_number = Some(line);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_detection() {
        assert_eq!(Language::from_extension("rs"), Language::Rust);
        assert_eq!(Language::from_extension("py"), Language::Python);
        assert_eq!(Language::from_extension("ts"), Language::TypeScript);
        assert_eq!(Language::from_extension("xyz"), Language::Unknown);
    }

    #[test]
    fn test_chunk_creation() {
        let chunk = Chunk::new(
            "fn main() {}".to_string(),
            Language::Rust,
            ChunkKind::Function,
            Some("main".to_string()),
        );
        
        assert_eq!(chunk.symbol_name, Some("main".to_string()));
        assert_eq!(chunk.language, Language::Rust);
        assert_eq!(chunk.kind, ChunkKind::Function);
        assert_eq!(chunk.byte_size, 12);
    }
}
