use actix_web::{test, web, App};
use articwake::api;
use articwake::auth::AppState;
use articwake::config::Config;
use std::io::Write;
use std::path::PathBuf;
use tempfile::NamedTempFile;

fn create_test_config(pin_hash_path: PathBuf) -> Config {
    Config {
        bind_host: "127.0.0.1".to_string(),
        port: 8080,
        homelab_mac: "aa:bb:cc:dd:ee:ff".to_string(),
        homelab_ip: "127.0.0.1".to_string(),
        homelab_broadcast: "255.255.255.255".to_string(),
        ssh_port: 2222,
        ssh_key_path: PathBuf::from("/tmp/nonexistent-key"),
        pin_hash_path,
    }
}

fn create_pin_hash(pin: &str) -> NamedTempFile {
    use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
    use argon2::Argon2;

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(pin.as_bytes(), &salt)
        .expect("Failed to hash PIN");

    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    writeln!(file, "{}", hash).expect("Failed to write hash");
    file
}

fn create_test_app(
    state: web::Data<AppState>,
) -> App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Response = actix_web::dev::ServiceResponse<impl actix_web::body::MessageBody>,
        Config = (),
        InitError = (),
        Error = actix_web::Error,
    >,
> {
    App::new()
        .app_data(state)
        .route("/api/auth", web::post().to(api::auth::authenticate))
        .route("/api/status", web::get().to(api::status::get_status))
        .route("/api/wol", web::post().to(api::wol::send_wol))
}

#[actix_rt::test]
async fn test_auth_success() {
    let hash_file = create_pin_hash("1234");
    let config = create_test_config(hash_file.path().to_path_buf());
    let state = web::Data::new(AppState::new(config));

    let app = test::init_service(create_test_app(state)).await;

    let req = test::TestRequest::post()
        .uri("/api/auth")
        .set_json(serde_json::json!({"pin": "1234"}))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("token").is_some());
    assert_eq!(body["token"].as_str().unwrap().len(), 64);
}

#[actix_rt::test]
async fn test_auth_invalid_pin() {
    let hash_file = create_pin_hash("1234");
    let config = create_test_config(hash_file.path().to_path_buf());
    let state = web::Data::new(AppState::new(config));

    let app = test::init_service(create_test_app(state)).await;

    let req = test::TestRequest::post()
        .uri("/api/auth")
        .set_json(serde_json::json!({"pin": "wrong"}))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_rt::test]
async fn test_auth_missing_pin() {
    let hash_file = create_pin_hash("1234");
    let config = create_test_config(hash_file.path().to_path_buf());
    let state = web::Data::new(AppState::new(config));

    let app = test::init_service(create_test_app(state)).await;

    let req = test::TestRequest::post()
        .uri("/api/auth")
        .set_json(serde_json::json!({}))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
}

#[actix_rt::test]
async fn test_status_unauthorized() {
    let hash_file = create_pin_hash("1234");
    let config = create_test_config(hash_file.path().to_path_buf());
    let state = web::Data::new(AppState::new(config));

    let app = test::init_service(create_test_app(state)).await;

    let req = test::TestRequest::get().uri("/api/status").to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_rt::test]
async fn test_status_with_valid_token() {
    let hash_file = create_pin_hash("1234");
    let config = create_test_config(hash_file.path().to_path_buf());
    let state = web::Data::new(AppState::new(config));

    let app = test::init_service(create_test_app(state.clone())).await;

    // First authenticate
    let auth_req = test::TestRequest::post()
        .uri("/api/auth")
        .set_json(serde_json::json!({"pin": "1234"}))
        .to_request();

    let auth_resp = test::call_service(&app, auth_req).await;
    let auth_body: serde_json::Value = test::read_body_json(auth_resp).await;
    let token = auth_body["token"].as_str().unwrap();

    // Then get status
    let status_req = test::TestRequest::get()
        .uri("/api/status")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let status_resp = test::call_service(&app, status_req).await;
    assert!(status_resp.status().is_success());

    let status_body: serde_json::Value = test::read_body_json(status_resp).await;
    assert!(status_body.get("reachable").is_some());
    assert!(status_body.get("initrd_ssh_open").is_some());
    assert!(status_body.get("system_ssh_open").is_some());
}

#[actix_rt::test]
async fn test_status_with_invalid_token() {
    let hash_file = create_pin_hash("1234");
    let config = create_test_config(hash_file.path().to_path_buf());
    let state = web::Data::new(AppState::new(config));

    let app = test::init_service(create_test_app(state)).await;

    let req = test::TestRequest::get()
        .uri("/api/status")
        .insert_header(("Authorization", "Bearer invalid-token"))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_rt::test]
async fn test_wol_unauthorized() {
    let hash_file = create_pin_hash("1234");
    let config = create_test_config(hash_file.path().to_path_buf());
    let state = web::Data::new(AppState::new(config));

    let app = test::init_service(create_test_app(state)).await;

    let req = test::TestRequest::post().uri("/api/wol").to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_rt::test]
async fn test_wol_with_valid_token() {
    let hash_file = create_pin_hash("1234");
    let config = create_test_config(hash_file.path().to_path_buf());
    let state = web::Data::new(AppState::new(config));

    let app = test::init_service(create_test_app(state.clone())).await;

    // First authenticate
    let auth_req = test::TestRequest::post()
        .uri("/api/auth")
        .set_json(serde_json::json!({"pin": "1234"}))
        .to_request();

    let auth_resp = test::call_service(&app, auth_req).await;
    let auth_body: serde_json::Value = test::read_body_json(auth_resp).await;
    let token = auth_body["token"].as_str().unwrap();

    // Then send WOL
    let wol_req = test::TestRequest::post()
        .uri("/api/wol")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let wol_resp = test::call_service(&app, wol_req).await;
    assert!(wol_resp.status().is_success());

    let wol_body: serde_json::Value = test::read_body_json(wol_resp).await;
    assert_eq!(wol_body["success"], true);
}

#[actix_rt::test]
async fn test_rate_limiting() {
    let hash_file = create_pin_hash("1234");
    let config = create_test_config(hash_file.path().to_path_buf());
    let state = web::Data::new(AppState::new(config));

    let app = test::init_service(create_test_app(state)).await;

    // Make 10 failed attempts (the limit)
    for _ in 0..10 {
        let req = test::TestRequest::post()
            .uri("/api/auth")
            .set_json(serde_json::json!({"pin": "wrong"}))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 401);
    }

    // 11th attempt should be rate limited
    let req = test::TestRequest::post()
        .uri("/api/auth")
        .set_json(serde_json::json!({"pin": "wrong"}))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 429); // Too Many Requests
}
