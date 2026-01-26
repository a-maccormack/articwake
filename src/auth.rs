use std::collections::HashMap;
use std::fs;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use argon2::{Argon2, PasswordHash, PasswordVerifier};
use rand::Rng;
use thiserror::Error;

use crate::config::Config;

const TOKEN_EXPIRY: Duration = Duration::from_secs(15 * 60); // 15 minutes
const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);
const MAX_ATTEMPTS_PER_WINDOW: usize = 10;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Invalid PIN")]
    InvalidPin,
    #[error("Rate limited")]
    RateLimited,
    #[error("Invalid or expired token")]
    InvalidToken,
    #[error("Failed to read PIN hash: {0}")]
    PinHashReadFailed(String),
    #[error("Invalid PIN hash format: {0}")]
    InvalidPinHash(String),
}

struct Session {
    expires_at: Instant,
}

struct RateLimitEntry {
    attempts: Vec<Instant>,
}

pub struct AppState {
    pub config: Config,
    sessions: Mutex<HashMap<String, Session>>,
    rate_limits: Mutex<HashMap<IpAddr, RateLimitEntry>>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        AppState {
            config,
            sessions: Mutex::new(HashMap::new()),
            rate_limits: Mutex::new(HashMap::new()),
        }
    }

    pub fn check_rate_limit(&self, ip: IpAddr) -> Result<(), AuthError> {
        let mut limits = self.rate_limits.lock().unwrap();
        let now = Instant::now();

        let entry = limits.entry(ip).or_insert(RateLimitEntry {
            attempts: Vec::new(),
        });

        // Remove old attempts outside the window
        entry.attempts.retain(|t| now.duration_since(*t) < RATE_LIMIT_WINDOW);

        if entry.attempts.len() >= MAX_ATTEMPTS_PER_WINDOW {
            return Err(AuthError::RateLimited);
        }

        entry.attempts.push(now);
        Ok(())
    }

    pub fn verify_pin(&self, pin: &str) -> Result<String, AuthError> {
        let hash_content = fs::read_to_string(&self.config.pin_hash_path)
            .map_err(|e| AuthError::PinHashReadFailed(e.to_string()))?;

        let hash = PasswordHash::new(hash_content.trim())
            .map_err(|e| AuthError::InvalidPinHash(e.to_string()))?;

        Argon2::default()
            .verify_password(pin.as_bytes(), &hash)
            .map_err(|_| AuthError::InvalidPin)?;

        // Generate session token
        let token = generate_token();
        let mut sessions = self.sessions.lock().unwrap();

        // Clean expired sessions
        let now = Instant::now();
        sessions.retain(|_, s| s.expires_at > now);

        sessions.insert(
            token.clone(),
            Session {
                expires_at: now + TOKEN_EXPIRY,
            },
        );

        Ok(token)
    }

    pub fn validate_token(&self, token: &str) -> Result<(), AuthError> {
        let sessions = self.sessions.lock().unwrap();
        let now = Instant::now();

        match sessions.get(token) {
            Some(session) if session.expires_at > now => Ok(()),
            _ => Err(AuthError::InvalidToken),
        }
    }
}

fn generate_token() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.r#gen();
    hex::encode(bytes)
}

