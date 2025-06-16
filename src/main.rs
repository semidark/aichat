//! Kindle AI Chat - Main binary entry point
//! 
//! This is a thin wrapper around the kindle_aichat library that handles
//! command-line argument parsing and delegates to the appropriate functionality.

use anyhow::Result;
use clap::Parser;

// Import the library
use aichat::{cli::Cli, run_cli, run_server};

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments
    let cli = Cli::parse();
    
    // Check if this is a CLI command (like --list-models) or server mode
    let is_cli_command = cli.list_models || cli.list_roles || cli.list_sessions || 
                        cli.list_agents || cli.list_rags || cli.list_macros ||
                        cli.info || cli.sync_models;
    
    if is_cli_command {
        // Run original CLI functionality
        run_cli(cli).await
    } else {
        // Run Rocket server
        run_server().await
    }
}
