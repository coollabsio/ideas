use anyhow::Context;
use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderMap, HeaderName, HeaderValue, StatusCode, Uri},
    response::{IntoResponse, Redirect, Response},
    routing::{get, patch, post},
    Json, Router,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use clap::{Parser, Subcommand};
use ideas_domain::{Comment, Idea, IdeaStatus};
use ideas_storage::Store;
use reqwest::Client;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tower_http::{
    limit::RequestBodyLimitLayer, set_header::SetResponseHeaderLayer, trace::TraceLayer,
};
use uuid::Uuid;

const TITLE_MIN: usize = 10;
const TITLE_MAX: usize = 300;
const BODY_MIN: usize = 30;
const BODY_MAX: usize = 10_000;
const AUTH_LOGIN_LIMIT: u32 = 10;
const AUTH_LOGIN_WINDOW: Duration = Duration::from_secs(60);
const CREATE_IDEA_LIMIT: u32 = 5;
const CREATE_IDEA_WINDOW: Duration = Duration::from_secs(60 * 60);
const UPVOTE_LIMIT: u32 = 120;
const UPVOTE_WINDOW: Duration = Duration::from_secs(60);
const COMMENT_LIMIT: u32 = 5;
const COMMENT_WINDOW: Duration = Duration::from_secs(60 * 5);
const COMMENT_MIN: usize = 1;
const COMMENT_MAX: usize = 2_000;
const CONTENT_SECURITY_POLICY: &str = concat!(
    "default-src 'self'; ",
    "base-uri 'self'; ",
    "object-src 'none'; ",
    "frame-ancestors 'none'; ",
    "img-src 'self' https: data:; ",
    "style-src 'self' 'unsafe-inline'; ",
    "script-src 'self' 'unsafe-inline'; ",
    "connect-src 'self'; ",
    "form-action 'self' https://github.com"
);

#[derive(Parser)]
#[command(name = "ideas", author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Serve(ServeArgs),
    Db {
        #[command(subcommand)]
        command: DbCommand,
    },
}

#[derive(Parser, Clone)]
struct ServeArgs {
    #[arg(long, env = "HOST", default_value = "0.0.0.0")]
    host: String,
    #[arg(long, env = "PORT", default_value_t = 4321)]
    port: u16,
    #[arg(long, env = "DB_PATH", default_value = "./data/ideas.db")]
    db_path: String,
}

#[derive(Subcommand)]
enum DbCommand {
    Migrate(DbArgs),
    Revert(DbArgs),
    Info(DbArgs),
    Seed(DbArgs),
}

#[derive(Parser)]
struct DbArgs {
    #[arg(long, env = "DB_PATH", default_value = "./data/ideas.db")]
    db_path: String,
}

#[derive(Clone)]
struct AppState {
    store: Store,
    config: Arc<Config>,
    http: Client,
    rate_limiter: Arc<RateLimiter>,
}

#[derive(Default)]
struct RateLimiter {
    buckets: Mutex<HashMap<String, RateLimitBucket>>,
}

struct RateLimitBucket {
    window_started: Instant,
    count: u32,
}

impl RateLimiter {
    fn allow(&self, scope: &str, key: &str, limit: u32, window: Duration) -> bool {
        let now = Instant::now();
        let mut buckets = self.buckets.lock().expect("rate limiter lock poisoned");
        buckets.retain(|_, bucket| now.duration_since(bucket.window_started) <= window * 2);
        let bucket_key = format!("{scope}:{key}");
        let bucket = buckets.entry(bucket_key).or_insert(RateLimitBucket {
            window_started: now,
            count: 0,
        });
        if now.duration_since(bucket.window_started) > window {
            bucket.window_started = now;
            bucket.count = 0;
        }
        if bucket.count >= limit {
            return false;
        }
        bucket.count += 1;
        true
    }
}

#[derive(Clone)]
struct Config {
    github_client_id: String,
    github_client_secret: String,
    github_login_enabled: bool,
    base_url: String,
    session_ttl_sec: i64,
    moderator_logins: HashSet<String>,
}

