/// Security module for IP allowlisting and network access control

use anyhow::{Context, Result};
use ipnetwork::IpNetwork;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::path::PathBuf;
use std::str::FromStr;

/// IP allowlist configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpAllowlist {
    /// List of allowed IP networks (CIDR notation)
    pub allowed_ips: Vec<String>,

    /// Block Tor exit nodes
    #[serde(default)]
    pub block_tor: bool,

    /// Block VPN connections
    #[serde(default)]
    pub block_vpn: bool,

    /// Allowed countries (ISO 3166-1 alpha-2 codes, empty = all)
    #[serde(default)]
    pub allowed_countries: Vec<String>,

    /// Trust proxy headers (X-Real-IP, X-Forwarded-For)
    #[serde(default)]
    pub trust_proxy: bool,

    /// Proxy header to use
    #[serde(default = "default_proxy_header")]
    pub proxy_header: String,
}

fn default_proxy_header() -> String {
    "X-Real-IP".to_string()
}

impl Default for IpAllowlist {
    fn default() -> Self {
        Self {
            allowed_ips: Vec::new(), // Empty = allow all
            block_tor: true,
            block_vpn: false,
            allowed_countries: Vec::new(),
            trust_proxy: false,
            proxy_header: default_proxy_header(),
        }
    }
}

impl IpAllowlist {
    /// Check if an IP address is allowed
    pub fn is_allowed(&self, ip: IpAddr) -> Result<bool> {
        // If no restrictions, allow all
        if self.allowed_ips.is_empty() {
            return Ok(true);
        }

        // Check against allowed networks
        for network_str in &self.allowed_ips {
            let network = IpNetwork::from_str(network_str)
                .with_context(|| format!("Invalid IP network: {}", network_str))?;

            if network.contains(ip) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Add IP or network to allowlist
    pub fn add_network(&mut self, network: String) -> Result<()> {
        // Validate network format
        IpNetwork::from_str(&network)
            .with_context(|| format!("Invalid IP network: {}", network))?;

        if !self.allowed_ips.contains(&network) {
            self.allowed_ips.push(network);
        }

        Ok(())
    }

    /// Remove IP or network from allowlist
    pub fn remove_network(&mut self, network: &str) -> bool {
        let initial_len = self.allowed_ips.len();
        self.allowed_ips.retain(|n| n != network);
        self.allowed_ips.len() < initial_len
    }

    /// Check if allowlist is empty (allowing all)
    pub fn is_empty(&self) -> bool {
        self.allowed_ips.is_empty()
    }
}

/// Security configuration file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub security: IpAllowlist,
}

/// Security manager with file-based storage
pub struct SecurityManager {
    config_path: PathBuf,
}

impl SecurityManager {
    pub fn new(config_dir: PathBuf) -> Result<Self> {
        // Create config directory if it doesn't exist
        std::fs::create_dir_all(&config_dir)
            .context("Failed to create security config directory")?;

        Ok(Self {
            config_path: config_dir.join("security.yaml"),
        })
    }

    /// Load security config from file
    pub fn load_config(&self) -> Result<IpAllowlist> {
        if !self.config_path.exists() {
            return Ok(IpAllowlist::default());
        }

        let content = std::fs::read_to_string(&self.config_path)
            .context("Failed to read security config file")?;

        let config: SecurityConfig = serde_yaml::from_str(&content)
            .context("Failed to parse security config file")?;

        Ok(config.security)
    }

    /// Save security config to file
    pub fn save_config(&self, allowlist: &IpAllowlist) -> Result<()> {
        let config = SecurityConfig {
            security: allowlist.clone(),
        };

        let content = serde_yaml::to_string(&config)
            .context("Failed to serialize security config")?;

        std::fs::write(&self.config_path, content)
            .context("Failed to write security config file")?;

        Ok(())
    }

    /// Add network to allowlist
    pub fn add_network(&self, network: String) -> Result<()> {
        let mut allowlist = self.load_config()?;
        allowlist.add_network(network)?;
        self.save_config(&allowlist)?;
        Ok(())
    }

    /// Remove network from allowlist
    pub fn remove_network(&self, network: &str) -> Result<bool> {
        let mut allowlist = self.load_config()?;
        let removed = allowlist.remove_network(network);

        if removed {
            self.save_config(&allowlist)?;
        }

        Ok(removed)
    }

