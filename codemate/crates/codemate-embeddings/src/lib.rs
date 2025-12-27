//! CodeMate Embeddings Library
//!
//! Generate embeddings for code chunks using fastembed.

use codemate_core::storage::Embedding;
use codemate_core::Result;

/// Embedding generator using fastembed.
pub struct EmbeddingGenerator {
    model: fastembed::TextEmbedding,
    model_id: String,
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
            model,
            model_id: model_name.to_string(),
        })
    }

    /// Get the model ID.
    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    /// Generate embedding for a single text.
    pub fn embed(&mut self, text: &str) -> Result<Embedding> {
        let embeddings = self
            .model
            .embed(vec![text], None)
            .map_err(|e| codemate_core::Error::Embedding(e.to_string()))?;

        let vector = embeddings
            .into_iter()
            .next()
            .ok_or_else(|| codemate_core::Error::Embedding("No embedding generated".to_string()))?;

        Ok(Embedding::new(vector, self.model_id.clone()))
    }

    /// Generate embeddings for multiple texts.
    pub fn embed_batch(&mut self, texts: &[&str]) -> Result<Vec<Embedding>> {
        let embeddings = self
            .model
            .embed(texts.to_vec(), None)
            .map_err(|e| codemate_core::Error::Embedding(e.to_string()))?;

        Ok(embeddings
            .into_iter()
            .map(|vector| Embedding::new(vector, self.model_id.clone()))
            .collect())
    }
}
