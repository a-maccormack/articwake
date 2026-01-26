use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

use russh::client;
use russh::keys::ssh_key;
use russh::keys::{load_secret_key, PrivateKeyWithHashAlg};

#[derive(Debug, Error)]
pub enum SshError {
    #[error("Failed to load SSH key: {0}")]
    KeyLoadFailed(String),
    #[error("SSH connection failed: {0}")]
    ConnectionFailed(String),
    #[error("SSH authentication failed")]
    AuthFailed,
    #[error("Failed to open channel: {0}")]
    ChannelFailed(String),
    #[error("Failed to send passphrase: {0}")]
    SendFailed(String),
}

struct ClientHandler;

impl client::Handler for ClientHandler {
    type Error = russh::Error;

    fn check_server_key(
        &mut self,
        _server_public_key: &ssh_key::PublicKey,
    ) -> impl std::future::Future<Output = Result<bool, Self::Error>> + Send {
        // Accept any host key (initrd dropbear regenerates keys)
        async { Ok(true) }
    }
}

pub async fn send_passphrase(
    host: &str,
    port: u16,
    key_path: &Path,
    passphrase: &str,
) -> Result<(), SshError> {
    let key = load_secret_key(key_path, None)
        .map_err(|e| SshError::KeyLoadFailed(e.to_string()))?;

    let key_with_hash = PrivateKeyWithHashAlg::new(Arc::new(key), None);

    let config = Arc::new(client::Config::default());
    let handler = ClientHandler;

    let addr = format!("{}:{}", host, port);
    let mut session = client::connect(config, &addr, handler)
        .await
        .map_err(|e| SshError::ConnectionFailed(e.to_string()))?;

    let username = "root";
    let auth_result = session
        .authenticate_publickey(username, key_with_hash)
        .await
        .map_err(|e| SshError::ConnectionFailed(e.to_string()))?;

    if !auth_result.success() {
        return Err(SshError::AuthFailed);
    }

    let channel = session
        .channel_open_session()
        .await
        .map_err(|e| SshError::ChannelFailed(e.to_string()))?;

    // Request a PTY - cryptsetup-askpass needs a terminal
    channel
        .request_pty(false, "xterm", 80, 24, 0, 0, &[])
        .await
        .map_err(|e| SshError::ChannelFailed(e.to_string()))?;

    // Request a shell - this triggers cryptsetup-askpass on the initrd
    channel
        .request_shell(false)
        .await
        .map_err(|e| SshError::ChannelFailed(e.to_string()))?;

    // Wait for the prompt to appear
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    // Send passphrase followed by newline to stdin
    channel
        .data(format!("{}\n", passphrase).as_bytes())
        .await
        .map_err(|e| SshError::SendFailed(e.to_string()))?;

    // Wait a bit for processing
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    channel.eof().await.ok();

    tracing::info!("Successfully sent passphrase via SSH to {}:{}", host, port);
    Ok(())
}