pub fn extract_bearer_token(auth_header: Option<&str>) -> Option<&str> {
    auth_header.and_then(|h| h.strip_prefix("Bearer "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::Ipv4Addr;
    use tempfile::NamedTempFile;

    fn create_test_config(pin_hash_path: std::path::PathBuf) -> Config {
        Config {
            bind_host: "127.0.0.1".to_string(),
            port: 8080,
            homelab_mac: "aa:bb:cc:dd:ee:ff".to_string(),
            homelab_ip: "192.168.1.100".to_string(),
            homelab_broadcast: "255.255.255.255".to_string(),
            ssh_port: 2222,
            ssh_key_path: std::path::PathBuf::from("/tmp/test-key"),
            pin_hash_path,
        }
    }

    fn create_pin_hash(pin: &str) -> NamedTempFile {
        use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(pin.as_bytes(), &salt)
            .expect("Failed to hash PIN");

        let mut file = NamedTempFile::new().expect("Failed to create temp file");
        writeln!(file, "{}", hash).expect("Failed to write hash");
        file
    }

    #[test]
    fn test_extract_bearer_token_valid() {
        assert_eq!(
            extract_bearer_token(Some("Bearer abc123")),
            Some("abc123")
        );
        assert_eq!(
            extract_bearer_token(Some("Bearer my-long-token-here")),
            Some("my-long-token-here")
        );
    }

    #[test]
    fn test_extract_bearer_token_invalid() {
        assert_eq!(extract_bearer_token(Some("Basic abc123")), None);
        assert_eq!(extract_bearer_token(Some("bearer abc123")), None);
        assert_eq!(extract_bearer_token(Some("abc123")), None);
        assert_eq!(extract_bearer_token(Some("")), None);
        assert_eq!(extract_bearer_token(None), None);
    }

    #[test]
    fn test_generate_token_length() {
        let token = generate_token();
        assert_eq!(token.len(), 64); // 32 bytes = 64 hex chars
    }

    #[test]
    fn test_generate_token_unique() {
        let token1 = generate_token();
        let token2 = generate_token();
        assert_ne!(token1, token2);
    }

    #[test]
    fn test_rate_limit_allows_under_limit() {
        let config = create_test_config(std::path::PathBuf::from("/tmp/nonexistent"));
        let state = AppState::new(config);
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        for _ in 0..MAX_ATTEMPTS_PER_WINDOW {
            assert!(state.check_rate_limit(ip).is_ok());
        }
    }

    #[test]
    fn test_rate_limit_blocks_over_limit() {
        let config = create_test_config(std::path::PathBuf::from("/tmp/nonexistent"));
        let state = AppState::new(config);
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2));

        for _ in 0..MAX_ATTEMPTS_PER_WINDOW {
            assert!(state.check_rate_limit(ip).is_ok());
        }

        // 11th attempt should be blocked
        assert!(matches!(
            state.check_rate_limit(ip),
            Err(AuthError::RateLimited)
        ));
    }

    #[test]
    fn test_rate_limit_different_ips() {
        let config = create_test_config(std::path::PathBuf::from("/tmp/nonexistent"));
        let state = AppState::new(config);
        let ip1 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 3));
        let ip2 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 4));

        // Max out ip1
        for _ in 0..MAX_ATTEMPTS_PER_WINDOW {
            assert!(state.check_rate_limit(ip1).is_ok());
        }
        assert!(state.check_rate_limit(ip1).is_err());

        // ip2 should still work
        assert!(state.check_rate_limit(ip2).is_ok());
    }

    #[test]
    fn test_verify_pin_correct() {
        let hash_file = create_pin_hash("1234");
        let config = create_test_config(hash_file.path().to_path_buf());
        let state = AppState::new(config);

        let result = state.verify_pin("1234");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 64); // Token should be 64 hex chars
    }

    #[test]
    fn test_verify_pin_incorrect() {
        let hash_file = create_pin_hash("1234");
        let config = create_test_config(hash_file.path().to_path_buf());
        let state = AppState::new(config);

        let result = state.verify_pin("wrong");
        assert!(matches!(result, Err(AuthError::InvalidPin)));
    }

    #[test]
    fn test_verify_pin_missing_file() {
        let config = create_test_config(std::path::PathBuf::from("/nonexistent/path"));
        let state = AppState::new(config);

        let result = state.verify_pin("1234");
        assert!(matches!(result, Err(AuthError::PinHashReadFailed(_))));
    }

    #[test]
    fn test_validate_token_valid() {
        let hash_file = create_pin_hash("1234");
        let config = create_test_config(hash_file.path().to_path_buf());
        let state = AppState::new(config);

        let token = state.verify_pin("1234").unwrap();
        assert!(state.validate_token(&token).is_ok());
    }

    #[test]
    fn test_validate_token_invalid() {
        let config = create_test_config(std::path::PathBuf::from("/tmp/nonexistent"));
        let state = AppState::new(config);

        assert!(matches!(
            state.validate_token("invalid-token"),
            Err(AuthError::InvalidToken)
        ));
    }

    #[test]
    fn test_validate_token_empty() {
        let config = create_test_config(std::path::PathBuf::from("/tmp/nonexistent"));
        let state = AppState::new(config);

        assert!(matches!(
            state.validate_token(""),
            Err(AuthError::InvalidToken)
        ));
    }
}
