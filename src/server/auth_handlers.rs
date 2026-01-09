/// Authentication handlers for login, logout, and session management

#[cfg(feature = "server")]
use axum::{
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode},
    response::Json,
};
#[cfg(feature = "server")]
use axum_login::AuthSession;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::auth_backend::{AuthenticatedUser, Credentials, FileAuthBackend};
use super::ApiResponse;
use crate::core::{AuditLogger, SecurityManager};

/// Login request
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Login response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub username: String,
    pub roles: Vec<String>,
    pub force_password_change: bool,
}

/// Session info response
#[derive(Debug, Serialize)]
pub struct SessionInfo {
    pub authenticated: bool,
    pub username: Option<String>,
    pub roles: Option<Vec<String>>,
    pub force_password_change: Option<bool>,
}

/// Shared state for auth handlers
#[derive(Clone)]
pub struct AuthState {
    pub audit_logger: Arc<RwLock<AuditLogger>>,
    pub security_manager: Arc<RwLock<SecurityManager>>,
}

/// Login handler
#[cfg(feature = "server")]
pub async fn login_handler(
    mut auth_session: AuthSession<FileAuthBackend>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AuthState>,
    Json(login_req): Json<LoginRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, StatusCode> {
    let ip = addr.ip();

    // Check IP allowlist
    let security_mgr = state.security_manager.read().await;
    let allowlist = security_mgr
        .load_config()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Extract real IP if behind proxy
    let real_ip = if allowlist.trust_proxy {
        let proxy_header = headers
            .get(&allowlist.proxy_header)
            .and_then(|v| v.to_str().ok());
        crate::core::security::extract_real_ip(ip, proxy_header, true)
    } else {
        ip
    };

    // Check if IP is allowed
    if !allowlist
        .is_allowed(real_ip)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        // Log blocked IP
        let audit = state.audit_logger.read().await;
        let _ = audit.log_ip_blocked(real_ip, "IP not in allowlist");

        return Err(StatusCode::FORBIDDEN);
    }

    drop(security_mgr);

    // Create credentials
    let creds = Credentials {
        username: login_req.username.clone(),
        password: login_req.password,
    };

    // Attempt authentication
    let user = match auth_session.authenticate(creds).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            // Authentication failed
            let audit = state.audit_logger.read().await;
            let _ = audit.log_login_failed(&login_req.username, real_ip, "invalid credentials");

            return Err(StatusCode::UNAUTHORIZED);
        }
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    // Log in the user (create session)
    if let Err(_) = auth_session.login(&user).await {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Log successful login
    let audit = state.audit_logger.read().await;
    let _ = audit.log_login_success(&user.username, real_ip);

    // Return user info
    let roles: Vec<String> = user.roles.iter().map(|r| r.to_string()).collect();
    let force_password_change = user.force_password_change;

    Ok(Json(ApiResponse::ok(LoginResponse {
        username: user.username,
        roles,
        force_password_change,
    })))
}

/// Logout handler
#[cfg(feature = "server")]
pub async fn logout_handler(
    mut auth_session: AuthSession<FileAuthBackend>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AuthState>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    let ip = addr.ip();

    // Get username before logout
    let username = auth_session
        .user
        .as_ref()
        .map(|u| u.username.clone())
        .unwrap_or_else(|| "unknown".to_string());

    // Logout (destroy session)
    auth_session
        .logout()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Log logout
    let audit = state.audit_logger.read().await;
    let _ = audit.log_logout(&username, ip);

    Ok(Json(ApiResponse::ok(())))
}

/// Session info handler (check if authenticated)
#[cfg(feature = "server")]
pub async fn session_info_handler(
    auth_session: AuthSession<FileAuthBackend>,
) -> Json<ApiResponse<SessionInfo>> {
    if let Some(user) = auth_session.user {
        let roles: Vec<String> = user.roles.iter().map(|r| r.to_string()).collect();

        Json(ApiResponse::ok(SessionInfo {
            authenticated: true,
            username: Some(user.username.clone()),
            roles: Some(roles),
            force_password_change: Some(user.force_password_change),
        }))
    } else {
        Json(ApiResponse::ok(SessionInfo {
            authenticated: false,
            username: None,
            roles: None,
            force_password_change: None,
        }))
    }
}

/// Require authentication middleware extractor
/// Use this in protected route handlers to get the authenticated user
#[cfg(feature = "server")]
pub async fn require_auth(
    auth_session: AuthSession<FileAuthBackend>,
) -> Result<AuthenticatedUser, StatusCode> {
    auth_session.user.ok_or(StatusCode::UNAUTHORIZED)
}

/// Require specific role
#[cfg(feature = "server")]
pub fn require_role(
    user: &AuthenticatedUser,
    required_roles: &[crate::core::Role],
) -> Result<(), StatusCode> {
    if user.roles.iter().any(|r| required_roles.contains(r)) {
        Ok(())
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}

/// Require admin role
#[cfg(feature = "server")]
pub fn require_admin(user: &AuthenticatedUser) -> Result<(), StatusCode> {
    if user.is_admin() {
        Ok(())
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}

/// Change password request (self-service)
#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

/// POST /api/auth/change-password - Change own password (authenticated users)
#[cfg(feature = "server")]
pub async fn change_password(
    mut auth_session: AuthSession<FileAuthBackend>,
    State(state): State<AuthState>,
    Json(payload): Json<ChangePasswordRequest>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    // Get authenticated user
    let user = auth_session.user.as_ref().ok_or(StatusCode::UNAUTHORIZED)?;

    // Verify current password
    let credentials = Credentials {
        username: user.username.clone(),
        password: payload.current_password.clone(),
    };

    let verified_user = auth_session
        .authenticate(credentials)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    if verified_user.is_none() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Change password using user manager
    // Get config directory for user manager
    let config_dir = dirs::config_dir()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
        .join("igra-cli");

    let user_manager =
        crate::core::UserManager::new(config_dir).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Hash the new password using argon2
    let password_hash = crate::core::user_manager::hash_password(&payload.new_password)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    user_manager
        .update_password(&user.username, password_hash)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Clear force_password_change flag after successful password change
    user_manager
        .clear_force_password_change(&user.username)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Log the password change
    let audit = state.audit_logger.read().await;
    let _ = audit.log_password_changed(&user.username, &user.username);

    // Logout current session to force re-login with new password
    auth_session
        .logout()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse::ok(
        "Password changed successfully".to_string(),
    )))
}
