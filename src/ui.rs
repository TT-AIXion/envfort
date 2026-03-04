use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::header::{
    AUTHORIZATION, CACHE_CONTROL, CONTENT_SECURITY_POLICY, CONTENT_TYPE, HOST, ORIGIN, PRAGMA,
    REFERRER_POLICY, X_CONTENT_TYPE_OPTIONS,
};
use axum::http::{HeaderValue, Method, Request, Response, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{Html, IntoResponse};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::net::TcpListener;
use tokio::sync::{Mutex, watch};
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

use crate::crypto::{AadData, KEK, generate_dek, wrap_dek};
use crate::error::{CliError, KeychainError};
use crate::keychain::get_backend;
use crate::storage::{SecretRecord, VaultDb};

const INDEX_HTML: &str = include_str!("ui/index.html");
const APP_CSS: &str = include_str!("ui/app.css");
const APP_JS: &str = include_str!("ui/app.js");
const UI_TOKEN_FILE: &str = "ui-token";
const DEFAULT_PROFILE: &str = "default";
const MASK_VALUE: &str = "********";

#[derive(Clone)]
struct UiState {
    token: Arc<String>,
    token_path: Arc<PathBuf>,
    token_consumed: Arc<AtomicBool>,
    port: u16,
    last_activity: Arc<Mutex<Instant>>,
    timeout: Duration,
    shutdown_tx: watch::Sender<bool>,
}

#[derive(Debug, Serialize)]
struct ApiError {
    error: String,
}

#[derive(Debug, Serialize)]
struct SecretListItem {
    key: String,
    masked: &'static str,
}

#[derive(Debug, Deserialize)]
struct ProfileQuery {
    profile: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SetSecretRequest {
    profile: String,
    key: String,
    value: String,
}

#[derive(Debug, Deserialize)]
struct CreateProfileRequest {
    name: String,
}

#[derive(Debug, Deserialize)]
struct ImportEnvRequest {
    profile: String,
    content: String,
}

type ApiResult<T> = Result<T, ApiErrorResponse>;
type ApiErrorResponse = (StatusCode, Json<ApiError>);

impl UiState {
    async fn touch(&self) {
        let mut last_activity = self.last_activity.lock().await;
        *last_activity = Instant::now();
    }
}

pub async fn run_ui(no_open: bool, timeout_minutes: u64) -> Result<(), CliError> {
    let data_dir = envfort_data_dir()?;
    ensure_secure_dir(&data_dir)?;

    let token = generate_ui_token()?;
    let token_path = data_dir.join(UI_TOKEN_FILE);
    fs::write(&token_path, token.as_bytes())?;
    fs::set_permissions(&token_path, fs::Permissions::from_mode(0o600))?;

    let std_listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    std_listener.set_nonblocking(true)?;
    let port = std_listener.local_addr()?.port();
    let listener = TcpListener::from_std(std_listener)?;

    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let state = Arc::new(UiState {
        token: Arc::new(token),
        token_path: Arc::new(token_path.clone()),
        token_consumed: Arc::new(AtomicBool::new(false)),
        port,
        last_activity: Arc::new(Mutex::new(Instant::now())),
        timeout: Duration::from_secs(timeout_minutes.saturating_mul(60)),
        shutdown_tx,
    });

    let app = build_router(Arc::clone(&state));
    let url = format!("http://127.0.0.1:{port}/");
    println!("envfort ui: {url}");

    if !no_open {
        let _ = open_browser(&url);
    }

    spawn_inactivity_shutdown(Arc::clone(&state));
    axum::serve(listener, app)
        .with_graceful_shutdown(wait_for_shutdown(shutdown_rx))
        .await
        .map_err(CliError::Io)?;

    if token_path.exists() {
        let _ = fs::remove_file(token_path);
    }
    Ok(())
}

fn build_router(state: Arc<UiState>) -> Router {
    Router::new()
        .route("/", get(index_handler))
        .route("/app.css", get(css_handler))
        .route("/app.js", get(js_handler))
        .route("/token", get(token_handler))
        .route("/api/health", get(health_handler))
        .route(
            "/api/profiles",
            get(list_profiles_handler).post(create_profile_handler),
        )
        .route("/api/profiles/{name}", delete(delete_profile_handler))
        .route(
            "/api/secrets",
            get(list_secrets_handler).post(set_secret_handler),
        )
        .route("/api/secrets/{key}", delete(delete_secret_handler))
        .route("/api/import-env", post(import_env_handler))
        .with_state(Arc::clone(&state))
        .layer(middleware::map_response(add_security_headers))
        .layer(middleware::from_fn_with_state(
            Arc::clone(&state),
            auth_guard,
        ))
        .layer(middleware::from_fn_with_state(state, host_origin_guard))
}

async fn index_handler() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn css_handler() -> impl IntoResponse {
    (
        [(CONTENT_TYPE, "text/css; charset=utf-8")],
        APP_CSS.to_string(),
    )
}

async fn js_handler() -> impl IntoResponse {
    (
        [(CONTENT_TYPE, "application/javascript; charset=utf-8")],
        APP_JS.to_string(),
    )
}

async fn token_handler(State(state): State<Arc<UiState>>) -> Result<Response<Body>, StatusCode> {
    if state.token_consumed.swap(true, Ordering::SeqCst) {
        return Err(StatusCode::GONE);
    }

    let token_from_file = match fs::read_to_string(&*state.token_path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Err(StatusCode::GONE),
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };
    let token = token_from_file.trim().to_string();
    if token != *state.token {
        return Err(StatusCode::UNAUTHORIZED);
    }

    fs::remove_file(&*state.token_path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    state.touch().await;
    Ok((StatusCode::OK, token).into_response())
}

async fn health_handler() -> Json<serde_json::Value> {
    Json(json!({ "ok": true }))
}

async fn list_profiles_handler() -> ApiResult<Json<serde_json::Value>> {
    let db = open_default_db().map_err(internal_error)?;
    let mut profiles = db.list_profiles().map_err(internal_error)?;
    if profiles.is_empty() {
        db.create_profile(DEFAULT_PROFILE).map_err(internal_error)?;
        ensure_profile_kek_exists(DEFAULT_PROFILE).map_err(internal_error)?;
        profiles = db.list_profiles().map_err(internal_error)?;
    }

    Ok(Json(json!({ "profiles": profiles })))
}

async fn create_profile_handler(
    Json(req): Json<CreateProfileRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    validate_profile_name(&req.name)?;
    let db = open_default_db().map_err(internal_error)?;
    db.create_profile(&req.name).map_err(internal_error)?;
    ensure_profile_kek_exists(&req.name).map_err(internal_error)?;
    db.log_audit("profile-create", None, Some(&req.name), None)
        .map_err(internal_error)?;

    Ok(Json(json!({ "ok": true, "profile": req.name })))
}

async fn delete_profile_handler(
    AxumPath(name): AxumPath<String>,
) -> ApiResult<Json<serde_json::Value>> {
    validate_profile_name(&name)?;
    let db = open_default_db().map_err(internal_error)?;
    let deleted = db.delete_profile(&name).map_err(internal_error)?;

    if deleted {
        let backend = get_backend();
        match backend.delete_kek(&name) {
            Ok(()) | Err(KeychainError::MissingEntry(_)) => {}
            Err(err) => return Err(internal_error(err)),
        }
        db.log_audit("profile-delete", None, Some(&name), None)
            .map_err(internal_error)?;
    }

    Ok(Json(json!({ "ok": true, "deleted": deleted })))
}

async fn list_secrets_handler(
    Query(query): Query<ProfileQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let profile = normalize_profile(query.profile)?;
    let db = open_default_db().map_err(internal_error)?;
    db.create_profile(&profile).map_err(internal_error)?;

    let keys = db.list_secrets(&profile).map_err(internal_error)?;
    let secrets = keys
        .into_iter()
        .map(|key| SecretListItem {
            key,
            masked: MASK_VALUE,
        })
        .collect::<Vec<_>>();

    Ok(Json(json!({ "profile": profile, "secrets": secrets })))
}

async fn set_secret_handler(
    Json(req): Json<SetSecretRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    validate_profile_name(&req.profile)?;
    validate_env_key(&req.key)?;

    let mut secret_bytes = Zeroizing::new(req.value.into_bytes());
    if secret_bytes.is_empty() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "secret value must not be empty",
        ));
    }

    let db = open_default_db().map_err(internal_error)?;
    db.create_profile(&req.profile).map_err(internal_error)?;

    let backend = get_backend();
    let kek = backend
        .retrieve_kek(&req.profile)
        .map_err(keychain_error_to_api)?;
    store_secret_with_kek(&db, &req.profile, &req.key, secret_bytes.as_slice(), &kek)
        .map_err(internal_error)?;
    secret_bytes.zeroize();

    db.log_audit("set", Some(&req.key), Some(&req.profile), Some("ui"))
        .map_err(internal_error)?;
    Ok(Json(json!({ "ok": true })))
}

