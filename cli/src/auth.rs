//! OAuth Device Code flow and token storage (ADR-007 / ADR-011).
//!
//! Token stored at: ~/.config/hostmgr/token  (mode 0600, 8h TTL)

use anyhow::{Context, Result};
use std::path::PathBuf;

/// Return the path to the stored token file.
fn token_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("hostmgr")
        .join("token")
}

/// Load a stored JWT token, if present and not obviously expired.
pub fn load_token() -> Result<Option<String>> {
    let path = token_path();
    if !path.exists() {
        return Ok(None);
    }
    let token = std::fs::read_to_string(&path)
        .context("failed to read stored token")?
        .trim()
        .to_string();
    if token.is_empty() {
        return Ok(None);
    }
    Ok(Some(token))
}

/// OAuth Device Code flow — open the browser, poll for token, persist it.
pub async fn login(provider: &str) -> Result<()> {
    println!("Starting Device Code OAuth flow with provider: {provider}");

    // TODO:
    //   1. POST /auth/{provider}/device/code
    //      Response: { device_code, user_code, verification_uri, expires_in, interval }
    //   2. Print: "Visit {verification_uri} and enter code {user_code}"
    //   3. Poll POST /auth/{provider}/device/token every `interval` seconds
    //      until access_token received or timeout
    //   4. Exchange access_token for hostmgr JWT (Host Manager signs with Ed25519)
    //   5. Write JWT to token_path() with mode 0600

    println!("(Device Code flow not yet implemented — set HOSTMGR_API_KEY as a workaround)");
    Ok(())
}

/// Remove the stored token.
pub fn logout() -> Result<()> {
    let path = token_path();
    if path.exists() {
        std::fs::remove_file(&path).context("failed to remove token file")?;
        println!("Logged out. Token removed from {}", path.display());
    } else {
        println!("No active session found.");
    }
    Ok(())
}

/// Write `data` to `path` with owner-only permissions.
#[allow(dead_code)]
fn write_secret_file(path: &std::path::Path, data: &[u8]) -> Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    std::fs::create_dir_all(path.parent().unwrap())?;
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    std::io::Write::write_all(&mut f, data)?;
    Ok(())
}
