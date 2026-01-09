/// User management with file-based storage and role-based access control
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

#[cfg(feature = "server")]
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

/// User role for RBAC
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,    // Full access
    Operator, // Service control + read
    Viewer,   // Read-only
}

impl Role {
    pub fn can_read(&self) -> bool {
        matches!(self, Role::Admin | Role::Operator | Role::Viewer)
    }

    pub fn can_control_services(&self) -> bool {
        matches!(self, Role::Admin | Role::Operator)
    }

    pub fn can_configure(&self) -> bool {
        matches!(self, Role::Admin)
    }

    pub fn can_manage_users(&self) -> bool {
        matches!(self, Role::Admin)
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Admin => write!(f, "admin"),
            Role::Operator => write!(f, "operator"),
            Role::Viewer => write!(f, "viewer"),
        }
    }
}

impl std::str::FromStr for Role {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "admin" => Ok(Role::Admin),
            "operator" => Ok(Role::Operator),
            "viewer" => Ok(Role::Viewer),
            _ => Err(anyhow::anyhow!("Invalid role: {}", s)),
        }
    }
}

/// User account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub username: String,
    pub password_hash: String,
    pub roles: HashSet<Role>,
    pub enabled: bool,
    #[serde(default)]
    pub force_password_change: bool,
}

impl User {
    pub fn new(username: String, password_hash: String, roles: HashSet<Role>) -> Self {
        Self {
            username,
            password_hash,
            roles,
            enabled: true,
            force_password_change: false,
        }
    }

    pub fn has_role(&self, role: &Role) -> bool {
        self.roles.contains(role)
    }

    pub fn has_any_role(&self, roles: &[Role]) -> bool {
        roles.iter().any(|r| self.roles.contains(r))
    }

    pub fn is_admin(&self) -> bool {
        self.has_role(&Role::Admin)
    }
}

/// Users container for file storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsersFile {
    pub users: Vec<User>,
}

/// User manager with file-based storage
#[derive(Debug, Clone)]
pub struct UserManager {
    config_path: PathBuf,
}

impl UserManager {
    pub fn new(config_dir: PathBuf) -> Result<Self> {
        // Create config directory if it doesn't exist
        std::fs::create_dir_all(&config_dir).context("Failed to create user config directory")?;

        Ok(Self {
            config_path: config_dir.join("users.yaml"),
        })
    }

    /// Load users from file
    pub fn load_users(&self) -> Result<Vec<User>> {
        if !self.config_path.exists() {
            return Ok(Vec::new());
        }

        let content =
            std::fs::read_to_string(&self.config_path).context("Failed to read users file")?;

        let users_file: UsersFile =
            serde_yaml::from_str(&content).context("Failed to parse users file")?;

        Ok(users_file.users)
    }

    /// Save users to file
    pub fn save_users(&self, users: &[User]) -> Result<()> {
        let users_file = UsersFile {
            users: users.to_vec(),
        };

        let content = serde_yaml::to_string(&users_file).context("Failed to serialize users")?;

        std::fs::write(&self.config_path, content).context("Failed to write users file")?;

        Ok(())
    }

    /// Get user by username
    pub fn get_user(&self, username: &str) -> Result<Option<User>> {
        let users = self.load_users()?;
        Ok(users.into_iter().find(|u| u.username == username))
    }

    /// Add new user
    pub fn add_user(&self, user: User) -> Result<()> {
        let mut users = self.load_users()?;

        // Check if user already exists
        if users.iter().any(|u| u.username == user.username) {
            return Err(anyhow::anyhow!("User '{}' already exists", user.username));
        }

        users.push(user);
        self.save_users(&users)?;

        Ok(())
    }

    /// Remove user
    pub fn remove_user(&self, username: &str) -> Result<()> {
        let mut users = self.load_users()?;
        let initial_len = users.len();

        users.retain(|u| u.username != username);

        if users.len() == initial_len {
            return Err(anyhow::anyhow!("User '{}' not found", username));
        }

        self.save_users(&users)?;

        Ok(())
    }

    /// Update user
    pub fn update_user(&self, username: &str, updated_user: User) -> Result<()> {
        let mut users = self.load_users()?;

        let user = users
            .iter_mut()
            .find(|u| u.username == username)
            .ok_or_else(|| anyhow::anyhow!("User '{}' not found", username))?;

        *user = updated_user;
        self.save_users(&users)?;

        Ok(())
    }