async fn delete_secret_handler(
    AxumPath(key): AxumPath<String>,
    Query(query): Query<ProfileQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    validate_env_key(&key)?;
    let profile = normalize_profile(query.profile)?;

    let db = open_default_db().map_err(internal_error)?;
    let deleted = db.delete_secret(&profile, &key).map_err(internal_error)?;
    if deleted {
        db.log_audit("rm", Some(&key), Some(&profile), Some("ui"))
            .map_err(internal_error)?;
    }

    Ok(Json(json!({ "ok": true, "deleted": deleted })))
}

async fn import_env_handler(
    Json(req): Json<ImportEnvRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    validate_profile_name(&req.profile)?;
    let pairs = parse_env_content(&req.content)?;
    if pairs.is_empty() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "no importable entries found in .env content",
        ));
    }

    let db = open_default_db().map_err(internal_error)?;
    db.create_profile(&req.profile).map_err(internal_error)?;

    let backend = get_backend();
    let kek = backend
        .retrieve_kek(&req.profile)
        .map_err(keychain_error_to_api)?;

    for (key, value) in &pairs {
        let value_bytes = Zeroizing::new(value.as_bytes().to_vec());
        store_secret_with_kek(&db, &req.profile, key, value_bytes.as_slice(), &kek)
            .map_err(internal_error)?;
    }

    db.log_audit(
        "import-env",
        None,
        Some(&req.profile),
        Some(&format!("count={}", pairs.len())),
    )
    .map_err(internal_error)?;

    Ok(Json(
        json!({ "ok": true, "imported": pairs.len(), "suggestion": "delete source .env file if no longer needed" }),
    ))
}

