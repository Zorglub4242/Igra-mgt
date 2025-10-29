use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirewallRule {
    pub port: String,
    pub protocol: String,
    pub action: String,
    pub source: String, // "0.0.0.0/0" for public, specific IP/CIDR for restricted, "Anywhere" for any
    pub destination: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirewallStatus {
    pub active: bool,
    pub default_incoming: String,
    pub default_outgoing: String,
    pub rules: Vec<FirewallRule>,
}

#[derive(Debug, Clone)]
pub struct FirewallManager;

impl FirewallManager {
    pub fn new() -> Self {
        Self
    }

    /// Get UFW firewall status and rules
    pub fn get_status(&self) -> Result<FirewallStatus> {
        #[cfg(target_os = "linux")]
        {
            let output = Command::new("ufw")
                .arg("status")
                .arg("verbose")
                .output()?;

            if !output.status.success() {
                return Ok(FirewallStatus {
                    active: false,
                    default_incoming: "deny".to_string(),
                    default_outgoing: "allow".to_string(),
                    rules: vec![],
                });
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            self.parse_ufw_output(&stdout)
        }

        #[cfg(not(target_os = "linux"))]
        {
            Ok(FirewallStatus {
                active: false,
                default_incoming: "deny".to_string(),
                default_outgoing: "allow".to_string(),
                rules: vec![],
            })
        }
    }

    fn parse_ufw_output(&self, output: &str) -> Result<FirewallStatus> {
        let mut status = FirewallStatus {
            active: false,
            default_incoming: "deny".to_string(),
            default_outgoing: "allow".to_string(),
            rules: vec![],
        };

        let mut in_rules_section = false;

        for line in output.lines() {
            let line = line.trim();

            // Check if firewall is active
            if line.starts_with("Status:") {
                status.active = line.contains("active");
            }

            // Parse default policies
            if line.starts_with("Default:") {
                // Format: "Default: deny (incoming), allow (outgoing), disabled (routed)"
                if let Some(incoming) = line.split(',').next() {
                    if let Some(policy) = incoming.split('(').nth(1) {
                        status.default_incoming = policy.trim_end_matches(')').to_lowercase();
                    }
                }
                if let Some(outgoing) = line.split(',').nth(1) {
                    if let Some(policy) = outgoing.split('(').nth(1) {
                        status.default_outgoing = policy.trim_end_matches(')').to_lowercase();
                    }
                }
            }

            // Detect start of rules section
            if line.starts_with("To") && line.contains("Action") && line.contains("From") {
                in_rules_section = true;
                continue;
            }

            if line.starts_with("--") || line.is_empty() {
                continue;
            }

            // Parse rules
            if in_rules_section && !line.is_empty() {
                if let Some(rule) = self.parse_rule_line(line) {
                    status.rules.push(rule);
                }
            }
        }

        Ok(status)
    }

    fn parse_rule_line(&self, line: &str) -> Option<FirewallRule> {
        // UFW output format examples:
        // "22/tcp                     ALLOW IN    Anywhere"
        // "80/tcp                     ALLOW IN    Anywhere"
        // "443                        ALLOW IN    Anywhere"
        // "16211/tcp                  ALLOW IN    37.27.122.164"
        // "Anywhere                   ALLOW IN    192.168.1.0/24"

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            return None;
        }

        let destination = parts[0].to_string();
        let action = parts[1].to_string();
        let _direction = parts[2]; // IN or OUT
        let source = parts[3..].join(" ");

        // Parse port and protocol from destination
        let (port, protocol) = if destination.contains('/') {
            let mut split = destination.split('/');
            let port = split.next().unwrap_or("").to_string();
            let proto = split.next().unwrap_or("tcp").to_string();
            (port, proto)
        } else if destination == "Anywhere" {
            ("*".to_string(), "all".to_string())
        } else {
            (destination.clone(), "tcp".to_string())
        };

        // Normalize source
        let normalized_source = if source == "Anywhere" || source.starts_with("Anywhere") {
            "0.0.0.0/0".to_string()
        } else {
            source
        };

        Some(FirewallRule {
            port,
            protocol,
            action: action.to_lowercase(),
            source: normalized_source,
            destination: Some(destination),
        })
    }

    /// Group rules by accessibility type
    pub fn categorize_rules(&self, rules: &[FirewallRule]) -> (Vec<FirewallRule>, Vec<FirewallRule>, Vec<FirewallRule>) {
        let mut public_rules = vec![];
        let mut lan_only_rules = vec![];
        let mut special_rules = vec![];

        for rule in rules {
            if rule.source == "0.0.0.0/0" || rule.source == "Anywhere" {
                public_rules.push(rule.clone());
            } else if rule.source.starts_with("192.168.") || rule.source.starts_with("10.") || rule.source.starts_with("172.") {
                lan_only_rules.push(rule.clone());
            } else {
                special_rules.push(rule.clone());
            }
        }

        (public_rules, lan_only_rules, special_rules)
    }

    /// Check if a specific port is publicly accessible
    pub fn is_port_public(&self, status: &FirewallStatus, port: u16) -> bool {
        let port_str = port.to_string();
        status.rules.iter().any(|rule| {
            rule.action == "allow" &&
            (rule.port == port_str || rule.port == "*") &&
            (rule.source == "0.0.0.0/0" || rule.source == "Anywhere")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rule_line() {
        let fm = FirewallManager::new();

        let rule = fm.parse_rule_line("22/tcp                     ALLOW IN    Anywhere").unwrap();
        assert_eq!(rule.port, "22");
        assert_eq!(rule.protocol, "tcp");
        assert_eq!(rule.action, "allow");
        assert_eq!(rule.source, "0.0.0.0/0");

        let rule = fm.parse_rule_line("16211/tcp                  ALLOW IN    37.27.122.164").unwrap();
        assert_eq!(rule.port, "16211");
        assert_eq!(rule.source, "37.27.122.164");
    }
}
