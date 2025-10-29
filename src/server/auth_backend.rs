/// Authentication backend using axum-login with file-based user storage

#[cfg(feature = "server")]
use axum_login::{AuthUser, AuthnBackend, UserId};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::core::{user_manager, Role, UserManager};

/// Authenticated user for axum-login
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatedUser {
    pub id: String,
    pub username: String,
    pub roles: HashSet<Role>,
    pub force_password_change: bool,
}

impl AuthenticatedUser {
    pub fn has_role(&self, role: &Role) -> bool {
        self.roles.contains(role)
    }

    pub fn is_admin(&self) -> bool {
        self.has_role(&Role::Admin)
    }

    pub fn can_control_services(&self) -> bool {
        self.roles.iter().any(|r| r.can_control_services())
    }

    pub fn can_configure(&self) -> bool {
        self.roles.iter().any(|r| r.can_configure())
    }

    pub fn can_manage_users(&self) -> bool {
        self.roles.iter().any(|r| r.can_manage_users())
    }
}

#[cfg(feature = "server")]
impl AuthUser for AuthenticatedUser {
    type Id = String;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }

    fn session_auth_hash(&self) -> &[u8] {
        self.username.as_bytes()
    }
}

/// Credentials for login
#[derive(Debug, Clone, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

/// File-based authentication backend
#[derive(Debug, Clone)]
pub struct FileAuthBackend {
    user_manager: UserManager,
}

impl FileAuthBackend {
    pub fn new(user_manager: UserManager) -> Self {
        Self { user_manager }
    }
}

#[cfg(feature = "server")]
impl AuthnBackend for FileAuthBackend {
    type User = AuthenticatedUser;
    type Credentials = Credentials;
    type Error = std::convert::Infallible;

    async fn authenticate(
        &self,
        creds: Self::Credentials,
    ) -> Result<Option<Self::User>, Self::Error> {
        // Get user from storage
        let user = match self.user_manager.get_user(&creds.username) {
            Ok(Some(u)) => u,
            Ok(None) | Err(_) => return Ok(None), // User not found or error
        };

        // Check if user is enabled
        if !user.enabled {
            return Ok(None);
        }

        // Verify password
        let password_valid = user_manager::verify_password(&creds.password, &user.password_hash)
            .unwrap_or(false);

        if !password_valid {
            return Ok(None);
        }

        // Authentication successful
        Ok(Some(AuthenticatedUser {
            id: user.username.clone(),
            username: user.username,
            roles: user.roles,
            force_password_change: user.force_password_change,
        }))
    }

    async fn get_user(&self, user_id: &UserId<Self>) -> Result<Option<Self::User>, Self::Error> {
        // Load user from storage
        let user = match self.user_manager.get_user(user_id) {
            Ok(Some(u)) => u,
            Ok(None) | Err(_) => return Ok(None),
        };

        // Check if user is enabled
        if !user.enabled {
            return Ok(None);
        }

        Ok(Some(AuthenticatedUser {
            id: user.username.clone(),
            username: user.username,
            roles: user.roles,
            force_password_change: user.force_password_change,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::user_manager::{hash_password, User};
    use tempfile::TempDir;

    #[tokio::test]
    #[cfg(feature = "server")]
    async fn test_authentication() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let user_manager = UserManager::new(temp_dir.path().to_path_buf())?;

        // Create test user
        let password = "test_password";
        let password_hash = hash_password(password)?;
        let mut roles = HashSet::new();
        roles.insert(Role::Admin);

        let user = User::new("testuser".to_string(), password_hash, roles);
        user_manager.add_user(user)?;

        // Create backend
        let backend = FileAuthBackend::new(user_manager);

        // Test successful authentication
        let creds = Credentials {
            username: "testuser".to_string(),
            password: password.to_string(),
        };

        let result = backend.authenticate(creds).await.ok();
        assert!(result.is_some());

        let auth_user = result.unwrap().unwrap();
        assert_eq!(auth_user.username, "testuser");
        assert!(auth_user.is_admin());

        // Test failed authentication (wrong password)
        let creds = Credentials {
            username: "testuser".to_string(),
            password: "wrong_password".to_string(),
        };

        let result = backend.authenticate(creds).await.ok().unwrap();
        assert!(result.is_none());

        // Test failed authentication (user not found)
        let creds = Credentials {
            username: "nonexistent".to_string(),
            password: "password".to_string(),
        };

        let result = backend.authenticate(creds).await.ok().unwrap();
        assert!(result.is_none());

        Ok(())
    }

    #[tokio::test]
    #[cfg(feature = "server")]
    async fn test_get_user() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let user_manager = UserManager::new(temp_dir.path().to_path_buf())?;

        // Create test user
        let password_hash = hash_password("password")?;
        let mut roles = HashSet::new();
        roles.insert(Role::Operator);

        let user = User::new("operator".to_string(), password_hash, roles);
        user_manager.add_user(user)?;

        // Create backend
        let backend = FileAuthBackend::new(user_manager);

        // Test get_user
        let result = backend.get_user(&"operator".to_string()).await.ok().unwrap();
        assert!(result.is_some());

        let auth_user = result.unwrap();
        assert_eq!(auth_user.username, "operator");
        assert!(auth_user.can_control_services());
        assert!(!auth_user.can_configure());

        Ok(())
    }

    #[test]
    fn test_authenticated_user_permissions() {
        let mut admin_roles = HashSet::new();
        admin_roles.insert(Role::Admin);

        let admin = AuthenticatedUser {
            id: "admin".to_string(),
            username: "admin".to_string(),
            roles: admin_roles,
            force_password_change: false,
        };

        assert!(admin.is_admin());
        assert!(admin.can_control_services());
        assert!(admin.can_configure());
        assert!(admin.can_manage_users());

        let mut operator_roles = HashSet::new();
        operator_roles.insert(Role::Operator);

        let operator = AuthenticatedUser {
            id: "operator".to_string(),
            username: "operator".to_string(),
            roles: operator_roles,
            force_password_change: false,
        };

        assert!(!operator.is_admin());
        assert!(operator.can_control_services());
        assert!(!operator.can_configure());
        assert!(!operator.can_manage_users());
    }
}
