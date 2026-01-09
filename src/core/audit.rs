/// Audit logging for security events and user actions
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::net::IpAddr;
use std::path::PathBuf;

/// Audit event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    LoginSuccess,
    LoginFailed,
    Logout,
    IpBlocked,
    PermissionDenied,
    UserCreated,
    UserDeleted,
    UserModified,
    PasswordChanged,
    ConfigChanged,
    ServiceStarted,
    ServiceStopped,
    ServiceRestarted,
}

impl std::fmt::Display for AuditEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditEventType::LoginSuccess => write!(f, "login_success"),
            AuditEventType::LoginFailed => write!(f, "login_failed"),
            AuditEventType::Logout => write!(f, "logout"),
            AuditEventType::IpBlocked => write!(f, "ip_blocked"),
            AuditEventType::PermissionDenied => write!(f, "permission_denied"),
            AuditEventType::UserCreated => write!(f, "user_created"),
            AuditEventType::UserDeleted => write!(f, "user_deleted"),
            AuditEventType::UserModified => write!(f, "user_modified"),
            AuditEventType::PasswordChanged => write!(f, "password_changed"),
            AuditEventType::ConfigChanged => write!(f, "config_changed"),
            AuditEventType::ServiceStarted => write!(f, "service_started"),
            AuditEventType::ServiceStopped => write!(f, "service_stopped"),
            AuditEventType::ServiceRestarted => write!(f, "service_restarted"),
        }
    }
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub timestamp: DateTime<Utc>,
    pub event: AuditEventType,
    pub username: Option<String>,
    pub ip: Option<IpAddr>,
    pub resource: Option<String>,
    pub reason: Option<String>,
    pub success: bool,
}

impl AuditEvent {
    pub fn new(event: AuditEventType) -> Self {
        Self {
            timestamp: Utc::now(),
            event,
            username: None,
            ip: None,
            resource: None,
            reason: None,
            success: true,
        }
    }

    pub fn with_username(mut self, username: String) -> Self {
        self.username = Some(username);
        self
    }

    pub fn with_ip(mut self, ip: IpAddr) -> Self {
        self.ip = Some(ip);
        self
    }

    pub fn with_resource(mut self, resource: String) -> Self {
        self.resource = Some(resource);
        self
    }

    pub fn with_reason(mut self, reason: String) -> Self {
        self.reason = Some(reason);
        self
    }

    pub fn with_success(mut self, success: bool) -> Self {
        self.success = success;
        self
    }
}

/// Audit logger
pub struct AuditLogger {
    log_path: PathBuf,
    enabled: bool,
}

impl AuditLogger {
    pub fn new(config_dir: PathBuf) -> Result<Self> {
        // Create config directory if it doesn't exist
        std::fs::create_dir_all(&config_dir).context("Failed to create audit log directory")?;

        Ok(Self {
            log_path: config_dir.join("audit.log"),
            enabled: true,
        })
    }

    pub fn new_disabled() -> Self {
        Self {
            log_path: PathBuf::new(),
            enabled: false,
        }
    }

    /// Log an audit event
    pub fn log(&self, event: AuditEvent) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        // Serialize event to JSON
        let mut json = serde_json::to_string(&event).context("Failed to serialize audit event")?;
        json.push('\n');

        // Append to log file
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .context("Failed to open audit log file")?;

        file.write_all(json.as_bytes())
            .context("Failed to write audit log")?;

        // Also log to stderr for systemd journal
        eprintln!(
            "AUDIT: {}",
            serde_json::to_string(&event).unwrap_or_default()
        );

        Ok(())
    }

    /// Read recent audit events
    pub fn read_recent(&self, limit: usize) -> Result<Vec<AuditEvent>> {
        if !self.log_path.exists() {
            return Ok(Vec::new());
        }

        let content =
            std::fs::read_to_string(&self.log_path).context("Failed to read audit log")?;

        let events: Vec<AuditEvent> = content
            .lines()
            .rev()
            .take(limit)
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();

        Ok(events)
    }

    /// Export all audit events
    pub fn export_all(&self) -> Result<Vec<AuditEvent>> {
        if !self.log_path.exists() {
            return Ok(Vec::new());
        }

        let content =
            std::fs::read_to_string(&self.log_path).context("Failed to read audit log")?;

        let events: Vec<AuditEvent> = content
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();

        Ok(events)
    }

    /// Clear audit log
    pub fn clear(&self) -> Result<()> {
        if self.log_path.exists() {
            std::fs::remove_file(&self.log_path).context("Failed to clear audit log")?;
        }
        Ok(())
    }
}

// Convenience logging functions
impl AuditLogger {
    pub fn log_login_success(&self, username: &str, ip: IpAddr) -> Result<()> {
        self.log(
            AuditEvent::new(AuditEventType::LoginSuccess)
                .with_username(username.to_string())
                .with_ip(ip)
                .with_success(true),
        )
    }

