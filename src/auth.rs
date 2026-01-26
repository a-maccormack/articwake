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
