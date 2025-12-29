use std::sync::Arc;
use axum::{Json, Extension, http::StatusCode};
use codemate_core::service::{CodeMateService, SearchOptions};
use crate::models::{
    IndexRequest, IndexResponse, ModuleGraphRequest, ModuleGraphResponse, SearchRequest, SearchResponse, TreeRequest,
    TreeResponse,
};

pub struct AppState {
    pub service: Arc<dyn CodeMateService>,
}

pub type SharedState = Arc<AppState>;

pub async fn search(
    Extension(state): Extension<SharedState>,
    Json(req): Json<SearchRequest>,
) -> Result<Json<SearchResponse>, (StatusCode, String)> {
    let options = SearchOptions {
        limit: req.limit.unwrap_or(5),
        threshold: req.threshold.unwrap_or(0.3),
    };
    
    let results = state.service.search(&req.query, options).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    Ok(Json(SearchResponse { results }))
}

pub async fn tree(
    Extension(state): Extension<SharedState>,
    Json(req): Json<TreeRequest>,
) -> Result<Json<TreeResponse>, (StatusCode, String)> {
    let all = req.all.unwrap_or(false);
    let depth = req.depth.unwrap_or(3);
    
    let symbol = if all { None } else { req.symbol.as_deref() };
    
    let tree = state.service.get_tree(symbol, depth).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(TreeResponse { tree }))
}

pub async fn module_graph(
    Extension(state): Extension<SharedState>,
    Json(req): Json<ModuleGraphRequest>,
) -> Result<Json<ModuleGraphResponse>, (StatusCode, String)> {
    let show_edges = req.show_edges.unwrap_or(false);
    
    let modules = state.service.get_module_graph(req.level, req.filters, show_edges).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ModuleGraphResponse { modules }))
}

pub async fn health() -> StatusCode {
    StatusCode::OK
}

pub async fn index(
    Extension(state): Extension<SharedState>,
    Json(req): Json<IndexRequest>,
) -> Result<Json<IndexResponse>, (StatusCode, String)> {
    let path = std::path::Path::new(&req.path);
    if !path.exists() {
        return Err((StatusCode::BAD_REQUEST, format!("Path does not exist: {}", req.path)));
    }

    let git_mode = req.git.unwrap_or(false);
    
    state.service.index(path, git_mode).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(IndexResponse {
        files: 0,
        chunks: 0,
        message: "Indexing started in background".to_string(),
    }))
}
