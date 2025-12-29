use clap::Parser;
use std::path::PathBuf;
use anyhow::Result;

#[derive(Parser)]
#[command(name = "codemate-server")]
#[command(author, version, about = "CodeMate HTTP API Server", long_about = None)]
struct Cli {
    /// Database path
    #[arg(short, long, default_value = ".codemate/index.db")]
    database: PathBuf,

    /// Port to listen on
    #[arg(short, long, default_value = "8080")]
    port: u16,

    /// Verbose output
    #[arg(short, long, default_value = "false")]
    verbose: bool,

    /// Start in MCP mode (stdio)
    #[arg(short, long, default_value = "false")]
    mcp: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let filter = if cli.verbose {
        "codemate_server=debug,codemate=debug"
    } else {
        "codemate_server=info,codemate=info"
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();

    if cli.mcp {
        use std::sync::Arc;
        use codemate_core::storage::SqliteStorage;
        use codemate_core::service::CodeMateService;
        use codemate_server::service::DefaultCodeMateService;
        use codemate_embeddings::EmbeddingGenerator;
        use codemate_server::mcp::McpHandler;

        let storage = Arc::new(SqliteStorage::new(&cli.database)?);
        let embedder = Arc::new(EmbeddingGenerator::new()?);
        let service = Arc::new(DefaultCodeMateService::new(storage, embedder)) as Arc<dyn CodeMateService>;
        
        let handler = McpHandler::new(service);
        handler.start_stdio().await?;
    } else {
        codemate_server::start(cli.database, cli.port).await?;
    }

    Ok(())
}
