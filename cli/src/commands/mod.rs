pub mod discovery;
pub mod endpoints;
pub mod exec;

use clap::{Args, Subcommand};

// ── endpoints ────────────────────────────────────────────────────────────────

#[derive(Args, Debug)]
pub struct EndpointsArgs {
    #[command(subcommand)]
    pub command: EndpointsCommand,
}

#[derive(Subcommand, Debug)]
pub enum EndpointsCommand {
    /// List all endpoints.
    List {
        /// Filter by status (discovered|identified|managed|offline|…).
        #[arg(long)]
        status: Option<String>,
    },
    /// Get details for a single endpoint.
    Get {
        /// Endpoint ID (ep-…) or hostname.
        id: String,
    },
}

// ── discovery ────────────────────────────────────────────────────────────────

#[derive(Args, Debug)]
pub struct DiscoveryArgs {
    #[command(subcommand)]
    pub command: DiscoveryCommand,
}

#[derive(Subcommand, Debug)]
pub enum DiscoveryCommand {
    /// Start a discovery run.
    Start {
        /// CIDR subnet to scan (e.g. 192.168.1.0/24). Defaults to control plane config.
        #[arg(long)]
        subnet: Option<String>,
    },
    /// Show the result of the most recent discovery run.
    Status,
}

// ── commands (exec) ──────────────────────────────────────────────────────────

#[derive(Args, Debug)]
pub struct CommandsArgs {
    #[command(subcommand)]
    pub command: CommandsCommand,
}

#[derive(Subcommand, Debug)]
pub enum CommandsCommand {
    /// Execute a shell command on one or more endpoints.
    Exec {
        /// Target endpoint ID(s), comma-separated, or "all".
        #[arg(long)]
        target: String,

        /// Shell command to run.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,

        /// Timeout in seconds.
        #[arg(long, default_value = "30")]
        timeout: u64,
    },
}
