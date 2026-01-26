use actix_web::{HttpRequest, HttpResponse, web};
use serde::Deserialize;

use crate::auth::{AppState, AuthError};

#[derive(Deserialize)]
pub struct AuthRequest {
    pin: String,
}

pub async fn authenticate(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<AuthRequest>,
) -> HttpResponse {
    // Extract client IP for rate limiting
    let ip = req
        .peer_addr()
        .map(|a| a.ip())
        .unwrap_or_else(|| std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST));

    // Check rate limit
    if let Err(AuthError::RateLimited) = state.check_rate_limit(ip) {
        tracing::warn!("Rate limited auth attempt from {}", ip);
        return HttpResponse::TooManyRequests().json(serde_json::json!({
            "error": "Too many authentication attempts. Please wait."
        }));
    }

    // Verify PIN
    match state.verify_pin(&body.pin) {
        Ok(token) => {
            tracing::info!("Successful authentication from {}", ip);
            HttpResponse::Ok().json(serde_json::json!({
                "token": token
            }))
        }
        Err(AuthError::InvalidPin) => {
            tracing::warn!("Failed authentication attempt from {}", ip);
            HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "Invalid PIN"
            }))
        }
        Err(e) => {
            tracing::error!("Auth error: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Authentication failed"
            }))
        }
    }
}
