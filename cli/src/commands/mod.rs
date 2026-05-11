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
