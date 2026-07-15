// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

mod cli;
mod server;
mod pid;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{EnvFilter, fmt};

use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("anvil=info".parse()?))
        .with_target(false)
        .init();

    match cli.command {
        Commands::Start { project } => {
            server::run(project.as_deref()).await?;
        }
        Commands::Stop => {
            pid::send_stop()?;
        }
        Commands::Status => {
            pid::print_status();
        }
        Commands::Auth(auth_cmd) => {
            cli::handle_auth(auth_cmd)?;
        }
        Commands::Models => {
            cli::list_models().await?;
        }
    }

    Ok(())
}
