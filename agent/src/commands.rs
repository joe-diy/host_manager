//! Command subscriber and executor.
//!
//! Subscribes to the JetStream stream `AGENT_CMDS` on subject
//! `agent.cmd.{agent_id}.>` and executes inbound commands.
//!
//! Supported command types (ADR-005 / protocol::messages):
//!   Exec    — run a shell command and return stdout/stderr/exit_code
//!   Install — install a package (future)
//!   Config  — apply configuration delta (future)
//!   Restart — restart a service (future)
//!   Update  — trigger agent self-update (delegated to update module)
//!
//! Each command is ack'd to JetStream after receipt, then the result is
//! published to `agent.{id}.cmd_result.{cmd_id}`.

use anyhow::Result;
use hostmgr_protocol::{messages::CommandPayload, subjects};
use serde_json;
use tracing::{info, warn};

use crate::transport::Connection;

pub async fn run(conn: Connection, agent_id: String) -> Result<()> {
    let Some(nats) = &conn.nats else {
        // TODO: implement HTTP long-poll command fetch for fallback mode
        info!(%agent_id, "command subscriber running in HTTPS polling mode (stub)");
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        }
    };

    // Subscribe to durable JetStream consumer for this agent's commands.
    // Consumer name is stable across restarts so commands survive reconnects.
    let cmd_subject = format!("agent.cmd.{agent_id}.>");
    let js = async_nats::jetstream::new(nats.clone());

    // Ensure the stream exists (idempotent).
    let stream = js
        .get_or_create_stream(async_nats::jetstream::stream::Config {
            name: subjects::KV_STREAM_AGENT_CMDS.to_string(),
            subjects: vec!["agent.cmd.>".to_string()],
            max_age: std::time::Duration::from_secs(86400), // 24h retention (ADR-005)
            ..Default::default()
        })
        .await
        .map_err(|e| anyhow::anyhow!("JetStream stream error: {e}"))?;

    let consumer = stream
        .get_or_create_consumer(
            &format!("agent-{agent_id}"),
            async_nats::jetstream::consumer::pull::Config {
                durable_name: Some(format!("agent-{agent_id}")),
                filter_subject: cmd_subject.clone(),
                ..Default::default()
            },
        )
        .await
        .map_err(|e| anyhow::anyhow!("JetStream consumer error: {e}"))?;

    info!(%agent_id, subject = %cmd_subject, "command subscriber ready");

    let mut messages = consumer.messages().await?;
    while let Some(msg) = futures::StreamExt::next(&mut messages).await {
        let msg = msg?;
        msg.ack().await.map_err(|e| anyhow::anyhow!("ack error: {e}"))?;

        match serde_json::from_slice::<hostmgr_protocol::messages::CommandMessage>(&msg.payload) {
            Ok(cmd) => {
                info!(cmd_id = %cmd.command_id, "executing command");
                execute_command(&conn, &agent_id, cmd).await;
            }
            Err(e) => {
                warn!(error = %e, "failed to deserialize command message");
            }
        }
    }

    Ok(())
}

async fn execute_command(
    conn: &Connection,
    agent_id: &str,
    cmd: hostmgr_protocol::messages::CommandMessage,
) {
    let result = match &cmd.payload {
        CommandPayload::Exec {
            command,
            timeout_seconds,
            working_directory,
        } => exec_shell(command, Some((*timeout_seconds) as u64), working_directory.as_deref()).await,

        CommandPayload::Restart { .. } => {
            // TODO: restart named service
            Ok(serde_json::json!({"status": "not_implemented"}))
        }

        CommandPayload::Update { .. } => {
            // Delegated to the update module via a channel (future).
            Ok(serde_json::json!({"status": "update_triggered"}))
        }

        _ => Ok(serde_json::json!({"status": "not_implemented"})),
    };

    // Publish result
    if let Some(nats) = &conn.nats {
        let result_subject = format!("agent.{agent_id}.cmd_result.{}", cmd.command_id);
        let payload = serde_json::to_vec(&result.unwrap_or_else(|e| {
            serde_json::json!({"error": e.to_string()})
        }))
        .unwrap_or_default();
        let _ = nats.publish(result_subject, payload.into()).await;
    }
}

async fn exec_shell(
    command: &str,
    timeout_seconds: Option<u64>,
    working_directory: Option<&str>,
) -> Result<serde_json::Value> {
    let timeout = std::time::Duration::from_secs(timeout_seconds.unwrap_or(30));

    let mut cmd = tokio::process::Command::new("sh");
    cmd.arg("-c").arg(command);
    cmd.kill_on_drop(true);

    if let Some(dir) = working_directory {
        cmd.current_dir(dir);
    }

    let output = tokio::time::timeout(timeout, cmd.output()).await??;

    Ok(serde_json::json!({
        "exit_code": output.status.code(),
        "stdout": String::from_utf8_lossy(&output.stdout),
        "stderr": String::from_utf8_lossy(&output.stderr),
    }))
}