impl Config {
    fn from_env() -> Self {
        Self {
            github_client_id: std::env::var("GITHUB_CLIENT_ID").unwrap_or_default(),
            github_client_secret: std::env::var("GITHUB_CLIENT_SECRET").unwrap_or_default(),
            github_login_enabled: std::env::var("GITHUB_LOGIN_ENABLED").as_deref() != Ok("false"),
            base_url: std::env::var("PUBLIC_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:4321".to_string()),
            session_ttl_sec: 7 * 24 * 3600,
            moderator_logins: parse_login_list_env("IDEAS_MODERATOR_LOGINS"),
        }
    }

    fn secure_cookies(&self) -> bool {
        self.base_url.starts_with("https://")
    }

    fn is_moderator(&self, login: &str) -> bool {
        self.moderator_logins.contains(&login.to_ascii_lowercase())
    }
}

fn parse_login_list_env(name: &str) -> HashSet<String> {
    std::env::var(name)
        .unwrap_or_default()
        .split(',')
        .map(|login| login.trim().to_ascii_lowercase())
        .filter(|login| !login.is_empty())
        .collect()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    match cli.command.unwrap_or(Commands::Serve(ServeArgs {
        host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
        port: std::env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(4321),
        db_path: std::env::var("DB_PATH").unwrap_or_else(|_| "./data/ideas.db".to_string()),
    })) {
        Commands::Serve(args) => serve(args).await,
        Commands::Db { command } => run_db(command).await,
    }
}

async fn run_db(command: DbCommand) -> anyhow::Result<()> {
    match command {
        DbCommand::Migrate(args) => {
            let store = Store::connect(&args.db_path).await?;
            store.migrate().await?;
            println!("migrated {}", args.db_path);
        }
        DbCommand::Revert(args) => {
            let store = Store::connect(&args.db_path).await?;
            store.revert_latest().await?;
            println!("reverted latest migration for {}", args.db_path);
        }
        DbCommand::Info(args) => {
            let store = Store::connect(&args.db_path).await?;
            let versions = store.migration_versions().await?;
            if versions.is_empty() {
                println!("no applied migrations");
            } else {
                for version in versions {
                    println!("{version}");
                }
            }
        }
        DbCommand::Seed(args) => {
            let store = Store::connect(&args.db_path).await?;
            store.migrate().await?;
            let report = store.seed_dev_examples().await?;
            println!(
                "seeded {} users, {} ideas, {} upvotes into {}",
                report.users, report.ideas, report.upvotes, args.db_path
            );
        }
    }
    Ok(())
}

async fn serve(args: ServeArgs) -> anyhow::Result<()> {
    let config = Arc::new(Config::from_env());
    let store = Store::connect(&args.db_path).await?;
    store.migrate().await?;
    store.sweep().await?;
    let sweep_store = store.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(3600));
        loop {
            interval.tick().await;
            if let Err(err) = sweep_store.sweep().await {
                tracing::warn!(?err, "session sweep failed");
            }
        }
    });

    let state = AppState {
        store,
        config,
        http: Client::new(),
        rate_limiter: Arc::new(RateLimiter::default()),
    };

    let app = router(state);
    let addr: SocketAddr = format!("{}:{}", args.host, args.port).parse()?;
    tracing::info!(%addr, "listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn router(state: AppState) -> Router {
    let secure = state.config.secure_cookies();
    let app = Router::new()
        .route("/api/health", get(health))
        .route("/api/me", get(me))
        .route("/api/auth/login", get(auth_login))
        .route("/api/auth/callback", get(auth_callback))
        .route("/api/auth/logout", post(auth_logout))
        .route("/api/ideas", get(list_ideas).post(create_idea))
        .route("/api/ideas/:id", patch(update_idea).delete(delete_idea))
        .route("/api/ideas/:id/status", patch(set_idea_status))
        .route("/api/ideas/:id/upvote", post(set_upvote))
        .route(
            "/api/ideas/:id/comments",
            get(list_comments).post(create_comment),
        )
        .route("/api/comments/:id/upvote", post(set_comment_upvote))
        .route(
            "/api/comments/:id",
            patch(update_comment).delete(delete_comment),
        )
        .layer(RequestBodyLimitLayer::new(64 * 1024))
        .layer(TraceLayer::new_for_http())
        .fallback(static_handler);
    security_headers(app, secure).with_state(state)
}

