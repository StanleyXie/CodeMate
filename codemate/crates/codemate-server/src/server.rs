use axum::{
    routing::{get, post},
    Router, Extension,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use anyhow::Result;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use crate::handlers::{AppState, index, search, tree, health};
use codemate_core::storage::SqliteStorage;
use codemate_core::service::CodeMateService;
use crate::service::DefaultCodeMateService;
use codemate_embeddings::EmbeddingGenerator;

pub async fn start(db_path: std::path::PathBuf, port: u16) -> Result<()> {
    // Initialize shared state
    let storage = Arc::new(SqliteStorage::new(&db_path)?);
    let embedder = Arc::new(EmbeddingGenerator::new()?);
    let service = Arc::new(DefaultCodeMateService::new(storage, embedder)) as Arc<dyn CodeMateService>;
    
    let state = Arc::new(AppState {
        service,
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/v1/index", post(index))
        .route("/api/v1/search", post(search))
        .route("/api/v1/graph/tree", post(tree))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .layer(Extension(state));

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("CodeMate server listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// We will implement the actual handlers in handlers.rs, but for now we need these function headers to compile server.rs
// Wait, I'll just put them in handlers.rs now.
