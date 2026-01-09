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

#[derive(Debug, Clone, Serialize)]
pub struct SystemResources {
    pub cpu_percent: f32,
    pub memory_used_gb: f32,
    pub memory_total_gb: f32,
    pub disk_free_gb: f32,
    pub disk_total_gb: f32,
    pub os_name: String,
    pub os_version: String,
    pub cpu_cores: usize,
    pub cpu_frequency_ghz: f32,
    pub cpu_model: String,
    pub public_ip: Option<String>,
}

pub fn collect_system_resources() -> SystemResources {
    use std::process::Command;
    use sysinfo::{CpuRefreshKind, Disks, RefreshKind, System};

    let mut sys =
        System::new_with_specifics(RefreshKind::new().with_cpu(CpuRefreshKind::everything()));

    std::thread::sleep(std::time::Duration::from_millis(200));
    sys.refresh_all();

    let cpu_percent = sys.global_cpu_info().cpu_usage();

    let memory_total_gb = sys.total_memory() as f32 / 1_073_741_824.0;
    let memory_used_gb = sys.used_memory() as f32 / 1_073_741_824.0;

    let disks = Disks::new_with_refreshed_list();
    let (disk_total_gb, disk_free_gb) = disks
        .iter()
        .next()
        .map(|disk| {
            let total = disk.total_space() as f32 / 1_000_000_000.0;
            let available = disk.available_space() as f32 / 1_000_000_000.0;
            (total, available)
        })
        .unwrap_or((0.0, 0.0));

    let os_name = System::name().unwrap_or_else(|| "Unknown".to_string());
    let os_version = System::os_version().unwrap_or_else(|| "Unknown".to_string());

    let cpus = sys.cpus();
    let cpu_cores = cpus.len();
    let cpu_model = cpus
        .first()
        .map(|cpu| cpu.brand().to_string())
        .unwrap_or_else(|| "Unknown CPU".to_string());
    let cpu_frequency_ghz = cpus
        .first()
        .map(|cpu| cpu.frequency() as f32 / 1000.0)
        .unwrap_or(0.0);

    let public_ip = Command::new("curl")
        .args(["-s", "--max-time", "2", "https://api.ipify.org"])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .filter(|ip| !ip.is_empty() && ip.len() < 50);

    SystemResources {
        cpu_percent,
        memory_used_gb,
        memory_total_gb,
        disk_free_gb,
        disk_total_gb,
        os_name,
        os_version,
        cpu_cores,
        cpu_frequency_ghz,
        cpu_model,
        public_ip,
    }
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
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await?;
    }

    Ok(())
}
