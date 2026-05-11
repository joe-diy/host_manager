//! Host Manager CLI (`hostmgr`)
//!
//! Provides operator access to the Host Manager control plane from the terminal.
//!
//! Authentication (ADR-007 / ADR-011):
//!   - First run: `hostmgr login` — Device Code OAuth flow (GitHub or Google)
//!     Token stored at ~/.config/hostmgr/token (mode 0600), 8h TTL.
//!   - API key: `hostmgr --api-key hm_ro_… endpoints list`
//!
//! Usage:
//!   hostmgr endpoints list
//!   hostmgr endpoints get <id>
//!   hostmgr discovery start [--subnet 192.168.1.0/24]
//!   hostmgr commands exec <endpoint-id> -- <command>
//!   hostmgr login [--provider github|google]
//!   hostmgr logout
//!   hostmgr version

use anyhow::Result;
use clap::{Parser, Subcommand};

mod auth;
mod client;
pub mod commands;

use commands::{CommandsArgs, DiscoveryArgs, EndpointsArgs};

/// Host Manager — fleet management from the command line.
#[derive(Parser, Debug)]
#[command(
    name = "hostmgr",
    version,
    about = "Manage your fleet with Host Manager",
    long_about = None,
)]
struct Cli {
    /// Host Manager API URL (overrides HOSTMGR_URL env var).
    #[arg(long, env = "HOSTMGR_URL", global = true)]
    url: Option<String>,

    /// API key for non-interactive auth (hm_ro_… or hm_rw_…).
    #[arg(long, env = "HOSTMGR_API_KEY", global = true)]
    api_key: Option<String>,

    /// Output format.
    #[arg(long, global = true, default_value = "table", value_parser = ["table", "json", "yaml"])]
    output: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Manage endpoints in the fleet.
    Endpoints(EndpointsArgs),

    /// Manage and trigger discovery runs.
    Discovery(DiscoveryArgs),

    /// Dispatch commands to endpoints.
    Commands(CommandsArgs),

    /// Authenticate with the Host Manager control plane.
    Login {
        /// OAuth provider to use.
        #[arg(long, default_value = "github", value_parser = ["github", "google"])]
        provider: String,
    },

    /// Clear stored credentials.
    Logout,

    /// Print version information.
    Version,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    let base_url = cli
        .url
        .clone()
        .unwrap_or_else(|| "https://localhost:443".to_string());

    let api_client = client::ApiClient::new(base_url, cli.api_key.clone(), &cli.output)?;

    match cli.command {
        Commands::Endpoints(args) => {
            commands::endpoints::run(api_client, args).await?;
        }
        Commands::Discovery(args) => {
            commands::discovery::run(api_client, args).await?;
        }
        Commands::Commands(args) => {
            commands::exec::run(api_client, args).await?;
        }
        Commands::Login { provider } => {
            auth::login(&provider).await?;
        }
        Commands::Logout => {
            auth::logout()?;
        }
        Commands::Version => {
            println!("hostmgr {}", env!("CARGO_PKG_VERSION"));
        }
    }

    Ok(())
}
