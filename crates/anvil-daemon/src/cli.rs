// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use anyhow::Result;
use clap::{Parser, Subcommand};

use anvil_config::ConfigLoader;
use anvil_inference::registry::BackendRegistry;
use anvil_providers::keystore;

#[derive(Parser)]
#[command(name = "anvil", about = "TPT Anvil — local AI development environment", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start the Anvil daemon
    Start {
        /// Project root directory to index
        #[arg(short, long)]
        project: Option<String>,
    },
    /// Stop the running daemon
    Stop,
    /// Show daemon status
    Status,
    /// Manage API keys
    Auth(AuthArgs),
    /// List available models
    Models,
}

#[derive(Parser)]
pub struct AuthArgs {
    #[command(subcommand)]
    pub command: AuthCommands,
}

#[derive(Subcommand)]
pub enum AuthCommands {
    /// Store an API key in the OS keychain
    Set {
        /// Key name (e.g. openai_api_key)
        name: String,
        /// API key value
        key: String,
    },
    /// Remove an API key from the OS keychain
    Remove {
        name: String,
    },
}

pub fn handle_auth(args: AuthArgs) -> Result<()> {
    match args.command {
        AuthCommands::Set { name, key } => {
            keystore::set_api_key(&name, &key)?;
            println!("API key '{name}' stored in OS keychain.");
        }
        AuthCommands::Remove { name } => {
            keystore::delete_api_key(&name)?;
            println!("API key '{name}' removed.");
        }
    }
    Ok(())
}

pub async fn list_models() -> Result<()> {
    let cfg = ConfigLoader::load(None)?;
    let registry = BackendRegistry::from_config(&cfg)?;
    let models = registry.active.list_models().await?;
    if models.is_empty() {
        println!("No models found. Make sure Ollama is running or a model path is configured.");
    } else {
        for model in models {
            println!("  {} — {} (context: {} tokens)", model.id, model.name, model.context_length);
        }
    }
    Ok(())
}
