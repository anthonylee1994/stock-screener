//! Web application wiring.

use std::sync::Arc;

use axum::Router;
use tower_http::cors::{Any, CorsLayer};

use crate::controllers::screener_controller::{AppState, routes};
use crate::services::api::screener_service::ScreenerService;
use crate::utils::stock_database::StockDatabase;

pub const DEFAULT_PORT: u16 = 3000;

pub fn init_logging() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

pub fn create_app(database: StockDatabase) -> Router {
    create_app_with_token(database, std::env::var("API_TOKEN").ok())
}

pub fn create_app_with_token(database: StockDatabase, api_token: Option<String>) -> Router {
    let service = ScreenerService::new(database, api_token);
    let state = AppState {
        service: Arc::new(service),
    };
    routes(state).layer(
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any),
    )
}

pub fn resolve_port() -> u16 {
    std::env::var("PORT")
        .ok()
        .and_then(|port| port.parse().ok())
        .unwrap_or(DEFAULT_PORT)
}

pub async fn run() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    init_logging();

    let database = StockDatabase::default();
    database.initialize()?;

    let port = resolve_port();
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
    tracing::info!("Stock Screener API 啟動咗 http://0.0.0.0:{}", port);
    axum::serve(listener, create_app(database)).await?;
    Ok(())
}
