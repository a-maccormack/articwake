mod api;
mod auth;
mod config;
mod services;

use actix_web::{web, App, HttpServer, HttpResponse, HttpRequest};
use rust_embed::Embed;
use tracing_actix_web::TracingLogger;

#[derive(Embed)]
#[folder = "src/static/"]
struct StaticAssets;

async fn serve_static(req: HttpRequest) -> HttpResponse {
    let path = req.match_info().query("filename");
    let path = if path.is_empty() { "index.html" } else { path };

    match StaticAssets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            HttpResponse::Ok()
                .content_type(mime.as_ref())
                .body(content.data.into_owned())
        }
        None => HttpResponse::NotFound().body("Not found"),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("articwake=info".parse()?)
                .add_directive("actix_web=info".parse()?),
        )
        .init();

    let config = config::Config::from_env()?;
    let bind_addr = format!("{}:{}", config.bind_host, config.port);

    tracing::info!("Starting articwake on {}", bind_addr);

    let app_state = web::Data::new(auth::AppState::new(config));

    HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .app_data(app_state.clone())
            .route("/api/auth", web::post().to(api::auth::authenticate))
            .route("/api/status", web::get().to(api::status::get_status))
            .route("/api/wol", web::post().to(api::wol::send_wol))
            .route("/api/unlock", web::post().to(api::unlock::unlock))
            .route("/", web::get().to(serve_static))
            .route("/{filename:.*}", web::get().to(serve_static))
    })
    .bind(&bind_addr)?
    .run()
    .await?;

    Ok(())
}