async fn host_origin_guard(
    State(state): State<Arc<UiState>>,
    req: Request<Body>,
    next: Next,
) -> Response<Body> {
    let host = req
        .headers()
        .get(HOST)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    if !is_allowed_host(host, state.port) {
        return StatusCode::FORBIDDEN.into_response();
    }

    if matches!(*req.method(), Method::POST | Method::PUT | Method::DELETE) {
        let origin = req
            .headers()
            .get(ORIGIN)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        if !is_allowed_origin(origin, state.port) {
            return StatusCode::FORBIDDEN.into_response();
        }
    }

    next.run(req).await
}

async fn auth_guard(
    State(state): State<Arc<UiState>>,
    req: Request<Body>,
    next: Next,
) -> Response<Body> {
    if !req.uri().path().starts_with("/api/") {
        return next.run(req).await;
    }

    let header = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok());
    let expected = format!("Bearer {}", state.token.as_str());
    if header != Some(expected.as_str()) {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    state.touch().await;
    next.run(req).await
}

async fn add_security_headers(mut response: Response<Body>) -> Response<Body> {
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
        .headers_mut()
        .insert(PRAGMA, HeaderValue::from_static("no-cache"));
    response
        .headers_mut()
        .insert(REFERRER_POLICY, HeaderValue::from_static("no-referrer"));
    response
        .headers_mut()
        .insert(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    response.headers_mut().insert(
        CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self'; connect-src 'self'; frame-ancestors 'none'; form-action 'self'",
        ),
    );
    response
}

fn is_allowed_host(host: &str, port: u16) -> bool {
    let host = host.trim().to_ascii_lowercase();
    if host.is_empty() {
        return false;
    }

    let allowed = ["127.0.0.1", "localhost", "[::1]"];
    allowed
        .iter()
        .any(|candidate| host == *candidate || host == format!("{candidate}:{port}"))
}

fn is_allowed_origin(origin: &str, port: u16) -> bool {
    let origin = origin.trim().to_ascii_lowercase();
    if origin.is_empty() {
        return false;
    }

    origin == format!("http://127.0.0.1:{port}")
        || origin == format!("http://localhost:{port}")
        || origin == format!("http://[::1]:{port}")
}

fn spawn_inactivity_shutdown(state: Arc<UiState>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            if *state.shutdown_tx.borrow() {
                break;
            }

            let last = *state.last_activity.lock().await;
            if last.elapsed() >= state.timeout {
                let _ = state.shutdown_tx.send(true);
                break;
            }
        }
    });
}

async fn wait_for_shutdown(mut shutdown_rx: watch::Receiver<bool>) {
    loop {
        if *shutdown_rx.borrow() {
            break;
        }
        if shutdown_rx.changed().await.is_err() {
            break;
        }
    }
}