    pub fn log_login_failed(&self, username: &str, ip: IpAddr, reason: &str) -> Result<()> {
        self.log(
            AuditEvent::new(AuditEventType::LoginFailed)
                .with_username(username.to_string())
                .with_ip(ip)
                .with_reason(reason.to_string())
                .with_success(false),
        )
    }

    pub fn log_logout(&self, username: &str, ip: IpAddr) -> Result<()> {
        self.log(
            AuditEvent::new(AuditEventType::Logout)
                .with_username(username.to_string())
                .with_ip(ip)
                .with_success(true),
        )
    }

    pub fn log_ip_blocked(&self, ip: IpAddr, reason: &str) -> Result<()> {
        self.log(
            AuditEvent::new(AuditEventType::IpBlocked)
                .with_ip(ip)
                .with_reason(reason.to_string())
                .with_success(false),
        )
    }

    pub fn log_permission_denied(
        &self,
        username: &str,
        resource: &str,
        reason: &str,
    ) -> Result<()> {
        self.log(
            AuditEvent::new(AuditEventType::PermissionDenied)
                .with_username(username.to_string())
                .with_resource(resource.to_string())
                .with_reason(reason.to_string())
                .with_success(false),
        )
    }

    pub fn log_user_created(&self, admin_username: &str, new_username: &str) -> Result<()> {
        self.log(
            AuditEvent::new(AuditEventType::UserCreated)
                .with_username(admin_username.to_string())
                .with_resource(new_username.to_string())
                .with_success(true),
        )
    }

    pub fn log_user_deleted(&self, admin_username: &str, deleted_username: &str) -> Result<()> {
        self.log(
            AuditEvent::new(AuditEventType::UserDeleted)
                .with_username(admin_username.to_string())
                .with_resource(deleted_username.to_string())
                .with_success(true),
        )
    }

    pub fn log_password_changed(&self, username: &str, changed_by: &str) -> Result<()> {
        self.log(
            AuditEvent::new(AuditEventType::PasswordChanged)
                .with_username(changed_by.to_string())
                .with_resource(username.to_string())
                .with_success(true),
        )
    }

    pub fn log_config_changed(&self, username: &str, config_key: &str) -> Result<()> {
        self.log(
            AuditEvent::new(AuditEventType::ConfigChanged)
                .with_username(username.to_string())
                .with_resource(config_key.to_string())
                .with_success(true),
        )
    }

    pub fn log_service_action(
        &self,
        event_type: AuditEventType,
        username: &str,
        service_name: &str,
    ) -> Result<()> {
        self.log(
            AuditEvent::new(event_type)
                .with_username(username.to_string())
                .with_resource(service_name.to_string())
                .with_success(true),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;
    use tempfile::TempDir;

    #[test]
    fn test_audit_event_creation() {
        let event = AuditEvent::new(AuditEventType::LoginSuccess)
            .with_username("admin".to_string())
            .with_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)))
            .with_success(true);

        assert_eq!(event.username, Some("admin".to_string()));
        assert!(event.ip.is_some());
        assert!(event.success);
    }

    #[test]
    fn test_audit_logger() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let logger = AuditLogger::new(temp_dir.path().to_path_buf())?;

        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        // Log some events
        logger.log_login_success("admin", ip)?;
        logger.log_login_failed("user", ip, "invalid password")?;
        logger.log_logout("admin", ip)?;

        // Read recent events
        let events = logger.read_recent(10)?;
        assert_eq!(events.len(), 3);

        // Check event order (most recent first)
        assert!(matches!(events[0].event, AuditEventType::Logout));
        assert!(matches!(events[1].event, AuditEventType::LoginFailed));
        assert!(matches!(events[2].event, AuditEventType::LoginSuccess));

        Ok(())
    }

    #[test]
    fn test_audit_logger_export() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let logger = AuditLogger::new(temp_dir.path().to_path_buf())?;

        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        logger.log_login_success("admin", ip)?;
        logger.log_user_created("admin", "operator")?;

        let all_events = logger.export_all()?;
        assert_eq!(all_events.len(), 2);

        Ok(())
    }

    #[test]
    fn test_audit_logger_clear() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let logger = AuditLogger::new(temp_dir.path().to_path_buf())?;

        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        logger.log_login_success("admin", ip)?;

        assert_eq!(logger.read_recent(10)?.len(), 1);

        logger.clear()?;
        assert_eq!(logger.read_recent(10)?.len(), 0);

        Ok(())
    }

    #[test]
    fn test_disabled_logger() -> Result<()> {
        let logger = AuditLogger::new_disabled();

        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        logger.log_login_success("admin", ip)?;

        // Should not fail, just do nothing
        Ok(())
    }
}
