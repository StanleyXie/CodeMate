use std::sync::Arc;
use tokio::sync::Mutex;
use async_trait::async_trait;
use mcp_rust_sdk::server::{Server, ServerHandler};
use mcp_rust_sdk::transport::stdio::StdioTransport;
use mcp_rust_sdk::transport::Transport;
use mcp_rust_sdk::types::{
    Implementation, ServerCapabilities, ClientCapabilities, Tool,
};
use mcp_rust_sdk::error::{Error, ErrorCode};
use codemate_core::service::{CodeMateService, SearchOptions};
use serde_json::{json, Value};
use anyhow::Result;

pub struct McpHandler {
    service: Arc<dyn CodeMateService>,
}

impl McpHandler {
    pub fn new(service: Arc<dyn CodeMateService>) -> Self {
        Self { service }
    }

    pub async fn start_stdio(self) -> Result<()> {
        let (transport, _sender) = StdioTransport::new();
        let transport = Arc::new(transport) as Arc<dyn Transport>;
        let handler = Arc::new(self) as Arc<dyn ServerHandler>;
        let server = Server::new(transport, handler);
        server.start().await.map_err(|e| anyhow::anyhow!(e))?;
        Ok(())
    }
}

#[async_trait]
impl ServerHandler for McpHandler {
    async fn initialize(
        &self,
        _implementation: Implementation,
        _capabilities: ClientCapabilities,
    ) -> std::result::Result<ServerCapabilities, Error> {
        let mut caps = ServerCapabilities::default();
        let mut custom = std::collections::HashMap::new();
        custom.insert("tools".to_string(), json!({
            "listChanged": false
        }));
        caps.custom = Some(custom);
        Ok(caps)
    }

    async fn shutdown(&self) -> std::result::Result<(), Error> {
        Ok(())
    }

    async fn handle_method(&self, method: &str, params: Option<Value>) -> std::result::Result<Value, Error> {
        match method {
            "tools/list" => {
                let tools = vec![
                    Tool {
                        name: "code_search".to_string(),
                        description: "Search for code context using hybrid search.".to_string(),
                        schema: json!({
                            "type": "object",
                            "properties": {
                                "query": { "type": "string", "description": "Search query" },
                                "limit": { "type": "number", "description": "Max results" },
                                "threshold": { "type": "number", "description": "Similarity threshold" }
                            },
                            "required": ["query"]
                        }),
                    },
                    Tool {
                        name: "get_dependency_tree".to_string(),
                        description: "Get a dependency tree for a symbol.".to_string(),
                        schema: json!({
                            "type": "object",
                            "properties": {
                                "symbol": { "type": "string", "description": "Target symbol" },
                                "depth": { "type": "number", "description": "Max depth" }
                            },
                            "required": ["symbol"]
                        }),
                    },
                    Tool {
                        name: "get_file_context".to_string(),
                        description: "Get code content for a file or symbol.".to_string(),
                        schema: json!({
                            "type": "object",
                            "properties": {
                                "symbol": { "type": "string", "description": "Target symbol" },
                                "path": { "type": "string", "description": "File path" }
                            }
                        }),
                    },
                    Tool {
                        name: "get_related_symbols".to_string(),
                        description: "Find related symbols using graph neighbors and vector similarity.".to_string(),
                        schema: json!({
                            "type": "object",
                            "properties": {
                                "symbol": { "type": "string", "description": "Target symbol" },
                                "limit": { "type": "number", "description": "Max results for similarity search" }
                            },
                            "required": ["symbol"]
                        }),
                    },
                    Tool {
                        name: "get_module_graph".to_string(),
                        description: "Get the module-level dependency graph of the project.".to_string(),
                        schema: json!({
                            "type": "object",
                            "properties": {
                                "level": { "type": "string", "description": "Abstraction level: crate|module" },
                                "filters": { "type": "array", "items": { "type": "string" }, "description": "Filter by specific module IDs" },
                                "show_edges": { "type": "boolean", "description": "Show specific symbol-level links" }
                            }
                        }),
                    },
                ];
                Ok(json!({ "tools": tools }))
            }
            "tools/call" => {
                let p = params.ok_or_else(|| Error::protocol(ErrorCode::InvalidParams, "Missing params"))?;
                let name = p["name"].as_str().ok_or_else(|| Error::protocol(ErrorCode::InvalidParams, "Missing tool name"))?;
                let args = &p["arguments"];

                match name {
                    "code_search" => {
                        let query_str = args["query"].as_str().unwrap_or("");
                        let options = SearchOptions {
                            limit: args["limit"].as_u64().unwrap_or(5) as usize,
                            threshold: args["threshold"].as_f64().unwrap_or(0.3) as f32,
                        };

                        let results = self.service.search(query_str, options).await
                            .map_err(|e| Error::protocol(ErrorCode::InternalError, e.to_string()))?;
                        
                        Ok(json!({ "content": [ { "type": "text", "text": format!("{:?}", results) } ] }))
                    }
                    "get_dependency_tree" => {
                        let symbol = args["symbol"].as_str().unwrap_or("");
                        let depth = args["depth"].as_u64().unwrap_or(3) as usize;

                        let tree = self.service.get_tree(Some(symbol), depth).await
                            .map_err(|e| Error::protocol(ErrorCode::InternalError, e.to_string()))?;
                        
                        Ok(json!({ "content": [ { "type": "text", "text": tree } ] }))
                    }
                    "get_file_context" => {
                        let symbol = args["symbol"].as_str().ok_or_else(|| Error::protocol(ErrorCode::InvalidParams, "Missing symbol"))?;
                        let chunks = self.service.get_context(symbol).await
                            .map_err(|e| Error::protocol(ErrorCode::InternalError, e.to_string()))?;
                        Ok(json!({ "content": [ { "type": "text", "text": format!("{:?}", chunks) } ] }))
                    }
                    "get_related_symbols" => {
                        let symbol = args["symbol"].as_str().ok_or_else(|| Error::protocol(ErrorCode::InvalidParams, "Missing symbol"))?;
                        let limit = args["limit"].as_u64().unwrap_or(5) as usize;

                        let related = self.service.get_related(symbol, limit).await
                            .map_err(|e| Error::protocol(ErrorCode::InternalError, e.to_string()))?;

                        Ok(json!({
                            "content": [
                                {
                                    "type": "text",
                                    "text": format!("Graph Neighbors: {:?}\nSemantically Similar: {:?}", related.graph_neighbors, related.semantic_relatives)
                                }
                            ]
                        }))
                    }
                    "get_module_graph" => {
                        let level = args["level"].as_str().map(|s| s.to_string());
                        let filters = args["filters"].as_array().map(|arr| {
                            arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
                        });
                        let show_edges = args["show_edges"].as_bool().unwrap_or(false);

                        let graph = self.service.get_module_graph(level, filters, show_edges).await
                            .map_err(|e| Error::protocol(ErrorCode::InternalError, e.to_string()))?;
                        
                        Ok(json!({
                            "content": [
                                {
                                    "type": "text",
                                    "text": format!("{:?}", graph)
                                }
                            ]
                        }))
                    }
                    _ => Err(Error::protocol(ErrorCode::MethodNotFound, format!("Tool not found: {}", name))),
                }
            }
            _ => Err(Error::protocol(ErrorCode::MethodNotFound, format!("Method not found: {}", method))),
        }
    }
}
