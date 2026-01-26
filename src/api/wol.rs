use actix_web::{HttpRequest, HttpResponse, web};

use crate::auth::AppState;
use crate::services::wol::send_magic_packet;

use super::require_auth;

pub async fn send_wol(req: HttpRequest, state: web::Data<AppState>) -> HttpResponse {
    if let Err(resp) = require_auth(&req, &state) {
        return resp;
    }

    match send_magic_packet(&state.config.homelab_mac, &state.config.homelab_broadcast) {
        Ok(()) => HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": "Wake-on-LAN packet sent"
        })),
        Err(e) => {
            tracing::error!("WOL failed: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to send WOL packet: {}", e)
            }))
        }
    }
}
