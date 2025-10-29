use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub public_ipv4: Option<String>,
    pub public_ipv6: Option<String>,
    pub lan_ip: Option<String>,
    pub hostname: Option<String>,
    pub domains: HashSet<String>,
}

#[derive(Debug, Clone)]
pub struct NetworkInfoDetector;

impl NetworkInfoDetector {
    pub fn new() -> Self {
        Self
    }

    /// Detect public IPv4 address
    pub fn get_public_ipv4(&self) -> Option<String> {
        // Try multiple methods

        // Method 1: Check .env file for PUBLIC_IP
        if let Ok(env_ip) = std::env::var("PUBLIC_IP") {
            if !env_ip.is_empty() {
                return Some(env_ip);
            }
        }

        // Method 2: Use ip route to get default gateway interface, then query external service
        if let Ok(output) = Command::new("curl")
            .args(&["-s", "-4", "https://api.ipify.org"])
            .output()
        {
            if output.status.success() {
                let ip = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !ip.is_empty() && ip.contains('.') {
                    return Some(ip);
                }
            }
        }

        // Method 3: Alternative service
        if let Ok(output) = Command::new("curl")
            .args(&["-s", "-4", "https://ifconfig.me"])
            .output()
        {
            if output.status.success() {
                let ip = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !ip.is_empty() && ip.contains('.') {
                    return Some(ip);
                }
            }
        }

        None
    }

    /// Detect public IPv6 address
    pub fn get_public_ipv6(&self) -> Option<String> {
        // Try to get IPv6 from external service
        if let Ok(output) = Command::new("curl")
            .args(&["-s", "-6", "https://api6.ipify.org"])
            .output()
        {
            if output.status.success() {
                let ip = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !ip.is_empty() && ip.contains(':') {
                    return Some(ip);
                }
            }
        }

        None
    }

    /// Get LAN IP address
    pub fn get_lan_ip(&self) -> Option<String> {
        #[cfg(target_os = "linux")]
        {
            // Use hostname -I to get all IPs, filter for private ranges
            if let Ok(output) = Command::new("hostname").arg("-I").output() {
                if output.status.success() {
                    let ips = String::from_utf8_lossy(&output.stdout);
                    for ip in ips.split_whitespace() {
                        if ip.starts_with("192.168.") || ip.starts_with("10.") || ip.starts_with("172.") {
                            return Some(ip.to_string());
                        }
                    }
                }
            }
        }

        None
    }

    /// Get hostname
    pub fn get_hostname(&self) -> Option<String> {
        if let Ok(output) = Command::new("hostname").output() {
            if output.status.success() {
                let hostname = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !hostname.is_empty() {
                    return Some(hostname);
                }
            }
        }

        None
    }

    /// Detect domains from various sources
    pub fn detect_domains(&self) -> HashSet<String> {
        let mut domains = HashSet::new();

        // Check environment variables
        if let Ok(domain) = std::env::var("DOMAIN") {
            if !domain.is_empty() {
                domains.insert(domain);
            }
        }

        if let Ok(domains_str) = std::env::var("DOMAINS") {
            for domain in domains_str.split(',') {
                let domain = domain.trim();
                if !domain.is_empty() {
                    domains.insert(domain.to_string());
                }
            }
        }

        // Parse nginx server_names (done separately by nginx parser)
        // Parse Traefik labels (done separately by docker manager)

        domains
    }

    /// Get complete network information
    pub fn get_info(&self) -> NetworkInfo {
        NetworkInfo {
            public_ipv4: self.get_public_ipv4(),
            public_ipv6: self.get_public_ipv6(),
            lan_ip: self.get_lan_ip(),
            hostname: self.get_hostname(),
            domains: self.detect_domains(),
        }
    }
}

/// Security issue types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SecurityIssueType {
    PubliclyExposedSensitivePort,
    MissingSSL,
    RunningAsRoot,
    WeakAuthentication,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityWarning {
    pub issue_type: SecurityIssueType,
    pub severity: String, // "critical", "high", "medium", "low"
    pub service: String,
    pub description: String,
    pub recommendation: String,
}

#[derive(Debug, Clone)]
pub struct SecurityScanner;

impl SecurityScanner {
    pub fn new() -> Self {
        Self
    }

    /// Check for publicly exposed sensitive ports
    pub fn scan_public_ports(&self, ports: &[(u16, String)]) -> Vec<SecurityWarning> {
        let mut warnings = Vec::new();

        let sensitive_ports = [
            (3306, "MySQL", "Database server should not be publicly accessible"),
            (5432, "PostgreSQL", "Database server should not be publicly accessible"),
            (6379, "Redis", "Cache server should not be publicly accessible"),
            (27017, "MongoDB", "Database server should not be publicly accessible"),
            (3389, "RDP", "Remote desktop protocol should not be publicly accessible"),
            (5900, "VNC", "VNC server should not be publicly accessible"),
            (9200, "Elasticsearch", "Search engine should not be publicly accessible"),
            (8080, "Common HTTP Alt", "May expose admin panels or development servers"),
        ];

        for (port, service) in ports {
            for (sensitive_port, service_name, desc) in &sensitive_ports {
                if port == sensitive_port {
                    warnings.push(SecurityWarning {
                        issue_type: SecurityIssueType::PubliclyExposedSensitivePort,
                        severity: if *sensitive_port == 3389 || *sensitive_port == 3306 || *sensitive_port == 5432 {
                            "critical".to_string()
                        } else {
                            "high".to_string()
                        },
                        service: service.clone(),
                        description: format!("Port {} ({}) is publicly accessible: {}", port, service_name, desc),
                        recommendation: format!("Use firewall rules to restrict access to {} or bind to 127.0.0.1 only", port),
                    });
                }
            }
        }

        warnings
    }

    /// Check if a service is missing SSL for public access
    pub fn check_ssl(&self, has_ssl: bool, is_public: bool, port: u16) -> Option<SecurityWarning> {
        if is_public && (port == 80 || port == 8080) && !has_ssl {
            return Some(SecurityWarning {
                issue_type: SecurityIssueType::MissingSSL,
                severity: "medium".to_string(),
                service: format!("Port {}", port),
                description: format!("HTTP service on port {} is not using SSL/TLS", port),
                recommendation: "Enable SSL/TLS encryption for public web services".to_string(),
            });
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_info_detection() {
        let detector = NetworkInfoDetector::new();
        let info = detector.get_info();
        println!("Network Info: {:?}", info);
    }

    #[test]
    fn test_security_scanner() {
        let scanner = SecurityScanner::new();
        let ports = vec![(3389, "rdp-service".to_string()), (22, "ssh".to_string())];
        let warnings = scanner.scan_public_ports(&ports);
        assert!(!warnings.is_empty());
        assert_eq!(warnings[0].issue_type, SecurityIssueType::PubliclyExposedSensitivePort);
    }
}
