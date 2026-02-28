use anyhow::Context;
use axum::{
    routing::{get, post},
    Router,
};
use sqlx::SqlitePool;
use tower_sessions::{SessionManagerLayer};
use tower_sessions_sqlx_store::SqliteStore;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod handlers;
#[allow(dead_code)]
mod lexicons;
mod models;
mod oauth;
mod views;
mod db;

#[derive(Clone)]
pub struct AppState {
    pub http_client: reqwest::Client,
    pub base_url: String,
    pub sqlite_pool: SqlitePool,
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

    // Create database in the server directory (where Cargo.toml is)
    let db_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sessions.db");
    let database_url = format!("sqlite://{}?mode=rwc", db_path.to_str().unwrap());
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

    db::init_db(&sqlite_pool).await
        .context("failed to initialize recipe database")
        .unwrap();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_same_site(tower_sessions::cookie::SameSite::Lax);

    let state = AppState {
        http_client: reqwest::Client::new(),
        base_url: "http://127.0.0.1:3000".to_string(),
        sqlite_pool,
    };

    let app = Router::new()
        .route("/", get(handlers::home))
        .route("/profile/{handle}", get(handlers::public_profile))
        .route("/profile/{handle}/recipe/{rkey}", get(handlers::recipe))
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
        .route("/client-metadata.json", get(handlers::client_metadata))
        .route(
            "/.well-known/oauth-client-metadata",
            get(handlers::client_metadata),
        )
        .layer(session_layer)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .context("failed to bind TcpListener")
        .unwrap();

    info!("Starting server at http://127.0.0.1:3000");
    axum::serve(listener, app).await.unwrap();
}