fn api_error(status: StatusCode, message: impl Into<String>) -> ApiErrorResponse {
    (
        status,
        Json(ApiError {
            error: message.into(),
        }),
    )
}

fn internal_error(err: impl std::fmt::Display) -> ApiErrorResponse {
    api_error(
        StatusCode::INTERNAL_SERVER_ERROR,
        format!("request failed: {err}"),
    )
}

fn keychain_error_to_api(err: KeychainError) -> ApiErrorResponse {
    match err {
        KeychainError::MissingEntry(_) => api_error(
            StatusCode::BAD_REQUEST,
            "missing key encryption key for profile; run init or create profile first",
        ),
        other => internal_error(other),
    }
}

fn normalize_profile(profile: Option<String>) -> ApiResult<String> {
    let profile = profile.unwrap_or_else(|| DEFAULT_PROFILE.to_string());
    validate_profile_name(&profile)?;
    Ok(profile)
}

fn validate_profile_name(profile: &str) -> ApiResult<()> {
    if profile.is_empty() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "profile name must not be empty",
        ));
    }
    if profile.len() > 64 {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "profile name must be 64 characters or fewer",
        ));
    }
    if profile
        .bytes()
        .any(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, b'_' | b'-' | b'.')))
    {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "profile name contains unsupported characters",
        ));
    }
    Ok(())
}

fn validate_env_key(key: &str) -> ApiResult<()> {
    if key.is_empty() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "key name must not be empty",
        ));
    }
    if !key.is_ascii() {
        return Err(api_error(StatusCode::BAD_REQUEST, "key name must be ASCII"));
    }

    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "key name must not be empty",
        ));
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "key name must start with A-Z, a-z, or _",
        ));
    }
    if chars.any(|ch| !(ch.is_ascii_alphanumeric() || ch == '_')) {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "key name must contain only A-Z, a-z, 0-9, or _",
        ));
    }
    Ok(())
}

fn parse_env_content(content: &str) -> ApiResult<Vec<(String, String)>> {
    let mut parsed = Vec::new();

    for (idx, raw_line) in content.lines().enumerate() {
        let mut line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("export ") {
            line = rest.trim_start();
        }

        let (raw_key, raw_value) = line.split_once('=').ok_or_else(|| {
            api_error(
                StatusCode::BAD_REQUEST,
                format!("line {} is missing '='", idx + 1),
            )
        })?;

        let key = raw_key.trim();
        validate_env_key(key)?;

        let value = parse_env_value(raw_value.trim_end_matches('\r').trim());
        parsed.push((key.to_string(), value));
    }

    Ok(parsed)
}

fn parse_env_value(raw: &str) -> String {
    if raw.len() >= 2 {
        let bytes = raw.as_bytes();
        let starts_ends_double = bytes.first() == Some(&b'"') && bytes.last() == Some(&b'"');
        let starts_ends_single = bytes.first() == Some(&b'\'') && bytes.last() == Some(&b'\'');
        if starts_ends_double || starts_ends_single {
            return raw[1..raw.len() - 1].to_string();
        }
    }
    raw.to_string()
}

fn store_secret_with_kek(
    db: &VaultDb,
    profile: &str,
    key: &str,
    value: &[u8],
    kek: &KEK,
) -> Result<(), CliError> {
    let dek = generate_dek()?;
    let key_id = Uuid::new_v4().to_string();
    let aad = AadData {
        profile_id: profile.to_string(),
        key_id: key_id.clone(),
        record_version: 1,
        aead_alg: "AES-256-GCM-SIV".to_string(),
        kek_id: format!("envfort:{profile}"),
    };

    let (nonce, ciphertext) = crate::crypto::encrypt_value(&dek, value, &aad)?;
    let (wrap_nonce, wrapped_dek) = wrap_dek(kek, &dek, &aad)?;

    let mut encrypted_dek = Vec::with_capacity(wrap_nonce.len() + wrapped_dek.len());
    encrypted_dek.extend_from_slice(&wrap_nonce);
    encrypted_dek.extend_from_slice(&wrapped_dek);

    let record = SecretRecord {
        profile: profile.to_string(),
        key_name: key.to_string(),
        key_id,
        kek_id: aad.kek_id,
        version: i64::from(aad.record_version),
        aead_alg: aad.aead_alg,
        nonce,
        encrypted_dek,
        ciphertext,
    };
    db.set_secret(&record)?;
    Ok(())
}

