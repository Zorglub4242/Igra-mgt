/// API Routes definition

use axum::{
    Router,
    routing::{get, post},
    middleware,
};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use super::handlers;
use super::websocket;
use super::static_files;
use super::auth;

pub fn create_router(enable_cors: bool) -> Router {
    // Protected routes (require authentication)
    let protected_routes = Router::new()
        .route("/api/services/:name/start", post(handlers::start_service))
        .route("/api/services/:name/stop", post(handlers::stop_service))
        .route("/api/services/:name/restart", post(handlers::restart_service))
        .route("/api/storage/prune", post(handlers::prune_storage))
        .route("/api/profiles/:name/start", post(handlers::start_profile))
        .route("/api/profiles/:name/stop", post(handlers::stop_profile))
        .route("/api/update", post(handlers::trigger_update))
        .layer(middleware::from_fn(auth::auth_middleware));

    // Public routes (read-only, no auth required)
    let public_routes = Router::new()
        .route("/api/services", get(handlers::get_services))
        .route("/api/services/:name/logs", get(handlers::get_logs))
        .route("/api/services/:name/logs/parsed", get(handlers::get_logs_parsed))
        .route("/api/wallets", get(handlers::get_wallets))
        .route("/api/wallets/:id/balance", get(handlers::get_wallet_balance))
        .route("/api/wallets/:id/detail", get(handlers::get_wallet_detail))
        .route("/api/storage", get(handlers::get_storage))
        .route("/api/storage/history", get(handlers::get_storage_history))
        .route("/api/config", get(handlers::get_config))
        .route("/api/system", get(handlers::get_system_info))
        .route("/api/rpc/tokens", get(handlers::get_rpc_tokens))
        .route("/api/ssl/info", get(handlers::get_ssl_info))
        .route("/api/profiles", get(handlers::get_profiles))
        .route("/api/transactions", get(handlers::get_transactions))
        .route("/api/transactions/stats", get(handlers::get_transaction_stats))
        .route("/api/health", get(handlers::health_check))
        .route("/api/metrics", get(handlers::get_metrics))
        .route("/api/version", get(handlers::get_version_info))
        .route("/ws/logs/:service", get(websocket::ws_logs_handler))
        .route("/ws/metrics", get(websocket::ws_metrics_handler));

    let mut app = Router::new()
        .merge(protected_routes)
        .merge(public_routes)
        // Serve static files (React UI) - must be last to act as catch-all
        .fallback(static_files::static_handler)
        // Add tracing middleware
        .layer(TraceLayer::new_for_http());

    if enable_cors {
        app = app.layer(CorsLayer::permissive());
    }

    app
}