    /// Update user password
    pub fn update_password(&self, username: &str, password_hash: String) -> Result<()> {
        let mut users = self.load_users()?;

        let user = users
            .iter_mut()
            .find(|u| u.username == username)
            .ok_or_else(|| anyhow::anyhow!("User '{}' not found", username))?;

        user.password_hash = password_hash;
        self.save_users(&users)?;

        Ok(())
    }

    /// Enable/disable user
    pub fn set_user_enabled(&self, username: &str, enabled: bool) -> Result<()> {
        let mut users = self.load_users()?;

        let user = users
            .iter_mut()
            .find(|u| u.username == username)
            .ok_or_else(|| anyhow::anyhow!("User '{}' not found", username))?;

        user.enabled = enabled;
        self.save_users(&users)?;

        Ok(())
    }

    /// Clear force_password_change flag for a user
    pub fn clear_force_password_change(&self, username: &str) -> Result<()> {
        let mut users = self.load_users()?;

        let user = users
            .iter_mut()
            .find(|u| u.username == username)
            .ok_or_else(|| anyhow::anyhow!("User '{}' not found", username))?;

        user.force_password_change = false;
        self.save_users(&users)?;

        Ok(())
    }

    /// Check if any users exist
    pub fn has_users(&self) -> Result<bool> {
        Ok(!self.load_users()?.is_empty())
    }

    /// Create default admin user if no users exist
    #[cfg(feature = "server")]
    pub fn ensure_default_admin(&self, password: &str) -> Result<bool> {
        if self.has_users()? {
            return Ok(false);
        }

        let password_hash = hash_password(password)?;
        let mut roles = HashSet::new();
        roles.insert(Role::Admin);

        let mut admin = User::new("admin".to_string(), password_hash, roles);
        // Force password change on first login with default credentials
        admin.force_password_change = true;
        self.add_user(admin)?;

        Ok(true)
    }
}

/// Hash password using Argon2id
#[cfg(feature = "server")]
pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Password hashing failed: {}", e))?
        .to_string();

    Ok(password_hash)
}

/// Verify password against hash
#[cfg(feature = "server")]
pub fn verify_password(password: &str, password_hash: &str) -> Result<bool> {
    let parsed_hash = PasswordHash::new(password_hash)
        .map_err(|e| anyhow::anyhow!("Invalid password hash: {}", e))?;

    let argon2 = Argon2::default();

    Ok(argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_role_permissions() {
        assert!(Role::Admin.can_read());
        assert!(Role::Admin.can_control_services());
        assert!(Role::Admin.can_configure());
        assert!(Role::Admin.can_manage_users());

        assert!(Role::Operator.can_read());
        assert!(Role::Operator.can_control_services());
        assert!(!Role::Operator.can_configure());
        assert!(!Role::Operator.can_manage_users());

        assert!(Role::Viewer.can_read());
        assert!(!Role::Viewer.can_control_services());
        assert!(!Role::Viewer.can_configure());
        assert!(!Role::Viewer.can_manage_users());
    }

    #[test]
    fn test_user_manager() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let manager = UserManager::new(temp_dir.path().to_path_buf())?;

        // Initially no users
        assert!(!manager.has_users()?);

        // Add user
        let mut roles = HashSet::new();
        roles.insert(Role::Admin);
        let user = User::new("test".to_string(), "hash".to_string(), roles);
        manager.add_user(user)?;

        // Now has users
        assert!(manager.has_users()?);

        // Get user
        let user = manager.get_user("test")?.expect("User should exist");
        assert_eq!(user.username, "test");
        assert!(user.is_admin());

        // Remove user
        manager.remove_user("test")?;
        assert!(!manager.has_users()?);

        Ok(())
    }

    #[cfg(feature = "server")]
    #[test]
    fn test_password_hashing() -> Result<()> {
        let password = "test_password_123";
        let hash = hash_password(password)?;

        // Hash should not equal password
        assert_ne!(hash, password);

        // Should verify correctly
        assert!(verify_password(password, &hash)?);

        // Wrong password should not verify
        assert!(!verify_password("wrong_password", &hash)?);

        Ok(())
    }
}