fn ensure_profile_kek_exists(profile: &str) -> Result<(), CliError> {
    let backend = get_backend();
    match backend.retrieve_kek(profile) {
        Ok(_) => Ok(()),
        Err(KeychainError::MissingEntry(_)) => {
            let kek = generate_random_kek()?;
            backend.store_kek(profile, &kek)?;
            Ok(())
        }
        Err(err) => Err(err.into()),
    }
}

fn generate_random_kek() -> Result<KEK, CliError> {
    let mut raw = [0_u8; crate::crypto::KEY_SIZE];
    getrandom::fill(&mut raw).map_err(|err| CliError::Io(io::Error::other(err.to_string())))?;
    let kek = KEK::from_slice(&raw)?;
    raw.zeroize();
    Ok(kek)
}

fn generate_ui_token() -> Result<String, CliError> {
    let mut token_bytes = [0_u8; 32];
    getrandom::fill(&mut token_bytes)
        .map_err(|err| CliError::Io(io::Error::other(err.to_string())))?;
    Ok(hex_encode(&token_bytes))
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn open_browser(url: &str) -> Result<(), io::Error> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(url).spawn()?;
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()?;
        Ok(())
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        std::process::Command::new("xdg-open").arg(url).spawn()?;
        Ok(())
    }
}

fn open_default_db() -> Result<VaultDb, CliError> {
    let data_dir = envfort_data_dir()?;
    ensure_secure_dir(&data_dir)?;
    let db_path = data_dir.join("vault.db");
    Ok(VaultDb::init_db(db_path)?)
}

fn envfort_data_dir() -> Result<PathBuf, CliError> {
    let home = std::env::var_os("HOME")
        .ok_or_else(|| CliError::InvalidArgument("HOME is not set".to_string()))?;
    Ok(PathBuf::from(home).join(".envfort"))
}

fn ensure_secure_dir(path: &Path) -> Result<(), CliError> {
    fs::create_dir_all(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tower::ServiceExt;

    fn unique_token_path(port: u16) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("envfort-ui-token-{port}-{nonce}.txt"))
    }

    async fn build_test_app(port: u16) -> Router {
        let token_path = unique_token_path(port);
        fs::write(&token_path, "test-token").expect("write test token");
        let (shutdown_tx, _shutdown_rx) = watch::channel(false);
        let state = Arc::new(UiState {
            token: Arc::new("test-token".to_string()),
            token_path: Arc::new(token_path),
            token_consumed: Arc::new(AtomicBool::new(false)),
            port,
            last_activity: Arc::new(Mutex::new(Instant::now())),
            timeout: Duration::from_secs(1800),
            shutdown_tx,
        });
        build_router(state)
    }

    #[tokio::test]
    async fn rejects_dns_rebinding_host_header() {
        let app = build_test_app(31337).await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header(HOST, "evil.example")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn requires_bearer_auth_for_api() {
        let app = build_test_app(31338).await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .header(HOST, "127.0.0.1:31338")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn accepts_valid_host_header() {
        let app = build_test_app(31341).await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header(HOST, "localhost:31341")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn cors_headers_are_not_present() {
        let app = build_test_app(31342).await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header(HOST, "127.0.0.1:31342")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            response
                .headers()
                .get("access-control-allow-origin")
                .is_none()
        );
        assert!(
            response
                .headers()
                .get("access-control-allow-methods")
                .is_none()
        );
        assert!(
            response
                .headers()
                .get("access-control-allow-headers")
                .is_none()
        );
    }

    #[tokio::test]
    async fn rejects_bad_origin_for_state_changes() {
        let app = build_test_app(31339).await;
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/secrets")
                    .header(HOST, "127.0.0.1:31339")
                    .header(ORIGIN, "http://evil.example")
                    .header(AUTHORIZATION, "Bearer test-token")
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"profile":"default","key":"TEST_KEY","value":"secret"}"#,
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn allows_valid_origin_for_state_changes() {
        let app = build_test_app(31343).await;
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/health")
                    .header(HOST, "127.0.0.1:31343")
                    .header(ORIGIN, "http://127.0.0.1:31343")
                    .header(AUTHORIZATION, "Bearer test-token")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_ne!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn token_file_is_single_use() {
        let app = build_test_app(31340).await;

        let first = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/token")
                    .header(HOST, "127.0.0.1:31340")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(first.status(), StatusCode::OK);

        let second = app
            .oneshot(
                Request::builder()
                    .uri("/token")
                    .header(HOST, "127.0.0.1:31340")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(second.status(), StatusCode::GONE);
    }
}
