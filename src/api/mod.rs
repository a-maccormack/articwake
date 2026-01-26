pub mod auth;
pub mod status;
pub mod unlock;
pub mod wol;

use actix_web::{HttpRequest, HttpResponse, http::header};
use crate::auth::{AppState, extract_bearer_token};

pub fn require_auth(req: &HttpRequest, state: &AppState) -> Result<(), HttpResponse> {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let token = extract_bearer_token(auth_header).ok_or_else(|| {
        HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "Missing or invalid Authorization header"
        }))
    })?;

    state.validate_token(token).map_err(|_| {
        HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "Invalid or expired token"
        }))
    })
}
