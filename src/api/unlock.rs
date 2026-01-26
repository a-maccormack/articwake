use actix_web::{web, HttpRequest, HttpResponse};
use serde::Deserialize;

use crate::auth::AppState;
use crate::services::ssh::send_passphrase;

use super::require_auth;

const MAX_PASSPHRASE_LEN: usize = 1024;

#[derive(Deserialize)]
pub struct UnlockRequest {
    passphrase: String,
}

#[derive(Debug, PartialEq)]
pub enum PassphraseValidationError {
    Empty,
    TooLong,
    ContainsControlChars,
}

pub fn validate_passphrase(passphrase: &str) -> Result<(), PassphraseValidationError> {
    if passphrase.is_empty() {
        return Err(PassphraseValidationError::Empty);
    }
    if passphrase.len() > MAX_PASSPHRASE_LEN {
        return Err(PassphraseValidationError::TooLong);
    }
    if passphrase.chars().any(|c| c.is_control()) {
        return Err(PassphraseValidationError::ContainsControlChars);
    }
    Ok(())
}

pub async fn unlock(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<UnlockRequest>,
) -> HttpResponse {
    if let Err(resp) = require_auth(&req, &state) {
        return resp;
    }

    if let Err(e) = validate_passphrase(&body.passphrase) {
        let error_msg = match e {
            PassphraseValidationError::Empty => "Passphrase cannot be empty",
            PassphraseValidationError::TooLong => "Passphrase too long",
            PassphraseValidationError::ContainsControlChars => "Passphrase contains invalid characters",
        };
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": error_msg
        }));
    }

    match send_passphrase(
        &state.config.homelab_ip,
        state.config.ssh_port,
        &state.config.ssh_key_path,
        &body.passphrase,
    )
    .await
    {
        Ok(()) => HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": "Passphrase sent successfully"
        })),
        Err(e) => {
            tracing::error!("Unlock failed: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to unlock: {}", e)
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_passphrase_valid() {
        assert!(validate_passphrase("my-secure-passphrase").is_ok());
        assert!(validate_passphrase("a").is_ok());
        assert!(validate_passphrase("with spaces").is_ok());
        assert!(validate_passphrase("with-special-chars!@#$%^&*()").is_ok());
        assert!(validate_passphrase("unicode: äöü").is_ok());
    }

    #[test]
    fn test_validate_passphrase_empty() {
        assert_eq!(
            validate_passphrase(""),
            Err(PassphraseValidationError::Empty)
        );
    }

    #[test]
    fn test_validate_passphrase_too_long() {
        let long_passphrase = "a".repeat(MAX_PASSPHRASE_LEN + 1);
        assert_eq!(
            validate_passphrase(&long_passphrase),
            Err(PassphraseValidationError::TooLong)
        );
    }

    #[test]
    fn test_validate_passphrase_max_length_ok() {
        let max_passphrase = "a".repeat(MAX_PASSPHRASE_LEN);
        assert!(validate_passphrase(&max_passphrase).is_ok());
    }

    #[test]
    fn test_validate_passphrase_control_chars() {
        // Null byte
        assert_eq!(
            validate_passphrase("pass\0word"),
            Err(PassphraseValidationError::ContainsControlChars)
        );
        // Newline
        assert_eq!(
            validate_passphrase("pass\nword"),
            Err(PassphraseValidationError::ContainsControlChars)
        );
        // Carriage return
        assert_eq!(
            validate_passphrase("pass\rword"),
            Err(PassphraseValidationError::ContainsControlChars)
        );
        // Tab
        assert_eq!(
            validate_passphrase("pass\tword"),
            Err(PassphraseValidationError::ContainsControlChars)
        );
        // Bell
        assert_eq!(
            validate_passphrase("pass\x07word"),
            Err(PassphraseValidationError::ContainsControlChars)
        );
        // Escape
        assert_eq!(
            validate_passphrase("pass\x1bword"),
            Err(PassphraseValidationError::ContainsControlChars)
        );
    }

    #[test]
    fn test_validate_passphrase_shell_injection_attempts() {
        // These should be valid passphrases (no control chars)
        // The validation only blocks control chars, not shell metacharacters
        // since cryptsetup-askpass uses `read` not `eval`
        assert!(validate_passphrase("; rm -rf /").is_ok());
        assert!(validate_passphrase("$(whoami)").is_ok());
        assert!(validate_passphrase("`id`").is_ok());
        assert!(validate_passphrase("| cat /etc/passwd").is_ok());
    }
}