    /// Check if IP is allowed
    pub fn is_ip_allowed(&self, ip: IpAddr) -> Result<bool> {
        let allowlist = self.load_config()?;
        allowlist.is_allowed(ip)
    }
}

/// Extract real IP from request, considering proxy headers
pub fn extract_real_ip(
    peer_ip: IpAddr,
    proxy_header: Option<&str>,
    trust_proxy: bool,
) -> IpAddr {
    if !trust_proxy {
        return peer_ip;
    }

    if let Some(header_value) = proxy_header {
        // Try to parse the first IP from the header (X-Forwarded-For can be a list)
        if let Some(first_ip) = header_value.split(',').next() {
            if let Ok(ip) = first_ip.trim().parse::<IpAddr>() {
                return ip;
            }
        }
    }

    peer_ip
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;
    use tempfile::TempDir;

    #[test]
    fn test_ip_allowlist_empty() -> Result<()> {
        let allowlist = IpAllowlist::default();
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        // Empty allowlist should allow all
        assert!(allowlist.is_allowed(ip)?);
        assert!(allowlist.is_empty());

        Ok(())
    }

    #[test]
    fn test_ip_allowlist_single_ip() -> Result<()> {
        let mut allowlist = IpAllowlist::default();
        allowlist.add_network("192.168.1.100/32".to_string())?;

        let allowed_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));
        let blocked_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 101));

        assert!(allowlist.is_allowed(allowed_ip)?);
        assert!(!allowlist.is_allowed(blocked_ip)?);

        Ok(())
    }

    #[test]
    fn test_ip_allowlist_network() -> Result<()> {
        let mut allowlist = IpAllowlist::default();
        allowlist.add_network("192.168.1.0/24".to_string())?;

        let allowed_ip1 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let allowed_ip2 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 255));
        let blocked_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 2, 1));

        assert!(allowlist.is_allowed(allowed_ip1)?);
        assert!(allowlist.is_allowed(allowed_ip2)?);
        assert!(!allowlist.is_allowed(blocked_ip)?);

        Ok(())
    }

    #[test]
    fn test_ip_allowlist_multiple_networks() -> Result<()> {
        let mut allowlist = IpAllowlist::default();
        allowlist.add_network("192.168.1.0/24".to_string())?;
        allowlist.add_network("10.0.0.0/8".to_string())?;

        let ip1 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 50));
        let ip2 = IpAddr::V4(Ipv4Addr::new(10, 20, 30, 40));
        let ip3 = IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1));

        assert!(allowlist.is_allowed(ip1)?);
        assert!(allowlist.is_allowed(ip2)?);
        assert!(!allowlist.is_allowed(ip3)?);

        Ok(())
    }

    #[test]
    fn test_remove_network() -> Result<()> {
        let mut allowlist = IpAllowlist::default();
        allowlist.add_network("192.168.1.0/24".to_string())?;

        assert!(!allowlist.is_empty());
        assert!(allowlist.remove_network("192.168.1.0/24"));
        assert!(allowlist.is_empty());

        // Removing non-existent network returns false
        assert!(!allowlist.remove_network("10.0.0.0/8"));

        Ok(())
    }

    #[test]
    fn test_security_manager() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let manager = SecurityManager::new(temp_dir.path().to_path_buf())?;

        // Initially empty (allow all)
        let config = manager.load_config()?;
        assert!(config.is_empty());

        // Add network
        manager.add_network("192.168.1.0/24".to_string())?;

        // Verify added
        let config = manager.load_config()?;
        assert!(!config.is_empty());
        assert_eq!(config.allowed_ips.len(), 1);

        // Check IP
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 50));
        assert!(manager.is_ip_allowed(ip)?);

        // Remove network
        assert!(manager.remove_network("192.168.1.0/24")?);

        // Verify removed
        let config = manager.load_config()?;
        assert!(config.is_empty());

        Ok(())
    }

    #[test]
    fn test_extract_real_ip() {
        let peer_ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        let forwarded_ip = "192.168.1.100";

        // Without trust_proxy, should return peer IP
        assert_eq!(
            extract_real_ip(peer_ip, Some(forwarded_ip), false),
            peer_ip
        );

        // With trust_proxy, should return forwarded IP
        let expected = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));
        assert_eq!(
            extract_real_ip(peer_ip, Some(forwarded_ip), true),
            expected
        );

        // With multiple IPs in header, should use first
        let multiple_ips = "192.168.1.100, 10.0.0.5";
        assert_eq!(
            extract_real_ip(peer_ip, Some(multiple_ips), true),
            expected
        );

        // With invalid header, should fall back to peer IP
        assert_eq!(
            extract_real_ip(peer_ip, Some("invalid"), true),
            peer_ip
        );
    }
}
