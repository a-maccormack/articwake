use actix_web::{web, HttpRequest, HttpResponse};
use serde::Deserialize;

use crate::auth::AppState;
use crate::services::ssh::send_passphrase;

use super::require_auth;

#[derive(Deserialize)]
pub struct UnlockRequest {
    passphrase: String,
}

pub async fn unlock(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<UnlockRequest>,
) -> HttpResponse {
    if let Err(resp) = require_auth(&req, &state) {
        return resp;
    }

    if body.passphrase.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Passphrase cannot be empty"
        }));
    }

    if body.passphrase.len() > 1024 {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Passphrase too long"
        }));
    }

    if body.passphrase.chars().any(|c| c.is_control()) {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Passphrase contains invalid characters"
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
