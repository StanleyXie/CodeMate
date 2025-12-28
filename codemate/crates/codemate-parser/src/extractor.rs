//! Chunk extraction from source code using tree-sitter.

use codemate_core::{Chunk, ChunkKind, Language, Edge, EdgeKind, Result, ContentHash};
use std::path::Path;

/// Extracts chunks from source code files.
pub struct ChunkExtractor {
    /// Maximum chunk size in lines
    pub max_lines: usize,
}

impl Default for ChunkExtractor {
    fn default() -> Self {
        Self { max_lines: 100 }
    }
}

impl ChunkExtractor {
    /// Create a new chunk extractor.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum lines per chunk.
    pub fn with_max_lines(mut self, max_lines: usize) -> Self {
        self.max_lines = max_lines;
        self
    }

    /// Extract chunks and edges from a file.
    pub fn extract_file(&self, path: &Path) -> Result<(Vec<Chunk>, Vec<Edge>)> {
        let content = std::fs::read_to_string(path)?;
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let language = Language::from_extension(extension);

        self.extract(&content, language)
    }

    /// Extract chunks and edges from source code.
    pub fn extract(&self, content: &str, language: Language) -> Result<(Vec<Chunk>, Vec<Edge>)> {
        match language {
            Language::Rust => self.extract_rust(content),
            Language::Python => self.extract_python(content),
            Language::TypeScript | Language::JavaScript => self.extract_typescript(content, language),
            Language::Go => self.extract_go(content),
            Language::Hcl => self.extract_hcl(content),
            _ => self.extract_fallback(content, language),
        }
    }

    /// Extract chunks from Rust source code.
    fn extract_rust(&self, content: &str) -> Result<(Vec<Chunk>, Vec<Edge>)> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .map_err(|e| codemate_core::Error::Parse(e.to_string()))?;

        let tree = parser
            .parse(content, None)
            .ok_or_else(|| codemate_core::Error::Parse("Failed to parse Rust".to_string()))?;

