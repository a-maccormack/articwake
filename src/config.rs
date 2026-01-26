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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_mac_colon_separated() {
        assert!(validate_mac("aa:bb:cc:dd:ee:ff").is_ok());
        assert!(validate_mac("AA:BB:CC:DD:EE:FF").is_ok());
        assert!(validate_mac("00:11:22:33:44:55").is_ok());
    }

    #[test]
    fn test_validate_mac_dash_separated() {
        assert!(validate_mac("aa-bb-cc-dd-ee-ff").is_ok());
        assert!(validate_mac("AA-BB-CC-DD-EE-FF").is_ok());
    }

    #[test]
    fn test_validate_mac_no_separator() {
        assert!(validate_mac("aabbccddeeff").is_ok());
        assert!(validate_mac("AABBCCDDEEFF").is_ok());
    }

    #[test]
    fn test_validate_mac_invalid_too_short() {
        assert!(validate_mac("aabbccddee").is_err());
        assert!(validate_mac("aa:bb:cc:dd:ee").is_err());
    }

    #[test]
    fn test_validate_mac_invalid_too_long() {
        assert!(validate_mac("aabbccddeeff00").is_err());
        assert!(validate_mac("aa:bb:cc:dd:ee:ff:00").is_err());
    }

    #[test]
    fn test_validate_mac_invalid_chars() {
        assert!(validate_mac("gg:hh:ii:jj:kk:ll").is_err());
        assert!(validate_mac("not-a-mac-addr").is_err());
    }

    #[test]
    fn test_validate_mac_empty() {
        assert!(validate_mac("").is_err());
    }
}
