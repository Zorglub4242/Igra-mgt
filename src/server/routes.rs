/// API Routes definition with session-based authentication
use axum::{
    routing::{delete, get, post, put},
    Router,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tower_sessions::cookie::time::Duration;
use tower_sessions::{Expiry, SessionManagerLayer};
use tower_sessions_memory_store::MemoryStore;

use super::auth_backend::FileAuthBackend;
use super::auth_handlers::{self, AuthState};
use super::handlers;
use super::static_files;
use super::system_service_handlers;
use super::websocket;

use crate::core::{
    AuditLogger, CategoryManager, SecurityManager, SystemServiceManager, UserManager,
};

#[cfg(feature = "server")]
use axum_login::{login_required, AuthManagerLayerBuilder};

pub fn create_router(enable_cors: bool) -> Router {
    // Initialize config directory
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("igra-cli");

    // Create config directory if needed
    std::fs::create_dir_all(&config_dir).expect("Failed to create config directory");

    // Initialize managers
    let system_service_mgr = Arc::new(RwLock::new(SystemServiceManager::new(true))); // use sudo
    let category_mgr = Arc::new(RwLock::new(
        CategoryManager::new(config_dir.clone()).unwrap_or_else(|e| {
            eprintln!("Failed to initialize CategoryManager: {}", e);
            std::process::exit(1);
        }),
    ));

    let user_manager =
        UserManager::new(config_dir.clone()).expect("Failed to initialize UserManager");

    let security_manager = Arc::new(RwLock::new(
        SecurityManager::new(config_dir.clone()).expect("Failed to initialize SecurityManager"),
    ));

    let audit_logger = Arc::new(RwLock::new(
        AuditLogger::new(config_dir.clone()).expect("Failed to initialize AuditLogger"),
    ));

    // Ensure at least one user exists (create default admin if needed)
    #[cfg(feature = "server")]
    {
        if !user_manager.has_users().unwrap_or(false) {
            eprintln!("⚠️  No users found. Creating default admin user...");
            eprintln!("   Default password: admin");
            eprintln!("   ⚠️  CHANGE THIS PASSWORD IMMEDIATELY!");

            if let Err(e) = user_manager.ensure_default_admin("admin") {
                eprintln!("Failed to create default admin user: {}", e);
                std::process::exit(1);
            }

            eprintln!("✓ Default admin user created");
        }
    }

    // Create auth backend
    let auth_backend = FileAuthBackend::new(user_manager);

    // Session store (in-memory)
    // NOTE: File-based sessions (FileStore) cause crashes - needs investigation
    // Sessions will be lost on server restart, but that's acceptable for development
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false) // Set to true in production with HTTPS
        .with_expiry(Expiry::OnInactivity(Duration::hours(24)));

    // Auth manager layer
    #[cfg(feature = "server")]
    let auth_layer = AuthManagerLayerBuilder::new(auth_backend, session_layer).build();

    // Auth state for handlers
    let auth_state = AuthState {
        audit_logger: audit_logger.clone(),
        security_manager: security_manager.clone(),
    };

    // Public routes (no authentication required)
    let public_routes = Router::new()
        .route("/api/health", get(handlers::health_check))
        .route("/api/version", get(handlers::get_version_info))
        // Authentication routes inline (public)
        .route("/api/auth/login", post(auth_handlers::login_handler))
        .route("/api/auth/logout", post(auth_handlers::logout_handler))
        .route(
            "/api/auth/session",
            get(auth_handlers::session_info_handler),
        )
        .route(
            "/api/auth/change-password",
            post(auth_handlers::change_password),
        )
        .with_state(auth_state.clone());

    // Protected routes (require authentication via session)
    #[cfg(feature = "server")]
    let protected_routes = Router::new()
        // Service management
        .route("/api/services", get(handlers::get_services))
        .route(
            "/api/services/{name}/details",
            get(handlers::get_service_details),
        )
        .route("/api/services/{name}/note", get(handlers::get_service_note))
        .route("/api/services/{name}/logs", get(handlers::get_logs))
        .route(
            "/api/services/{name}/logs/parsed",
            get(handlers::get_logs_parsed),
        )
        .route("/api/services/{name}/start", post(handlers::start_service))
        .route("/api/services/{name}/stop", post(handlers::stop_service))
        .route(
            "/api/services/{name}/restart",
            post(handlers::restart_service),
        )
        .route(
            "/api/services/{name}/note",
            put(handlers::update_service_note),
        )
        // Network topology
        .route("/api/network-topology", get(handlers::get_network_topology))
        // Wallet management
        .route("/api/wallets", get(handlers::get_wallets))
        .route(
            "/api/wallets/{id}/balance",
            get(handlers::get_wallet_balance),
        )
        .route("/api/wallets/{id}/detail", get(handlers::get_wallet_detail))
        // Storage management
        .route("/api/storage", get(handlers::get_storage))
        .route("/api/storage/history", get(handlers::get_storage_history))
        .route("/api/storage/prune", post(handlers::prune_storage))
        .route(
            "/api/storage/container-logs/{container_id}/truncate",
            post(handlers::truncate_container_log),
        )
        .route(
            "/api/storage/log-rotation",
            get(handlers::get_log_rotation_config),
        )
        .route(
            "/api/storage/log-rotation/global",
            put(handlers::update_global_log_rotation),
        )
        .route(
            "/api/storage/log-rotation/container/{name}",
            get(handlers::get_container_log_rotation),
        )
        .route(
            "/api/storage/log-rotation/container/{name}",
            put(handlers::update_container_log_rotation),
        )
        .route(
            "/api/storage/log-rotation/container/{name}",
            delete(handlers::delete_container_log_rotation),
        )
        // Configuration
        .route("/api/config", get(handlers::get_config))
        .route("/api/system", get(handlers::get_system_info))
        .route("/api/rpc/tokens", get(handlers::get_rpc_tokens))
        .route("/api/ssl/info", get(handlers::get_ssl_info))
        // Profiles
        .route("/api/profiles", get(handlers::get_profiles))
        .route("/api/profiles/{name}/start", post(handlers::start_profile))
        .route("/api/profiles/{name}/stop", post(handlers::stop_profile))
        // Transactions
        .route("/api/transactions", get(handlers::get_transactions))
        .route(
            "/api/transactions/stats",
            get(handlers::get_transaction_stats),
        )
        // Metrics
        .route("/api/metrics", get(handlers::get_metrics))
        // Updates
        .route("/api/update", post(handlers::trigger_update))
        .route("/api/service/restart", post(handlers::restart_igra_service))
        // WebSocket connections
        .route("/ws/logs/{service}", get(websocket::ws_logs_handler))
        .route("/ws/metrics", get(websocket::ws_metrics_handler))
        // System services
        .route(
            "/api/system-services",
            get(system_service_handlers::list_system_services),
        )
        .route(
            "/api/system-services/available",
            get(system_service_handlers::list_available_services),
        )
        .route(
            "/api/system-services/{name}/details",
            get(system_service_handlers::get_service_details),
        )
        .route(
            "/api/system-services/{name}/logs",
            get(system_service_handlers::get_service_logs),
        )
        .route(
            "/api/system-services/{name}/start",
            post(system_service_handlers::start_service),
        )
        .route(
            "/api/system-services/{name}/stop",
            post(system_service_handlers::stop_service),
        )
        .route(
            "/api/system-services/{name}/restart",
            post(system_service_handlers::restart_service),
        )
        .route(
            "/api/system-services/{name}/enable",
            post(system_service_handlers::enable_service),
        )
        .route(
            "/api/system-services/{name}/disable",
            post(system_service_handlers::disable_service),
        )
        // Categories
        .route(
            "/api/categories",
            get(system_service_handlers::list_categories),
        )
        .route(
            "/api/categories/{id}",
            get(system_service_handlers::get_category),
        )
        .route(
            "/api/categories",
            post(system_service_handlers::create_category),
        )
        .route(
            "/api/categories/{id}",
            put(system_service_handlers::update_category),
        )
        .route(
            "/api/categories/{id}",
            delete(system_service_handlers::delete_category),
        )
        .route(
            "/api/categories/{id}/services",
            post(system_service_handlers::add_service_to_category),
        )
        // Tracked services
        .route(
            "/api/tracked-services",
            get(system_service_handlers::get_tracked_services),
        )
        .route(
            "/api/tracked-services/{name}",
            put(system_service_handlers::update_tracked_service),
        )
        .route(
            "/api/tracked-services/{name}",
            delete(system_service_handlers::remove_tracked_service),
        )
        // User management (admin only)
        .route("/api/users", get(handlers::get_users))
        .route("/api/users", post(handlers::add_user))
        .route("/api/users/{username}", delete(handlers::delete_user))
        .route(
            "/api/users/{username}/password",
            put(handlers::reset_user_password),
        )
        .route(
            "/api/users/{username}/roles",
            put(handlers::update_user_roles),
        )
        // Security management (admin only)
        .route("/api/security", get(handlers::get_security_config))
        .route("/api/security/ips", post(handlers::add_allowed_network))
        .route(
            "/api/security/ips/{network}",
            delete(handlers::remove_allowed_network),
        )
        // Audit logs (admin only)
        .route("/api/audit", get(handlers::get_audit_logs))
        .route("/api/audit/export", get(handlers::export_audit_logs))
        .with_state((system_service_mgr.clone(), category_mgr.clone()))
        .route_layer(login_required!(FileAuthBackend, login_url = "/login"));

    #[cfg(not(feature = "server"))]
    let protected_routes = Router::new();

    // Build final app
    #[cfg(feature = "server")]
    let api_routes = Router::new().merge(public_routes).merge(protected_routes);

    #[cfg(feature = "server")]
    let mut app = Router::new()
        .merge(api_routes)
        // Serve static files (React UI) - must be last to act as catch-all
        .fallback(static_files::static_handler)
        .layer(auth_layer) // Auth layer provides session management for all routes
        .layer(TraceLayer::new_for_http());

    #[cfg(not(feature = "server"))]
    let mut app = Router::new()
        .merge(public_routes)
        .fallback(static_files::static_handler)
        .layer(TraceLayer::new_for_http());

    if enable_cors {
        app = app.layer(CorsLayer::permissive());
    }

    app
}
