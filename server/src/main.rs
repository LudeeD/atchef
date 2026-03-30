use anyhow::Context;
use axum::{
    Router,
    routing::{get, post},
};
use sqlx::SqlitePool;
use std::sync::Arc;
use tower_sessions::SessionManagerLayer;
use tower_sessions_sqlx_store::SqliteStore;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod blob_cache;
mod db;
mod handlers;
#[allow(dead_code)]
mod lexicons;
mod models;
mod oauth;
mod sync;
mod views;

#[derive(Clone)]
pub struct AppState {
    pub http_client: reqwest::Client,
    pub base_url: String,
    pub client_id: String,
    pub sqlite_pool: SqlitePool,
    pub blob_cache: Arc<blob_cache::BlobCacheService>,
    pub admin_token: Option<String>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db_path = std::env::var("DATABASE_PATH").unwrap_or_else(|_| "sessions.db".to_string());
    let database_url = format!("sqlite://{}?mode=rwc", db_path);
    info!("DATABASE_PATH: {}", db_path);
    let sqlite_pool = sqlx::sqlite::SqlitePool::connect(&database_url)
        .await
        .context("failed to connect to SQLite session database")
        .unwrap();
    let session_store = SqliteStore::new(sqlite_pool.clone());
    session_store
        .migrate()
        .await
        .context("failed to migrate session database")
        .unwrap();

    db::init_db(&sqlite_pool)
        .await
        .context("failed to initialize recipe database")
        .unwrap();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_same_site(tower_sessions::cookie::SameSite::Lax);

    let base_url =
        std::env::var("BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());
    let secure_cookies = base_url.starts_with("https://");
    let is_loopback = base_url.starts_with("http://localhost") || base_url.starts_with("http://127.0.0.1");
    let client_id = if is_loopback {
        format!(
            "http://localhost?redirect_uri={}&scope=atproto%20transition%3Ageneric",
            urlencoding::encode(&format!("{}/oauth/callback", base_url)),
        )
    } else {
        format!("{}/client-metadata.json", base_url)
    };
    info!("BASE_URL:      {}", base_url);
    info!("CLIENT_ID:     {}", client_id);
    info!("Secure cookies: {}", secure_cookies);

    let session_layer = session_layer.with_secure(secure_cookies);

    // Initialize blob cache service
    let cache_size_mb = std::env::var("BLOB_CACHE_SIZE_MB")
        .unwrap_or_else(|_| "200".to_string())
        .parse::<u64>()
        .unwrap_or(200);
    let blob_cache = Arc::new(blob_cache::BlobCacheService::new(
        Arc::new(sqlite_pool.clone()),
        cache_size_mb,
    ));

    let admin_token = std::env::var("ADMIN_TOKEN").ok();
    if admin_token.is_none() {
        info!("ADMIN_TOKEN not set — admin routes disabled");
    }

    let state = AppState {
        http_client: reqwest::Client::new(),
        base_url,
        client_id,
        sqlite_pool,
        blob_cache,
        admin_token,
    };

    tokio::spawn(sync::run(state.http_client.clone(), state.sqlite_pool.clone(), state.blob_cache.clone()));

    let app = Router::new()
        .route("/", get(handlers::home))
        .route("/profile/{handle}", get(handlers::public_profile))
        .route("/profile/{handle}/recipe/{rkey}", get(handlers::recipe))
        .route("/profile/{handle}/recipe/{rkey}/delete", post(handlers::delete_recipe))
        .route(
            "/profile/{handle}/recipe/{rkey}/edit",
            get(handlers::edit_recipe_form).post(handlers::update_recipe),
        )
        .route("/blob/{cid}", get(handlers::serve_blob))
        .route(
            "/recipe/new",
            get(handlers::new_recipe_form).post(handlers::create_recipe),
        )
        .route(
            "/login",
            get(handlers::login_page_handler).post(handlers::login_start),
        )
        .route("/oauth/callback", get(handlers::oauth_callback))
        .route("/logout", post(handlers::logout))
        .route("/profile", get(handlers::profile))
        .route("/chefs", get(handlers::chefs))
        .route("/admin", get(handlers::admin_page).post(handlers::admin_login))
        .route("/admin/cleanup", post(handlers::admin_cleanup))
        .route("/admin/fix-image-cache", post(handlers::admin_fix_image_cache))
        .route("/client-metadata.json", get(handlers::client_metadata))
        .route(
            "/.well-known/oauth-client-metadata",
            get(handlers::client_metadata),
        )
        .layer(session_layer)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .context("failed to bind TcpListener")
        .unwrap();

    info!("Starting server at 0.0.0.0:3000");
    axum::serve(listener, app).await.unwrap();
}
