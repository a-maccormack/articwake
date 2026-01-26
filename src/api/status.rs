use actix_web::{HttpRequest, HttpResponse, web};

use crate::auth::AppState;
use crate::services::network::check_host_status;

use super::require_auth;

pub async fn get_status(req: HttpRequest, state: web::Data<AppState>) -> HttpResponse {
    if let Err(resp) = require_auth(&req, &state) {
        return resp;
    }

    let status = check_host_status(&state.config.homelab_ip, state.config.ssh_port);

    HttpResponse::Ok().json(serde_json::json!({
        "homelab_ip": state.config.homelab_ip,
        "reachable": status.reachable,
        "initrd_ssh_open": status.initrd_ssh_open,
        "system_ssh_open": status.system_ssh_open,
        "initrd_ssh_port": state.config.ssh_port
    }))
}