        let mut chunks = Vec::new();
        let mut edges = Vec::new();
        self.extract_rust_nodes(&tree.root_node(), content, &mut chunks, &mut edges);
        Ok((chunks, edges))
    }

    fn extract_rust_nodes(
        &self,
        node: &tree_sitter::Node,
        content: &str,
        chunks: &mut Vec<Chunk>,
        edges: &mut Vec<Edge>,
    ) {
        // Extract function definitions, structs, enums, traits, impls
        match node.kind() {
            "function_item" => {
                if let Some(chunk) = self.node_to_chunk(node, content, Language::Rust, ChunkKind::Function) {
                    self.extract_rust_edges(node, content, &chunk, edges);
                    chunks.push(chunk);
                }
            }
            "struct_item" => {
                if let Some(chunk) = self.node_to_chunk(node, content, Language::Rust, ChunkKind::Struct) {
                    chunks.push(chunk);
                }
            }
            "enum_item" => {
                if let Some(chunk) = self.node_to_chunk(node, content, Language::Rust, ChunkKind::Enum) {
                    chunks.push(chunk);
                }
            }
            "trait_item" => {
                if let Some(chunk) = self.node_to_chunk(node, content, Language::Rust, ChunkKind::Trait) {
                    chunks.push(chunk);
                }
            }
            "impl_item" => {
                if let Some(chunk) = self.node_to_chunk(node, content, Language::Rust, ChunkKind::Impl) {
                    chunks.push(chunk);
                }
            }
            "mod_item" => {
                if let Some(chunk) = self.node_to_chunk(node, content, Language::Rust, ChunkKind::Module) {
                    chunks.push(chunk);
                }
            }
            "use_declaration" => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "use_path" {
                        if let Ok(text) = child.utf8_text(content.as_bytes()) {
                            edges.push(Edge::new(
                                // For file-level imports, we don't have a source chunk yet
                                // so we'll associate it with a dummy "file" chunk or similar if needed.
                                // Actually, let's just stick to function-level calls for now
                                // or find a way to associate with the module/file chunk.
                                // If chunks is empty, it's a file header import.
                                ContentHash::from_content(content.as_bytes()),
                                text.to_string(),
                                EdgeKind::Imports,
                            ).with_line(child.start_position().row + 1));
                        }
                    }
                }
            }
            "call_expression" => {
                // If we are inside a function, we'll handle this in extract_rust_edges
            }
            _ => {
                // Recurse into children
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.extract_rust_nodes(&child, content, chunks, edges);
                }
            }
        }
    }

    fn extract_rust_edges(&self, node: &tree_sitter::Node, content: &str, source_chunk: &Chunk, edges: &mut Vec<Edge>) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "call_expression" {
                if let Some(target) = self.extract_rust_call_target(&child, content) {
                    edges.push(Edge::new(
                        source_chunk.content_hash.clone(),
                        target,
                        EdgeKind::Calls,
                    ).with_line(child.start_position().row + 1));
                }
            }
            // Recurse to find nested calls
            self.extract_rust_edges(&child, content, source_chunk, edges);
        }
    }

    fn extract_rust_call_target(&self, node: &tree_sitter::Node, content: &str) -> Option<String> {
        // In Rust, a call_expression's first child is the function being called (field "function")
        if let Some(function_node) = node.child_by_field_name("function") {
            return Some(function_node.utf8_text(content.as_bytes()).ok()?.to_string());
        }
        None
    }

    fn node_to_chunk(
        &self,
        node: &tree_sitter::Node,
        content: &str,
        language: Language,
        kind: ChunkKind,
    ) -> Option<Chunk> {
        let text = node.utf8_text(content.as_bytes()).ok()?;
        let line_count = text.lines().count();

        // Skip if too large
        if line_count > self.max_lines {
            return None;
        }

        // Extract symbol name
        let symbol_name = self.extract_symbol_name(node, content);

        let start_pos = node.start_position();
        let end_pos = node.end_position();

        Some(Chunk::new(
            text.to_string(),
            language,
            kind,
            symbol_name,
        ).with_line_range(start_pos.row + 1, end_pos.row + 1))
    }

    fn extract_symbol_name(&self, node: &tree_sitter::Node, content: &str) -> Option<String> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" || child.kind() == "type_identifier" {
                return child.utf8_text(content.as_bytes()).ok().map(String::from);
            }
            if child.kind() == "name" {
                return child.utf8_text(content.as_bytes()).ok().map(String::from);
            }
        }
        None
    }

    /// Extract chunks from Python source code.
    fn extract_python(&self, content: &str) -> Result<(Vec<Chunk>, Vec<Edge>)> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .map_err(|e| codemate_core::Error::Parse(e.to_string()))?;

        let tree = parser
            .parse(content, None)
            .ok_or_else(|| codemate_core::Error::Parse("Failed to parse Python".to_string()))?;

        let mut chunks = Vec::new();
        let mut edges = Vec::new();
        self.extract_python_nodes(&tree.root_node(), content, &mut chunks, &mut edges);
        Ok((chunks, edges))
    }

    fn extract_python_nodes(
        &self,
        node: &tree_sitter::Node,
        content: &str,
        chunks: &mut Vec<Chunk>,
        edges: &mut Vec<Edge>,
    ) {
        match node.kind() {
            "function_definition" => {
                if let Some(chunk) = self.node_to_chunk(node, content, Language::Python, ChunkKind::Function) {
                    self.extract_python_edges(node, content, &chunk, edges);
                    chunks.push(chunk);
                }
            }
            "class_definition" => {
                if let Some(chunk) = self.node_to_chunk(node, content, Language::Python, ChunkKind::Class) {
                    chunks.push(chunk);
                }
            }
            "import_statement" | "import_from_statement" => {
                if let Ok(text) = node.utf8_text(content.as_bytes()) {
                     edges.push(Edge::new(
                        ContentHash::from_content(content.as_bytes()),
                        text.trim().to_string(),
                        EdgeKind::Imports,
                    ).with_line(node.start_position().row + 1));
                }
            }
            _ => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.extract_python_nodes(&child, content, chunks, edges);
                }
            }
        }
    }

    fn extract_python_edges(&self, node: &tree_sitter::Node, content: &str, source_chunk: &Chunk, edges: &mut Vec<Edge>) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "call" || child.kind() == "call_expression" {
                if let Some(target) = self.extract_python_call_target(&child, content) {
                    edges.push(Edge::new(
                        source_chunk.content_hash.clone(),
                        target,
                        EdgeKind::Calls,
                    ).with_line(child.start_position().row + 1));
                }
            }
            self.extract_python_edges(&child, content, source_chunk, edges);
        }
    }

    fn extract_python_call_target(&self, node: &tree_sitter::Node, content: &str) -> Option<String> {
        // In Python, a call's first child is the function being called (field "function")
        if let Some(function_node) = node.child_by_field_name("function") {
            return function_node.utf8_text(content.as_bytes()).ok().map(|s| s.to_string());
        }
        
        // Fallback: try to find an identifier child
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" {
                return child.utf8_text(content.as_bytes()).ok().map(|s| s.to_string());
            }
        }
        
        None
    }

    /// Extract chunks from TypeScript/JavaScript source code.
    fn extract_typescript(&self, content: &str, language: Language) -> Result<(Vec<Chunk>, Vec<Edge>)> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .map_err(|e| codemate_core::Error::Parse(e.to_string()))?;

        let tree = parser
            .parse(content, None)
            .ok_or_else(|| codemate_core::Error::Parse("Failed to parse TypeScript".to_string()))?;

        let mut chunks = Vec::new();
        let mut edges = Vec::new();
        self.extract_typescript_nodes(&tree.root_node(), content, language, &mut chunks, &mut edges);
        Ok((chunks, edges))
    }

    fn extract_typescript_nodes(
        &self,
        node: &tree_sitter::Node,
        content: &str,
        language: Language,
        chunks: &mut Vec<Chunk>,
        edges: &mut Vec<Edge>,
    ) {
        match node.kind() {
            "function_declaration" | "arrow_function" | "method_definition" => {
                if let Some(chunk) = self.node_to_chunk(node, content, language, ChunkKind::Function) {
                    self.extract_typescript_edges(node, content, &chunk, edges);
                    chunks.push(chunk);
                }
            }
            "class_declaration" => {
                if let Some(chunk) = self.node_to_chunk(node, content, language, ChunkKind::Class) {
                    chunks.push(chunk);
                }
            }
            "import_declaration" => {
                // TODO: Extract imports
            }
            _ => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.extract_typescript_nodes(&child, content, language, chunks, edges);
                }
            }
        }
    }

    fn extract_typescript_edges(&self, node: &tree_sitter::Node, content: &str, source_chunk: &Chunk, edges: &mut Vec<Edge>) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "call_expression" {
                if let Some(target) = self.extract_typescript_call_target(&child, content) {
                    edges.push(Edge::new(
                        source_chunk.content_hash.clone(),
                        target,
                        EdgeKind::Calls,
                    ).with_line(child.start_position().row + 1));
                }
            }
            self.extract_typescript_edges(&child, content, source_chunk, edges);
        }
    }

    fn extract_typescript_call_target(&self, node: &tree_sitter::Node, content: &str) -> Option<String> {
        // In TypeScript, a call_expression's first child is the function being called (field "function")
        if let Some(function_node) = node.child_by_field_name("function") {
            return Some(function_node.utf8_text(content.as_bytes()).ok()?.to_string());
        }
        None
    }

    /// Extract chunks from Go source code.
    fn extract_go(&self, content: &str) -> Result<(Vec<Chunk>, Vec<Edge>)> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_go::LANGUAGE.into())
            .map_err(|e| codemate_core::Error::Parse(e.to_string()))?;

        let tree = parser
            .parse(content, None)
            .ok_or_else(|| codemate_core::Error::Parse("Failed to parse Go".to_string()))?;

        let mut chunks = Vec::new();
        let mut edges = Vec::new();
        self.extract_go_nodes(&tree.root_node(), content, &mut chunks, &mut edges);
        Ok((chunks, edges))
    }

    fn extract_go_nodes(
        &self,
        node: &tree_sitter::Node,
        content: &str,
        chunks: &mut Vec<Chunk>,
        edges: &mut Vec<Edge>,
    ) {
        match node.kind() {
            "function_declaration" | "method_declaration" => {
                if let Some(chunk) = self.node_to_chunk(node, content, Language::Go, ChunkKind::Function) {
                    self.extract_go_edges(node, content, &chunk, edges);
                    chunks.push(chunk);
                }
            }
            "type_declaration" => {
                // Check if it's a struct or interface
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "type_spec" {
                        let mut inner_cursor = child.walk();
                        for inner_child in child.children(&mut inner_cursor) {
                            let kind = match inner_child.kind() {
                                "struct_type" => Some(ChunkKind::Struct),
                                "interface_type" => Some(ChunkKind::Trait),
                                _ => None,
                            };
                            if let Some(k) = kind {
                                if let Some(chunk) = self.node_to_chunk(node, content, Language::Go, k) {
                                    chunks.push(chunk);
                                }
                            }
                        }
                    }
                }
            }
            "import_declaration" => {
                // TODO: Extract imports
            }
            _ => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.extract_go_nodes(&child, content, chunks, edges);
                }
            }
        }
    }

    fn extract_go_edges(&self, node: &tree_sitter::Node, content: &str, source_chunk: &Chunk, edges: &mut Vec<Edge>) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "call_expression" {
                if let Some(target) = self.extract_go_call_target(&child, content) {
                    edges.push(Edge::new(
                        source_chunk.content_hash.clone(),
                        target,
                        EdgeKind::Calls,
                    ).with_line(child.start_position().row + 1));
                }
            }
            self.extract_go_edges(&child, content, source_chunk, edges);
        }
    }

    fn extract_go_call_target(&self, node: &tree_sitter::Node, content: &str) -> Option<String> {
        // In Go, a call_expression's first child is the function being called (field "function")
        if let Some(function_node) = node.child_by_field_name("function") {
            return Some(function_node.utf8_text(content.as_bytes()).ok()?.to_string());
        }
        None
    }

    /// Extract chunks from HCL/Terraform source code.
    fn extract_hcl(&self, content: &str) -> Result<(Vec<Chunk>, Vec<Edge>)> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_hcl::LANGUAGE.into())
            .map_err(|e| codemate_core::Error::Parse(e.to_string()))?;

        let tree = parser
            .parse(content, None)
            .ok_or_else(|| codemate_core::Error::Parse("Failed to parse HCL".to_string()))?;

        let mut chunks = Vec::new();
        let mut edges = Vec::new();
        self.extract_hcl_nodes(&tree.root_node(), content, &mut chunks, &mut edges);
        Ok((chunks, edges))
    }

    fn extract_hcl_nodes(
        &self,
        node: &tree_sitter::Node,
        content: &str,
        chunks: &mut Vec<Chunk>,
        edges: &mut Vec<Edge>,
    ) {
        match node.kind() {
            "block" => {
                // Get the block type (resource, data, variable, output, etc.)
                if let Some(block_type) = self.get_hcl_block_type(node, content) {
                    let kind = match block_type.as_str() {
                        "resource" => ChunkKind::Resource,
                        "data" => ChunkKind::DataSource,
                        "variable" => ChunkKind::Variable,
                        "output" => ChunkKind::Output,
                        "module" => ChunkKind::Module,
                        "provider" => ChunkKind::Block,
                        "locals" => ChunkKind::Block,
                        "terraform" => ChunkKind::Block,
                        _ => ChunkKind::Block,
                    };

                    let symbol_name = self.get_hcl_resource_name(node, content);
                    let text = node.utf8_text(content.as_bytes()).ok();
                    
                    if let Some(text) = text {
                        let line_count = text.lines().count();
                        if line_count <= self.max_lines {
                            chunks.push(Chunk::new(
                                text.to_string(),
                                Language::Hcl,
                                kind,
                                symbol_name,
                            ));
                        }
                    }
                }
            }
            _ => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.extract_hcl_nodes(&child, content, chunks, edges);
                }
            }
        }
    }

    fn get_hcl_block_type(&self, node: &tree_sitter::Node, content: &str) -> Option<String> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" {
                return child.utf8_text(content.as_bytes()).ok().map(String::from);
            }
        }
        None
    }

    fn get_hcl_resource_name(&self, node: &tree_sitter::Node, content: &str) -> Option<String> {
        let mut cursor = node.walk();
        let mut labels = Vec::new();
        for child in node.children(&mut cursor) {
            if child.kind() == "string_lit" {
                if let Ok(text) = child.utf8_text(content.as_bytes()) {
                    // Remove quotes
                    let cleaned = text.trim_matches('"');
                    labels.push(cleaned.to_string());
                }
            }
        }
        if labels.is_empty() {
            None
        } else {
            Some(labels.join("."))
        }
    }

    /// Fallback extraction for unsupported languages.
    fn extract_fallback(&self, content: &str, language: Language) -> Result<(Vec<Chunk>, Vec<Edge>)> {
        // For unsupported languages, treat entire file as one chunk
        let chunk = Chunk::new(content.to_string(), language, ChunkKind::Block, None);
        Ok((vec![chunk], Vec::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_rust_function() {
        let content = r#"
fn hello() {
    println!("Hello, world!");
}

fn goodbye() {
    println!("Goodbye!");
}
"#;
        let extractor = ChunkExtractor::new();
        let chunks = extractor.extract(content, Language::Rust).unwrap();
        
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].symbol_name, Some("hello".to_string()));
        assert_eq!(chunks[1].symbol_name, Some("goodbye".to_string()));
    }

    #[test]
    fn test_extract_rust_struct() {
        let content = r#"
pub struct User {
    name: String,
    age: u32,
}
"#;
        let extractor = ChunkExtractor::new();
        let chunks = extractor.extract(content, Language::Rust).unwrap();
        
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].kind, ChunkKind::Struct);
        assert_eq!(chunks[0].symbol_name, Some("User".to_string()));
    }

    #[test]
    fn test_extract_go_function() {
        let content = r#"
package main

func hello() {
    fmt.Println("Hello, world!")
}

func goodbye() {
    fmt.Println("Goodbye!")
}
"#;
        let extractor = ChunkExtractor::new();
        let chunks = extractor.extract(content, Language::Go).unwrap();
        
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].symbol_name, Some("hello".to_string()));
        assert_eq!(chunks[1].symbol_name, Some("goodbye".to_string()));
    }

    #[test]
    fn test_extract_terraform_resource() {
        let content = r#"
resource "aws_instance" "web" {
  ami           = "ami-12345678"
  instance_type = "t2.micro"
}

variable "region" {
  type    = string
  default = "us-west-2"
}

output "instance_ip" {
  value = aws_instance.web.public_ip
}
"#;
        let extractor = ChunkExtractor::new();
        let chunks = extractor.extract(content, Language::Hcl).unwrap();
        
        assert_eq!(chunks.len(), 3);
        
        // Check resource
        let resource = chunks.iter().find(|c| c.kind == ChunkKind::Resource);
        assert!(resource.is_some());
        assert_eq!(resource.unwrap().symbol_name, Some("aws_instance.web".to_string()));
        
        // Check variable
        let variable = chunks.iter().find(|c| c.kind == ChunkKind::Variable);
        assert!(variable.is_some());
        
        // Check output
        let output = chunks.iter().find(|c| c.kind == ChunkKind::Output);
        assert!(output.is_some());
    }

    #[test]
    fn test_language_detection() {
        assert_eq!(Language::from_extension("rs"), Language::Rust);
        assert_eq!(Language::from_extension("py"), Language::Python);
        assert_eq!(Language::from_extension("ts"), Language::TypeScript);
        assert_eq!(Language::from_extension("go"), Language::Go);
        assert_eq!(Language::from_extension("tf"), Language::Hcl);
        assert_eq!(Language::from_extension("tfvars"), Language::Hcl);
    }
}

