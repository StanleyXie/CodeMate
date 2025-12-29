//! CodeMate Embeddings Library
//!
//! Generate embeddings for code chunks using fastembed.

use std::sync::Mutex;
use codemate_core::storage::{Embedding, Embedder};
use codemate_core::Result;

/// Embedding generator using fastembed.
pub struct EmbeddingGenerator {
    model: Mutex<fastembed::TextEmbedding>,
    model_id: String,
}

impl Embedder for EmbeddingGenerator {
    fn embed(&self, text: &str) -> Result<Embedding> {
        let mut model = self.model.lock().map_err(|e| codemate_core::Error::Embedding(e.to_string()))?;
        let embeddings = model
            .embed(vec![text], None)
            .map_err(|e| codemate_core::Error::Embedding(e.to_string()))?;

        let vector = embeddings
            .into_iter()
            .next()
            .ok_or_else(|| codemate_core::Error::Embedding("No embedding generated".to_string()))?;

        Ok(Embedding::new(vector, self.model_id.clone()))
    }

    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Embedding>> {
        let mut model = self.model.lock().map_err(|e| codemate_core::Error::Embedding(e.to_string()))?;
        let embeddings = model
            .embed(texts.to_vec(), None)
            .map_err(|e| codemate_core::Error::Embedding(e.to_string()))?;

        Ok(embeddings
            .into_iter()
            .map(|vector| Embedding::new(vector, self.model_id.clone()))
            .collect())
    }
}

impl EmbeddingGenerator {
    /// Create a new embedding generator with the default model.
    pub fn new() -> Result<Self> {
        Self::with_model("sentence-transformers/all-MiniLM-L6-v2")
    }

    /// Create a new embedding generator with a specific model.
    pub fn with_model(model_name: &str) -> Result<Self> {
        let model = fastembed::TextEmbedding::try_new(
            fastembed::InitOptions::new(fastembed::EmbeddingModel::AllMiniLML6V2)
                .with_show_download_progress(true),
        )
        .map_err(|e| codemate_core::Error::Embedding(e.to_string()))?;

        Ok(Self {
            model: Mutex::new(model),
            model_id: model_name.to_string(),
        })
    }

    /// Get the model ID.
    pub fn model_id(&self) -> &str {
        &self.model_id
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialization() {
        let mut generator = EmbeddingGenerator::new().expect("Failed to create generator");
        let embedding = generator.embed("Hello world").expect("Failed to embed text");
        assert_eq!(embedding.dimensions, 384); // all-MiniLM-L6-v2 dimensions
    }
}