fn security_headers(router: Router<AppState>, secure: bool) -> Router<AppState> {
    let router = router
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("content-security-policy"),
            HeaderValue::from_static(CONTENT_SECURITY_POLICY),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("x-content-type-options"),
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("referrer-policy"),
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("x-frame-options"),
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("permissions-policy"),
            HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
        ));

    if secure {
        router.layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("strict-transport-security"),
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        ))
    } else {
        router
    }
}

#[derive(RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/../../frontend/build"]
struct Assets;

async fn static_handler(uri: Uri) -> Response {
    let mut path = uri.path().trim_start_matches('/').to_string();
    if path.is_empty() {
        path = "index.html".to_string();
    }
    serve_asset(&path)
        .or_else(|| serve_asset("200.html"))
        .unwrap_or_else(|| StatusCode::NOT_FOUND.into_response())
}

fn serve_asset(path: &str) -> Option<Response> {
    let file = Assets::get(path)?;
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    let mut res = Response::new(Body::from(file.data.into_owned()));
    res.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(mime.as_ref()).ok()?,
    );
    if path.starts_with("_app/")
        || path.starts_with("_astro/")
        || path.contains('.') && !path.ends_with(".html")
    {
        res.headers_mut().insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        );
    }
    Some(res)
}

async fn health(State(state): State<AppState>) -> impl IntoResponse {
    match sqlx::query("SELECT 1").execute(state.store.pool()).await {
        Ok(_) => Json(serde_json::json!({ "status": "ok" })).into_response(),
        Err(err) => {
            tracing::warn!(?err, "health check failed");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({ "status": "error" })),
            )
                .into_response()
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MeResponse {
    user: Option<MeUser>,
    csrf_token: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MeUser {
    login: String,
    avatar_url: String,
}

async fn me(State(state): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    let session = get_session(&state, &jar).await;
    let body = if let Some(session) = session {
        MeResponse {
            user: Some(MeUser {
                login: session.user.login,
                avatar_url: session.user.avatar_url,
            }),
            csrf_token: Some(session.csrf_token),
        }
    } else {
        MeResponse {
            user: None,
            csrf_token: None,
        }
    };
    ([(header::CACHE_CONTROL, "no-store")], Json(body))
}

async fn auth_login(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
) -> impl IntoResponse {
    let key = rate_limit_key(&headers, &jar, None);
    if !state
        .rate_limiter
        .allow("auth_login", &key, AUTH_LOGIN_LIMIT, AUTH_LOGIN_WINDOW)
    {
        return (StatusCode::TOO_MANY_REQUESTS, "Too many login attempts").into_response();
    }
    if !state.config.github_login_enabled {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "GitHub login is temporarily disabled.",
        )
            .into_response();
    }
    if state.config.github_client_id.is_empty() {
        return (StatusCode::SERVICE_UNAVAILABLE, "Missing GITHUB_CLIENT_ID").into_response();
    }
    let state_token = Uuid::new_v4().to_string();
    if let Err(err) = state.store.create_oauth_state(&state_token).await {
        tracing::error!(?err, "failed to save oauth state");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    let mut url =
        reqwest::Url::parse("https://github.com/login/oauth/authorize").expect("valid github url");
    url.query_pairs_mut()
        .append_pair("client_id", &state.config.github_client_id)
        .append_pair("state", &state_token)
        .append_pair(
            "redirect_uri",
            &format!("{}/api/auth/callback", state.config.base_url),
        )
        .append_pair("allow_signup", "true");
    Redirect::temporary(url.as_str()).into_response()
}

#[derive(Deserialize)]
struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

async fn auth_callback(
    State(state): State<AppState>,
    jar: CookieJar,
    axum::extract::Query(query): axum::extract::Query<CallbackQuery>,
) -> impl IntoResponse {
    if query.error.is_some() {
        if let Some(s) = query.state.as_deref() {
            let _ = state.store.consume_oauth_state(s).await;
        }
        return Redirect::temporary("/").into_response();
    }
    let Some(code) = query.code else {
        return (StatusCode::BAD_REQUEST, "Missing code").into_response();
    };
    let Some(oauth_state) = query.state else {
        return (StatusCode::BAD_REQUEST, "Missing state").into_response();
    };
    match state.store.consume_oauth_state(&oauth_state).await {
        Ok(true) => {}
        Ok(false) => return (StatusCode::BAD_REQUEST, "Invalid or expired state").into_response(),
        Err(err) => {
            tracing::error!(?err, "failed to consume state");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }
    let tokens = match exchange_code(&state, &code).await {
        Ok(tokens) => tokens,
        Err(err) => {
            tracing::warn!(?err, "github oauth token exchange failed");
            return (StatusCode::BAD_GATEWAY, "GitHub sign-in failed").into_response();
        }
    };
    let gh_user = match fetch_github_user(&state, &tokens.access_token).await {
        Ok(user) => user,
        Err(err) => {
            tracing::warn!(?err, "failed to fetch github user");
            return (StatusCode::BAD_GATEWAY, "GitHub sign-in failed").into_response();
        }
    };
    let user = match state
        .store
        .upsert_user(gh_user.id, &gh_user.login, &gh_user.avatar_url)
        .await
    {
        Ok(user) => user,
        Err(err) => {
            tracing::error!(?err, "failed to upsert user");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    let sid = Uuid::new_v4().to_string();
    let csrf = Uuid::new_v4().to_string();
    let session = match state
        .store
        .create_session(&sid, &csrf, &user, state.config.session_ttl_sec)
        .await
    {
        Ok(session) => session,
        Err(err) => {
            tracing::error!(?err, "failed to create session");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    let cookie = Cookie::build(("sid", session.sid))
        .http_only(true)
        .secure(state.config.secure_cookies())
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(time::Duration::seconds(state.config.session_ttl_sec))
        .build();
    (jar.add(cookie), Redirect::temporary("/")).into_response()
}

async fn auth_logout(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Some(session) = get_session(&state, &jar).await {
        if !csrf_ok(&headers, &session.csrf_token) {
            return (jar, (StatusCode::FORBIDDEN, "Invalid CSRF token")).into_response();
        }
        let _ = state.store.delete_session(&session.sid).await;
    }
    let cookie = Cookie::build(("sid", ""))
        .http_only(true)
        .secure(state.config.secure_cookies())
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(time::Duration::seconds(0))
        .build();
    (jar.add(cookie), StatusCode::NO_CONTENT).into_response()
}

async fn list_ideas(State(state): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    let session = get_session(&state, &jar).await;
    let viewer = session.as_ref().map(|s| s.user_id);
    let viewer_is_moderator = session
        .as_ref()
        .is_some_and(|s| state.config.is_moderator(&s.user.login));
    match state.store.list_ideas(viewer, viewer_is_moderator).await {
        Ok(ideas) => Json(ideas).into_response(),
        Err(err) => {
            tracing::error!(?err, "failed to list ideas");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[derive(Deserialize)]
struct IdeaBody {
    title: String,
    body: String,
}

async fn create_idea(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(payload): Json<IdeaBody>,
) -> impl IntoResponse {
    let Some(session) = require_session(&state, &jar).await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let key = rate_limit_key(&headers, &jar, Some(session.user_id));
    if !state
        .rate_limiter
        .allow("create_idea", &key, CREATE_IDEA_LIMIT, CREATE_IDEA_WINDOW)
    {
        return (StatusCode::TOO_MANY_REQUESTS, "Too many idea submissions").into_response();
    }
    if !csrf_ok(&headers, &session.csrf_token) {
        return (StatusCode::FORBIDDEN, "Bad CSRF token").into_response();
    }
    if let Err(msg) = validate_idea(&payload.title, &payload.body) {
        return (StatusCode::BAD_REQUEST, msg).into_response();
    }
    match state
        .store
        .create_idea(
            payload.title.trim(),
            payload.body.trim(),
            session.user_id,
            state.config.is_moderator(&session.user.login),
        )
        .await
    {
        Ok(idea) => (StatusCode::CREATED, Json(idea)).into_response(),
        Err(err) => {
            tracing::error!(?err, "failed to create idea");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn update_idea(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(payload): Json<IdeaBody>,
) -> impl IntoResponse {
    let Some(session) = require_session(&state, &jar).await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    if !csrf_ok(&headers, &session.csrf_token) {
        return (StatusCode::FORBIDDEN, "Bad CSRF token").into_response();
    }
    if let Err(msg) = validate_idea(&payload.title, &payload.body) {
        return (StatusCode::BAD_REQUEST, msg).into_response();
    }
    match state
        .store
        .update_idea(
            id,
            payload.title.trim(),
            payload.body.trim(),
            session.user_id,
            state.config.is_moderator(&session.user.login),
        )
        .await
    {
        Ok(idea) => Json(idea).into_response(),
        Err(ideas_storage::StorageError::NotFound(_)) => StatusCode::NOT_FOUND.into_response(),
        Err(err) => {
            tracing::error!(?err, "failed to update idea");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[derive(Deserialize)]
struct StatusBody {
    status: IdeaStatus,
}

async fn set_idea_status(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(payload): Json<StatusBody>,
) -> impl IntoResponse {
    let Some(session) = require_session(&state, &jar).await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    if !csrf_ok(&headers, &session.csrf_token) {
        return (StatusCode::FORBIDDEN, "Bad CSRF token").into_response();
    }
    let actor_is_moderator = state.config.is_moderator(&session.user.login);
    if !actor_is_moderator {
        return (
            StatusCode::FORBIDDEN,
            "Only moderators can update idea status",
        )
            .into_response();
    }
    match state
        .store
        .set_idea_status(id, payload.status, session.user_id, actor_is_moderator)
        .await
    {
        Ok(idea) => Json(idea).into_response(),
        Err(ideas_storage::StorageError::NotFound(_)) => StatusCode::NOT_FOUND.into_response(),
        Err(err) => {
            tracing::error!(?err, "failed to set idea status");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn delete_idea(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let Some(session) = require_session(&state, &jar).await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    if !csrf_ok(&headers, &session.csrf_token) {
        return (StatusCode::FORBIDDEN, "Bad CSRF token").into_response();
    }
    match state
        .store
        .delete_idea(
            id,
            session.user_id,
            state.config.is_moderator(&session.user.login),
        )
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(ideas_storage::StorageError::NotFound(_)) => StatusCode::NOT_FOUND.into_response(),
        Err(err) => {
            tracing::error!(?err, "failed to delete idea");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[derive(Deserialize)]
struct UpvoteBody {
    upvoted: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UpvoteResponse {
    upvote_count: i64,
    viewer_has_upvoted: bool,
    idea: Idea,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CommentUpvoteResponse {
    upvote_count: i64,
    viewer_has_upvoted: bool,
    comment: Comment,
}

async fn set_upvote(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpvoteBody>,
) -> impl IntoResponse {
    let Some(session) = require_session(&state, &jar).await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let key = rate_limit_key(&headers, &jar, Some(session.user_id));
    if !state
        .rate_limiter
        .allow("upvote", &key, UPVOTE_LIMIT, UPVOTE_WINDOW)
    {
        return (StatusCode::TOO_MANY_REQUESTS, "Too many upvote changes").into_response();
    }
    if !csrf_ok(&headers, &session.csrf_token) {
        return (StatusCode::FORBIDDEN, "Bad CSRF token").into_response();
    }
    match state
        .store
        .set_upvote(
            id,
            session.user_id,
            payload.upvoted,
            state.config.is_moderator(&session.user.login),
        )
        .await
    {
        Ok(idea) => Json(UpvoteResponse {
            upvote_count: idea.upvote_count,
            viewer_has_upvoted: idea.viewer_has_upvoted,
            idea,
        })
        .into_response(),
        Err(ideas_storage::StorageError::NotFound(_)) => StatusCode::NOT_FOUND.into_response(),
        Err(err) => {
            tracing::error!(?err, "failed to set upvote");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[derive(Deserialize)]
struct CommentBody {
    body: String,
}

async fn list_comments(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let session = get_session(&state, &jar).await;
    let viewer = session.as_ref().map(|s| s.user_id);
    let viewer_is_moderator = session
        .as_ref()
        .is_some_and(|s| state.config.is_moderator(&s.user.login));
    match state
        .store
        .list_comments(id, viewer, viewer_is_moderator)
        .await
    {
        Ok(comments) => Json(comments).into_response(),
        Err(ideas_storage::StorageError::NotFound(_)) => StatusCode::NOT_FOUND.into_response(),
        Err(err) => {
            tracing::error!(?err, "failed to list comments");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn create_comment(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(payload): Json<CommentBody>,
) -> impl IntoResponse {
    let Some(session) = require_session(&state, &jar).await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let key = rate_limit_key(&headers, &jar, Some(session.user_id));
    if !state
        .rate_limiter
        .allow("comment", &key, COMMENT_LIMIT, COMMENT_WINDOW)
    {
        return (StatusCode::TOO_MANY_REQUESTS, "Too many comments").into_response();
    }
    if !csrf_ok(&headers, &session.csrf_token) {
        return (StatusCode::FORBIDDEN, "Bad CSRF token").into_response();
    }
    if let Err(msg) = validate_comment(&payload.body) {
        return (StatusCode::BAD_REQUEST, msg).into_response();
    }
    let actor_is_moderator = state.config.is_moderator(&session.user.login);
    match state
        .store
        .create_comment(id, payload.body.trim(), session.user_id, actor_is_moderator)
        .await
    {
        Ok(comment) => (StatusCode::CREATED, Json(comment)).into_response(),
        Err(ideas_storage::StorageError::NotFound(_)) => StatusCode::NOT_FOUND.into_response(),
        Err(err) => {
            tracing::error!(?err, "failed to create comment");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn set_comment_upvote(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpvoteBody>,
) -> impl IntoResponse {
    let Some(session) = require_session(&state, &jar).await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let key = rate_limit_key(&headers, &jar, Some(session.user_id));
    if !state
        .rate_limiter
        .allow("comment-upvote", &key, UPVOTE_LIMIT, UPVOTE_WINDOW)
    {
        return (StatusCode::TOO_MANY_REQUESTS, "Too many upvote changes").into_response();
    }
    if !csrf_ok(&headers, &session.csrf_token) {
        return (StatusCode::FORBIDDEN, "Bad CSRF token").into_response();
    }
    match state
        .store
        .set_comment_upvote(
            id,
            session.user_id,
            payload.upvoted,
            state.config.is_moderator(&session.user.login),
        )
        .await
    {
        Ok(comment) => Json(CommentUpvoteResponse {
            upvote_count: comment.upvote_count,
            viewer_has_upvoted: comment.viewer_has_upvoted,
            comment,
        })
        .into_response(),
        Err(ideas_storage::StorageError::NotFound(_)) => StatusCode::NOT_FOUND.into_response(),
        Err(err) => {
            tracing::error!(?err, "failed to set comment upvote");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn update_comment(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(payload): Json<CommentBody>,
) -> impl IntoResponse {
    let Some(session) = require_session(&state, &jar).await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    if !csrf_ok(&headers, &session.csrf_token) {
        return (StatusCode::FORBIDDEN, "Bad CSRF token").into_response();
    }
    if let Err(msg) = validate_comment(&payload.body) {
        return (StatusCode::BAD_REQUEST, msg).into_response();
    }
    let actor_is_moderator = state.config.is_moderator(&session.user.login);
    match state
        .store
        .update_comment(id, payload.body.trim(), session.user_id, actor_is_moderator)
        .await
    {
        Ok(comment) => Json(comment).into_response(),
        Err(ideas_storage::StorageError::NotFound(_)) => StatusCode::NOT_FOUND.into_response(),
        Err(err) => {
            tracing::error!(?err, "failed to update comment");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn delete_comment(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let Some(session) = require_session(&state, &jar).await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    if !csrf_ok(&headers, &session.csrf_token) {
        return (StatusCode::FORBIDDEN, "Bad CSRF token").into_response();
    }
    match state
        .store
        .delete_comment(
            id,
            session.user_id,
            state.config.is_moderator(&session.user.login),
        )
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(ideas_storage::StorageError::NotFound(_)) => StatusCode::NOT_FOUND.into_response(),
        Err(err) => {
            tracing::error!(?err, "failed to delete comment");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

fn validate_idea(title: &str, body: &str) -> Result<(), String> {
    let title_len = title.trim().chars().count();
    let body_len = body.trim().chars().count();
    if !(TITLE_MIN..=TITLE_MAX).contains(&title_len) {
        return Err(format!("Title must be {TITLE_MIN}–{TITLE_MAX} characters"));
    }
    if !(BODY_MIN..=BODY_MAX).contains(&body_len) {
        return Err(format!("Body must be {BODY_MIN}–{BODY_MAX} characters"));
    }
    Ok(())
}

fn validate_comment(body: &str) -> Result<(), String> {
    let body_len = body.trim().chars().count();
    if !(COMMENT_MIN..=COMMENT_MAX).contains(&body_len) {
        return Err(format!(
            "Comment must be {COMMENT_MIN}–{COMMENT_MAX} characters"
        ));
    }
    Ok(())
}

async fn get_session(state: &AppState, jar: &CookieJar) -> Option<ideas_domain::Session> {
    let sid = jar.get("sid")?.value().to_string();
    match state.store.session(&sid).await {
        Ok(session) => session,
        Err(err) => {
            tracing::warn!(?err, "failed to load session");
            None
        }
    }
}

async fn require_session(state: &AppState, jar: &CookieJar) -> Option<ideas_domain::Session> {
    get_session(state, jar).await
}

fn csrf_ok(headers: &HeaderMap, csrf: &str) -> bool {
    headers.get("x-csrf-token").and_then(|v| v.to_str().ok()) == Some(csrf)
}

fn rate_limit_key(headers: &HeaderMap, jar: &CookieJar, user_id: Option<Uuid>) -> String {
    if let Some(user_id) = user_id {
        return format!("user:{user_id}");
    }
    if let Some(sid) = jar.get("sid").map(|cookie| cookie.value()) {
        return format!("sid:{sid}");
    }
    // In direct deployments there is no proxy-provided client IP available in
    // handlers. These headers are useful behind a trusted reverse proxy, but do
    // not rely on them for hostile direct-to-origin traffic.
    headers
        .get("x-real-ip")
        .or_else(|| headers.get("x-forwarded-for"))
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("ip:{value}"))
        .unwrap_or_else(|| "anonymous".to_string())
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

struct OAuthTokens {
    access_token: String,
}

async fn exchange_code(state: &AppState, code: &str) -> anyhow::Result<OAuthTokens> {
    let res = state
        .http
        .post("https://github.com/login/oauth/access_token")
        .header(header::ACCEPT, "application/json")
        .json(&serde_json::json!({
            "client_id": state.config.github_client_id,
            "client_secret": state.config.github_client_secret,
            "code": code,
            "redirect_uri": format!("{}/api/auth/callback", state.config.base_url),
        }))
        .send()
        .await?
        .error_for_status()?;
    let data: TokenResponse = res.json().await?;
    let access_token = data.access_token.with_context(|| {
        format!(
            "OAuth error: {} {}",
            data.error.unwrap_or_default(),
            data.error_description.unwrap_or_default()
        )
    })?;
    Ok(OAuthTokens { access_token })
}

#[derive(Deserialize)]
struct GitHubUser {
    id: i64,
    login: String,
    avatar_url: String,
}

async fn fetch_github_user(state: &AppState, token: &str) -> anyhow::Result<GitHubUser> {
    let res = state
        .http
        .get("https://api.github.com/user")
        .bearer_auth(token)
        .header(header::ACCEPT, "application/vnd.github+json")
        .header(header::USER_AGENT, "coollabs-ideas")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await?
        .error_for_status()?;
    Ok(res.json().await?)
}

#[cfg(test)]
mod tests {
    use super::{
        router, validate_idea, AppState, Config, RateLimiter, AUTH_LOGIN_LIMIT,
        CONTENT_SECURITY_POLICY,
    };
    use axum::{body::Body, http::Request, http::StatusCode};
    use ideas_storage::Store;
    use reqwest::Client;
    use std::{collections::HashSet, sync::Arc, time::Duration};
    use tower::ServiceExt;
    use uuid::Uuid;

    #[test]
    fn validates_idea_lengths() {
        assert!(validate_idea(
            "A valid title",
            "This body is definitely longer than thirty chars."
        )
        .is_ok());
        assert!(
            validate_idea("short", "This body is definitely longer than thirty chars.").is_err()
        );
    }

    async fn test_state(base_url: &str) -> AppState {
        let path = std::env::temp_dir().join(format!("ideas-server-test-{}.db", Uuid::new_v4()));
        let store = Store::connect(path.to_str().expect("utf8 temp path"))
            .await
            .expect("connect");
        store.migrate().await.expect("migrate");
        AppState {
            store,
            config: Arc::new(Config {
                github_client_id: "client-id".to_string(),
                github_client_secret: "client-secret".to_string(),
                github_login_enabled: true,
                base_url: base_url.to_string(),
                session_ttl_sec: 3600,
                moderator_logins: HashSet::new(),
            }),
            http: Client::new(),
            rate_limiter: Arc::new(RateLimiter::default()),
        }
    }

    #[tokio::test]
    async fn security_headers_are_added_to_api_and_static_responses() {
        let app = router(test_state("http://localhost:4321").await);

        let api = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            api.headers().get("content-security-policy").unwrap(),
            CONTENT_SECURITY_POLICY
        );
        assert_eq!(
            api.headers().get("x-content-type-options").unwrap(),
            "nosniff"
        );
        assert_eq!(
            api.headers().get("referrer-policy").unwrap(),
            "strict-origin-when-cross-origin"
        );
        assert_eq!(api.headers().get("x-frame-options").unwrap(), "DENY");
        assert_eq!(
            api.headers().get("permissions-policy").unwrap(),
            "camera=(), microphone=(), geolocation=()"
        );
        assert!(api.headers().get("strict-transport-security").is_none());

        let static_response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(
            static_response
                .headers()
                .get("content-security-policy")
                .unwrap(),
            CONTENT_SECURITY_POLICY
        );
        assert_eq!(
            static_response
                .headers()
                .get("x-content-type-options")
                .unwrap(),
            "nosniff"
        );
    }

    #[tokio::test]
    async fn hsts_is_added_only_for_https_base_url() {
        let app = router(test_state("https://ideas.example.com").await);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            response.headers().get("strict-transport-security").unwrap(),
            "max-age=31536000; includeSubDomains"
        );
    }

    #[tokio::test]
    async fn auth_login_rate_limit_returns_429() {
        let app = router(test_state("http://localhost:4321").await);
        for _ in 0..AUTH_LOGIN_LIMIT {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri("/api/auth/login")
                        .header("x-real-ip", "203.0.113.1")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_ne!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        }

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/auth/login")
                    .header("x-real-ip", "203.0.113.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn comment_upvote_route_updates_comment() {
        let state = test_state("http://localhost:4321").await;
        let author = state
            .store
            .upsert_user(301, "author", "https://example.com/author.png")
            .await
            .expect("author");
        let voter = state
            .store
            .upsert_user(302, "voter", "https://example.com/voter.png")
            .await
            .expect("voter");
        let idea = state
            .store
            .create_idea(
                "A routed comment upvote",
                "This body is long enough for a routed comment upvote test.",
                author.id,
                false,
            )
            .await
            .expect("idea");
        let comment = state
            .store
            .create_comment(idea.id, "A comment worth upvoting.", author.id, false)
            .await
            .expect("comment");
        state
            .store
            .create_session("sid-comment-upvote", "csrf-comment-upvote", &voter, 3600)
            .await
            .expect("session");
        let app = router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/comments/{}/upvote", comment.id))
                    .header("cookie", "sid=sid-comment-upvote")
                    .header("content-type", "application/json")
                    .header("x-csrf-token", "csrf-comment-upvote")
                    .body(Body::from(r#"{"upvoted":true}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn moderator_login_matching_is_case_insensitive() {
        let config = Config {
            github_client_id: String::new(),
            github_client_secret: String::new(),
            github_login_enabled: true,
            base_url: "http://localhost:4321".to_string(),
            session_ttl_sec: 3600,
            moderator_logins: HashSet::from(["alice".to_string()]),
        };

        assert!(config.is_moderator("Alice"));
        assert!(config.is_moderator("ALICE"));
        assert!(!config.is_moderator("bob"));
    }

    #[test]
    fn rate_limiter_enforces_window_limit() {
        let limiter = RateLimiter::default();
        assert!(limiter.allow("test", "key", 1, Duration::from_secs(60)));
        assert!(!limiter.allow("test", "key", 1, Duration::from_secs(60)));
    }
}
