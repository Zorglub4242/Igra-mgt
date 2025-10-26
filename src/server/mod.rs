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
pub use routes::create_router;

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

    // Check if IGRA_WEB_TOKEN is set, prompt if not
    if std::env::var("IGRA_WEB_TOKEN").is_err() {
        println!("⚠️  IGRA_WEB_TOKEN environment variable not set!");
        println!("    This token is required for API authentication.");
        println!();
        print!("Enter a secure token (or press Enter to continue without auth): ");
        io::stdout().flush()?;

        let mut token = String::new();
        io::stdin().read_line(&mut token)?;
        let token = token.trim();

        if !token.is_empty() {
            std::env::set_var("IGRA_WEB_TOKEN", token);
            println!("✓ Token set for this session");
            println!("  To persist, add to your environment: export IGRA_WEB_TOKEN=\"{}\"", token);
            println!();
        } else {
            println!("⚠️  Starting without authentication - API will be open!");
            println!();
        }
    }

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

    if std::env::var("IGRA_WEB_TOKEN").is_ok() {
        println!("   🔒 Auth:   Enabled (token required)");
    } else {
        println!("   ⚠️  Auth:   Disabled (no token)");
    }

    println!();
    println!("📚 API Endpoints:");
    println!("   GET  /api/services               - List all services");
    println!("   POST /api/services/:name/start   - Start service");
    println!("   POST /api/services/:name/stop    - Stop service");
    println!("   POST /api/services/:name/restart - Restart service");
    println!("   GET  /api/services/:name/logs    - Get service logs");
    println!("   GET  /api/wallets                - List wallets");
    println!("   GET  /api/storage                - Get storage info");
    println!("   GET  /api/config                 - Get configuration");
    println!("   GET  /api/version                - Check for updates");
    println!("   GET  /api/health                 - Health check");
    println!("   GET  /ws/logs/:service           - WebSocket log stream");
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
            .serve(app.into_make_service())
            .await?;
    } else {
        // HTTP mode without TLS
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;
    }

    Ok(())
}
