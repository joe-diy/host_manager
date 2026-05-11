//! Credentials capability provider — OpenBao adapter.
//!
//! Wraps the OpenBao (Vault-compatible) KV v2 secret engine so that actors
//! never interact with OpenBao directly. All reads and writes are brokered
//! through the credential-manager actor, which calls this provider.
//!
//! Protocol (request-reply over NATS):
//!   Subject:  `credentials.backend.read`
//!   Request:  `{ "path": "secret/endpoints/ep-xxx/core" }`
//!   Response: `{ "data": { … } }` or `{ "error": "…" }`
//!
//!   Subject:  `credentials.backend.write`
//!   Request:  `{ "path": "…", "data": { … } }`
//!   Response: `{ "ok": true }` or `{ "error": "…" }`
//!
//! Config (env vars read at startup):
//!   HOSTMGR_OPENBAO_URL   — OpenBao address, e.g. http://127.0.0.1:8200
//!   HOSTMGR_OPENBAO_TOKEN_REF — OpenBao path in itself that holds the provider token

use anyhow::Result;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "info,hostmgr_provider_credentials=debug".into()),
        )
        .init();

    let openbao_url = std::env::var("HOSTMGR_OPENBAO_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8200".to_string());

    info!(url = %openbao_url, "credentials provider starting");

    // TODO: build vaultrs client pointed at openbao_url
    // TODO: authenticate using HOSTMGR_OPENBAO_TOKEN_REF or VAULT_TOKEN env var
    // TODO: subscribe to "credentials.backend.read" and "credentials.backend.write"
    // TODO: proxy requests to OpenBao KV v2

    tokio::signal::ctrl_c().await?;
    info!("credentials provider shutting down");
    Ok(())
}
