use actix_web::{App, HttpRequest, HttpResponse, HttpServer, web};
use articwake::{api, auth, config};
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

fn hash_pin() -> anyhow::Result<()> {
    use argon2::Argon2;
    use argon2::password_hash::{PasswordHasher, SaltString, rand_core::OsRng};
    use std::io::{self, BufRead};

    let stdin = io::stdin();
    let pin = stdin
        .lock()
        .lines()
        .next()
        .ok_or_else(|| anyhow::anyhow!("No input provided"))??;
    let pin = pin.trim();

    if pin.is_empty() {
        anyhow::bail!("PIN cannot be empty");
    }

    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(pin.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Failed to hash PIN: {}", e))?;

    println!("{}", hash);
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Handle hash-pin subcommand before any other initialization
    if let Some("hash-pin") = std::env::args().nth(1).as_deref() {
        return hash_pin();
    }

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
