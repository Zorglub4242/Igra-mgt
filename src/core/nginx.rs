use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NginxSite {
    pub name: String,
    pub config_file: String,
    pub enabled: bool,
    pub server_names: Vec<String>,
    pub listen_ports: Vec<u16>,
    pub ssl_enabled: bool,
    pub ssl_certificate: Option<String>,
    pub upstreams: Vec<NginxUpstream>,
    pub locations: Vec<NginxLocation>,
    pub auth_basic: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NginxUpstream {
    pub name: String,
    pub servers: Vec<String>,  // host:port
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NginxLocation {
    pub path: String,
    pub proxy_pass: Option<String>,
    pub upstream_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NginxParser;

impl NginxParser {
    pub fn new() -> Self {
        Self
    }

    /// Parse nginx configuration files from /etc/nginx/sites-enabled
    pub fn parse_sites(&self) -> Result<Vec<NginxSite>> {
        let sites_enabled = Path::new("/etc/nginx/sites-enabled");
        let mut sites = Vec::new();

        if !sites_enabled.exists() {
            return Ok(sites);
        }

        for entry in fs::read_dir(sites_enabled)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Ok(site) = self.parse_site_file(&path) {
                    sites.push(site);
                }
            }
        }

        Ok(sites)
    }

    fn parse_site_file(&self, path: &Path) -> Result<NginxSite> {
        let content = fs::read_to_string(path)?;
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let mut site = NginxSite {
            name: name.clone(),
            config_file: path.to_string_lossy().to_string(),
            enabled: true,
            server_names: Vec::new(),
            listen_ports: Vec::new(),
            ssl_enabled: false,
            ssl_certificate: None,
            upstreams: Vec::new(),
            locations: Vec::new(),
            auth_basic: false,
        };

        // Parse content line by line
        let mut in_server_block = false;
        let mut in_location_block = false;
        let mut in_upstream_block = false;
        let mut current_location: Option<NginxLocation> = None;
        let mut current_upstream: Option<NginxUpstream> = None;
        let mut brace_depth = 0;

        for line in content.lines() {
            let line = line.trim();

            // Track brace depth
            brace_depth += line.matches('{').count() as i32;
            brace_depth -= line.matches('}').count() as i32;

            // Detect blocks
            if line.starts_with("server") && line.contains('{') {
                in_server_block = true;
                continue;
            }

            if line.starts_with("upstream") {
                if let Some(name) = line.split_whitespace().nth(1) {
                    current_upstream = Some(NginxUpstream {
                        name: name.trim_end_matches('{').trim().to_string(),
                        servers: Vec::new(),
                    });
                    in_upstream_block = true;
                }
                continue;
            }

            if line.starts_with("location") {
                let path = line
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or("/")
                    .trim_end_matches('{')
                    .trim()
                    .to_string();

                current_location = Some(NginxLocation {
                    path,
                    proxy_pass: None,
                    upstream_name: None,
                });
                in_location_block = true;
                continue;
            }

            // Close blocks when braces close
            if brace_depth == 0 {
                if in_location_block {
                    if let Some(loc) = current_location.take() {
                        site.locations.push(loc);
                    }
                    in_location_block = false;
                }
                if in_upstream_block {
                    if let Some(upstream) = current_upstream.take() {
                        site.upstreams.push(upstream);
                    }
                    in_upstream_block = false;
                }
                if in_server_block {
                    in_server_block = false;
                }
            }

            // Parse directives
            if in_server_block {
                if line.starts_with("server_name") {
                    let names: Vec<String> = line
                        .trim_start_matches("server_name")
                        .trim_end_matches(';')
                        .split_whitespace()
                        .map(|s| s.to_string())
                        .collect();
                    site.server_names.extend(names);
                }

                if line.starts_with("listen") {
                    if let Some(port_str) = line.split_whitespace().nth(1) {
                        let port_str = port_str.trim_end_matches(';');
                        // Handle "listen 80", "listen 443 ssl", "listen [::]:80"
                        let port = port_str
                            .split(':')
                            .last()
                            .and_then(|p| p.split_whitespace().next())
                            .and_then(|p| p.parse().ok())
                            .unwrap_or(80);

                        if !site.listen_ports.contains(&port) {
                            site.listen_ports.push(port);
                        }

                        if line.contains("ssl") {
                            site.ssl_enabled = true;
                        }
                    }
                }

                if line.starts_with("ssl_certificate") && !line.contains("ssl_certificate_key") {
                    let cert = line
                        .trim_start_matches("ssl_certificate")
                        .trim_end_matches(';')
                        .trim()
                        .to_string();
                    site.ssl_certificate = Some(cert);
                }

                if line.starts_with("auth_basic") && !line.contains("auth_basic_user_file") {
                    site.auth_basic = true;
                }
            }

            if in_location_block {
                if let Some(ref mut loc) = current_location {
                    if line.starts_with("proxy_pass") {
                        let proxy = line
                            .trim_start_matches("proxy_pass")
                            .trim_end_matches(';')
                            .trim()
                            .to_string();

                        // Check if it's an upstream reference or direct URL
                        if proxy.starts_with("http://") || proxy.starts_with("https://") {
                            loc.proxy_pass = Some(proxy.clone());

                            // Extract host and port
                            if let Some(host_port) = proxy
                                .trim_start_matches("http://")
                                .trim_start_matches("https://")
                                .split('/')
                                .next()
                            {
                                loc.upstream_name = Some(host_port.to_string());
                            }
                        } else {
                            // Upstream reference like "http://backend"
                            loc.upstream_name = Some(
                                proxy
                                    .trim_start_matches("http://")
                                    .trim_start_matches("https://")
                                    .to_string(),
                            );
                        }
                    }
                }
            }

            if in_upstream_block {
                if let Some(ref mut upstream) = current_upstream {
                    if line.starts_with("server") {
                        let server = line
                            .trim_start_matches("server")
                            .split_whitespace()
                            .next()
                            .unwrap_or("")
                            .trim_end_matches(';')
                            .to_string();

                        if !server.is_empty() {
                            upstream.servers.push(server);
                        }
                    }
                }
            }
        }

        Ok(site)
    }

    /// Extract all proxy targets from nginx configuration
    pub fn get_proxy_targets(&self, sites: &[NginxSite]) -> HashMap<String, Vec<String>> {
        let mut targets: HashMap<String, Vec<String>> = HashMap::new();

        for site in sites {
            for location in &site.locations {
                if let Some(ref proxy_pass) = location.proxy_pass {
                    targets
                        .entry(site.name.clone())
                        .or_insert_with(Vec::new)
                        .push(proxy_pass.clone());
                } else if let Some(ref upstream_name) = location.upstream_name {
                    // Find upstream definition
                    if let Some(upstream) = site.upstreams.iter().find(|u| &u.name == upstream_name) {
                        for server in &upstream.servers {
                            targets
                                .entry(site.name.clone())
                                .or_insert_with(Vec::new)
                                .push(server.clone());
                        }
                    }
                }
            }
        }

        targets
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nginx_parser() {
        let parser = NginxParser::new();
        // Test only runs if nginx config exists
        if let Ok(sites) = parser.parse_sites() {
            for site in sites {
                println!("Site: {} - Domains: {:?}", site.name, site.server_names);
            }
        }
    }
}
