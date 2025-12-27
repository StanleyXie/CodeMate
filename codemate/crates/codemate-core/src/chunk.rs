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
    /// Detect language from file extension.
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "rs" => Language::Rust,
            "py" | "pyi" => Language::Python,
            "ts" | "tsx" => Language::TypeScript,
            "js" | "jsx" | "mjs" => Language::JavaScript,
            "go" => Language::Go,
            "java" => Language::Java,
            "tf" | "tfvars" | "hcl" => Language::Hcl,
            _ => Language::Unknown,
        }
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
    /// Line count
    pub line_count: usize,
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
            line_count,
        }
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
