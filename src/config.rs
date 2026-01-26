use std::env;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Missing required environment variable: {0}")]
    MissingEnvVar(String),
    #[error("Invalid MAC address format: {0}")]
    InvalidMac(String),
    #[error("Invalid port number: {0}")]
    InvalidPort(String),
}

fn validate_mac(mac_str: &str) -> Result<(), ConfigError> {
    let clean: String = mac_str.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if clean.len() != 12 {
        return Err(ConfigError::InvalidMac(mac_str.to_string()));
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_host: String,
    pub port: u16,
    pub homelab_mac: String,
    pub homelab_ip: String,
    pub homelab_broadcast: String,
    pub ssh_port: u16,
    pub ssh_key_path: PathBuf,
    pub pin_hash_path: PathBuf,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Config {
            bind_host: env::var("ARTICWAKE_BIND_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: env::var("ARTICWAKE_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .map_err(|_| ConfigError::InvalidPort("ARTICWAKE_PORT".to_string()))?,
            homelab_mac: {
                let mac = env::var("ARTICWAKE_HOMELAB_MAC")
                    .map_err(|_| ConfigError::MissingEnvVar("ARTICWAKE_HOMELAB_MAC".to_string()))?;
                validate_mac(&mac)?;
                mac
            },
            homelab_ip: env::var("ARTICWAKE_HOMELAB_IP")
                .map_err(|_| ConfigError::MissingEnvVar("ARTICWAKE_HOMELAB_IP".to_string()))?,
            homelab_broadcast: env::var("ARTICWAKE_HOMELAB_BROADCAST")
                .unwrap_or_else(|_| "255.255.255.255".to_string()),
            ssh_port: env::var("ARTICWAKE_SSH_PORT")
                .unwrap_or_else(|_| "2222".to_string())
                .parse()
                .map_err(|_| ConfigError::InvalidPort("ARTICWAKE_SSH_PORT".to_string()))?,
            ssh_key_path: PathBuf::from(
                env::var("ARTICWAKE_SSH_KEY_PATH")
                    .unwrap_or_else(|_| "/etc/secrets/articwake-key".to_string()),
            ),
            pin_hash_path: PathBuf::from(
                env::var("ARTICWAKE_PIN_HASH_PATH")
                    .unwrap_or_else(|_| "/var/lib/articwake/pin.hash".to_string()),
            ),
        })
    }
}
