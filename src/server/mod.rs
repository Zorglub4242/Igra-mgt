/// HTTP API Server module for IGRA CLI
/// Provides REST API endpoints that reuse core business logic

#[cfg(feature = "server")]
pub mod routes;

#[cfg(feature = "server")]
pub mod handlers;

#[cfg(feature = "server")]
pub mod websocket;

#[cfg(feature = "server")]
pub mod static_files;

#[cfg(feature = "server")]
pub mod auth;

#[cfg(feature = "server")]
pub mod auth_backend;

#[cfg(feature = "server")]
pub mod auth_handlers;

#[cfg(feature = "server")]
pub mod system_service_handlers;

#[cfg(feature = "server")]
pub use routes::create_router;

// Common API response type
use serde::Serialize;

#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

#[cfg(feature = "server")]
pub async fn run(
    host: String,
    port: u16,
    enable_cors: bool,
    tls_cert: Option<String>,
    tls_key: Option<String>,
) -> anyhow::Result<()> {
    use std::net::SocketAddr;
    use std::io::{self, Write};

    // Session-based authentication is now handled by axum-login
    // Default admin user will be created if no users exist

    let app = create_router(enable_cors);
    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;

    // Check if TLS is configured
    let use_tls = tls_cert.is_some() && tls_key.is_some();

    println!("🚀 IGRA Management Server");
    if use_tls {
        println!("   📍 Web UI: https://{}", addr);
        println!("   🔌 API:    https://{}/api", addr);
        println!("   🔒 TLS:    Enabled");
    } else {
        println!("   📍 Web UI: http://{}", addr);
        println!("   🔌 API:    http://{}/api", addr);
    }

    println!("   🔐 Auth:   Session-based authentication enabled");
    println!();
    println!("   📝 Login:  POST /api/auth/login (username/password)");
    println!("   👤 Default: admin / admin");
    println!("   ⚠️  Change the default password after first login!");
    println!();

    if use_tls {
        // HTTPS mode with TLS
        let cert_path = tls_cert.unwrap();
        let key_path = tls_key.unwrap();

        println!("📜 Loading TLS certificates...");
        println!("   Certificate: {}", cert_path);
        println!("   Private Key: {}", key_path);

        use axum_server::tls_rustls::RustlsConfig;

        let config = RustlsConfig::from_pem_file(&cert_path, &key_path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to load TLS certificates: {}", e))?;

        println!("✓ TLS certificates loaded successfully");
        println!();

        axum_server::bind_rustls(addr, config)
            .serve(app.into_make_service_with_connect_info::<std::net::SocketAddr>())
            .await?;
    } else {
        // HTTP mode without TLS
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>()
        ).await?;
    }

    Ok(())
}
