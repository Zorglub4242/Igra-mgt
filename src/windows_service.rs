/// Windows Service integration for igra-cli web server
///
/// This module provides Windows Service support, allowing igra-cli serve
/// to run as a background Windows Service similar to systemd on Linux.

#[cfg(windows)]
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
};

use std::ffi::OsString;
use std::sync::mpsc;
use std::time::Duration;
use anyhow::Result;

/// Service name for Windows Service Manager
pub const SERVICE_NAME: &str = "IgraWebUI";

/// Display name shown in Windows Services
pub const SERVICE_DISPLAY_NAME: &str = "IGRA Orchestra Web Management UI";

/// Service description
pub const SERVICE_DESCRIPTION: &str = "Web-based management interface for IGRA Orchestra blockchain nodes";

#[cfg(windows)]
define_windows_service!(ffi_service_main, service_main);

/// Main service entry point called by Windows Service Manager
#[cfg(windows)]
fn service_main(_arguments: Vec<OsString>) {
    if let Err(e) = run_service() {
        // Log error to Windows Event Log
        eprintln!("Service error: {}", e);
    }
}

/// Run the service with proper state management
#[cfg(windows)]
fn run_service() -> Result<()> {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    // Create a channel to receive shutdown signals
    let (shutdown_tx, shutdown_rx) = mpsc::channel();

    // Service is running flag
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    // Define the service control handler
    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Stop | ServiceControl::Shutdown => {
                r.store(false, Ordering::SeqCst);
                shutdown_tx.send(()).ok();
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    // Register the service control handler
    let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;

    // Tell Windows we're starting
    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::StartPending,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::from_secs(5),
        process_id: None,
    })?;

    // Read configuration from environment or registry
    let port = std::env::var("IGRA_WEB_PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(3000);

    let host = std::env::var("IGRA_WEB_HOST")
        .unwrap_or_else(|_| "0.0.0.0".to_string());

    let cors = std::env::var("IGRA_WEB_CORS")
        .ok()
        .and_then(|c| c.parse::<bool>().ok())
        .unwrap_or(false);

    // Create tokio runtime for async server
    let runtime = tokio::runtime::Runtime::new()?;

    // Tell Windows we're running
    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    // Start the web server in the runtime
    let server_handle = runtime.spawn(async move {
        #[cfg(feature = "server")]
        {
            use crate::server;
            let _ = server::run(host, port, cors, None, None).await;
        }
    });

    // Wait for shutdown signal
    shutdown_rx.recv().ok();
    running.store(false, Ordering::SeqCst);

    // Tell Windows we're stopping
    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::StopPending,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::from_secs(5),
        process_id: None,
    })?;

    // Shutdown the server gracefully
    server_handle.abort();
    runtime.shutdown_timeout(Duration::from_secs(10));

    // Tell Windows we're stopped
    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    Ok(())
}

/// Dispatch to Windows Service Manager
/// This should be called from main() when running as a service
#[cfg(windows)]
pub fn dispatch_service() -> Result<()> {
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)
        .map_err(|e| anyhow::anyhow!("Failed to start service dispatcher: {}", e))
}

/// Check if running as Windows Service
#[cfg(windows)]
pub fn is_running_as_service() -> bool {
    // Check if parent process is services.exe
    // This is a heuristic - not 100% accurate but works for most cases
    use std::env;
    env::var("IGRA_SERVICE_MODE").is_ok()
}

/// Install the Windows Service
#[cfg(windows)]
pub fn install_service(
    binary_path: &str,
    port: u16,
    host: &str,
    cors: bool,
    token: &str,
) -> Result<()> {
    use std::process::Command;

    // Build service command line
    let command = format!(
        "\"{}\" serve --host {} --port {} {}",
        binary_path,
        host,
        port,
        if cors { "--cors" } else { "" }
    );

    // Create service using sc.exe
    let output = Command::new("sc")
        .args(&[
            "create",
            SERVICE_NAME,
            format!("binPath= {}", command).as_str(),
            format!("DisplayName= {}", SERVICE_DISPLAY_NAME).as_str(),
            "start= auto",
        ])
        .output()?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to create service: {}", error);
    }

    // Set service description
    Command::new("sc")
        .args(&[
            "description",
            SERVICE_NAME,
            SERVICE_DESCRIPTION,
        ])
        .output()?;

    // Configure environment variables via registry
    // Environment variables are stored in:
    // HKLM\SYSTEM\CurrentControlSet\Services\<ServiceName>\Environment
    set_service_env_var("IGRA_WEB_TOKEN", token)?;
    set_service_env_var("IGRA_WEB_PORT", &port.to_string())?;
    set_service_env_var("IGRA_WEB_HOST", host)?;
    set_service_env_var("IGRA_WEB_CORS", if cors { "true" } else { "false" })?;
    set_service_env_var("IGRA_SERVICE_MODE", "1")?;

    println!("✓ Service installed successfully");
    println!("\nTo start the service, run:");
    println!("  sc start {}", SERVICE_NAME);
    println!("\nOr use Windows Services Manager (services.msc)");

    Ok(())
}

/// Uninstall the Windows Service
#[cfg(windows)]
pub fn uninstall_service() -> Result<()> {
    use std::process::Command;

    // Stop the service first if running
    let _ = Command::new("sc")
        .args(&["stop", SERVICE_NAME])
        .output();

    // Delete the service
    let output = Command::new("sc")
        .args(&["delete", SERVICE_NAME])
        .output()?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to delete service: {}", error);
    }

    println!("✓ Service uninstalled successfully");
    Ok(())
}

/// Set environment variable for the service via registry
#[cfg(windows)]
fn set_service_env_var(name: &str, value: &str) -> Result<()> {
    use std::process::Command;

    let key = format!("HKLM\\SYSTEM\\CurrentControlSet\\Services\\{}\\Environment", SERVICE_NAME);

    let output = Command::new("reg")
        .args(&[
            "add",
            &key,
            "/v",
            name,
            "/t",
            "REG_SZ",
            "/d",
            value,
            "/f",
        ])
        .output()?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        eprintln!("Warning: Failed to set environment variable {}: {}", name, error);
    }

    Ok(())
}

// Stub implementations for non-Windows platforms
#[cfg(not(windows))]
pub fn dispatch_service() -> Result<()> {
    anyhow::bail!("Windows Service support is only available on Windows")
}

#[cfg(not(windows))]
pub fn is_running_as_service() -> bool {
    false
}

#[cfg(not(windows))]
pub fn install_service(_binary_path: &str, _port: u16, _host: &str, _cors: bool, _token: &str) -> Result<()> {
    anyhow::bail!("Windows Service support is only available on Windows")
}

#[cfg(not(windows))]
pub fn uninstall_service() -> Result<()> {
    anyhow::bail!("Windows Service support is only available on Windows")
}
