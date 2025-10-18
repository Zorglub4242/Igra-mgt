/// SSL/TLS certificate management

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::process::Command;

pub struct SslManager {
    project_root: std::path::PathBuf,
}

#[derive(Debug, Clone)]
pub struct CertificateInfo {
    pub domain: String,
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_until: Option<DateTime<Utc>>,
    pub days_remaining: Option<i64>,
    pub is_valid: bool,
}

#[derive(Debug, Deserialize)]
struct AcmeData {
    #[serde(default)]
    #[serde(rename = "Certificates")]
    certificates: Vec<AcmeCertificate>,
}

#[derive(Debug, Deserialize)]
struct AcmeCertificate {
    domain: AcmeDomain,
}

#[derive(Debug, Deserialize)]
struct AcmeDomain {
    main: String,
    #[serde(default)]
    sans: Vec<String>,
}

impl SslManager {
    pub fn new() -> Result<Self> {
        let project_root = crate::utils::get_project_root()?;
        Ok(Self { project_root })
    }

    /// Check certificate from ACME JSON file
    pub async fn get_certificate_info(&self, domain: &str) -> Result<CertificateInfo> {
        // First check ACME JSON file
        let acme_file = self.project_root.join("traefik_certs/acme.json");

        if !acme_file.exists() {
            return Err(anyhow!(
                "ACME certificate file not found at {}",
                acme_file.display()
            ));
        }

        // Read and parse ACME JSON
        let acme_content = fs::read_to_string(&acme_file)
            .context("Failed to read acme.json")?;

        let acme_json: Value = serde_json::from_str(&acme_content)
            .context("Failed to parse acme.json")?;

        // Extract certificate info
        // ACME JSON structure varies, try to find certificate
        let cert_found = acme_json
            .as_object()
            .and_then(|obj| {
                obj.values().find_map(|resolver| {
                    resolver
                        .get("Certificates")
                        .and_then(|certs| certs.as_array())
                        .and_then(|arr| {
                            arr.iter().find(|cert| {
                                cert.get("domain")
                                    .and_then(|d| d.get("main"))
                                    .and_then(|m| m.as_str())
                                    .map(|s| s == domain)
                                    .unwrap_or(false)
                            })
                        })
                })
            });

        if cert_found.is_none() {
            return Ok(CertificateInfo {
                domain: domain.to_string(),
                valid_from: None,
                valid_until: None,
                days_remaining: None,
                is_valid: false,
            });
        }

        // Use openssl to check the actual certificate
        self.check_certificate_with_openssl(domain).await
    }

    /// Check certificate using openssl s_client
    async fn check_certificate_with_openssl(&self, domain: &str) -> Result<CertificateInfo> {
        let output = Command::new("sh")
            .arg("-c")
            .arg(format!(
                "echo | timeout 5 openssl s_client -servername {} -connect {}:443 2>/dev/null | openssl x509 -noout -dates",
                domain, domain
            ))
            .output()
            .context("Failed to run openssl")?;

        if !output.status.success() {
            return Ok(CertificateInfo {
                domain: domain.to_string(),
                valid_from: None,
                valid_until: None,
                days_remaining: None,
                is_valid: false,
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse dates from openssl output
        // Format: notBefore=Jan  1 00:00:00 2024 GMT
        //         notAfter=Apr  1 23:59:59 2024 GMT

        let valid_from = stdout
            .lines()
            .find(|line| line.starts_with("notBefore="))
            .and_then(|line| {
                let date_str = line.strip_prefix("notBefore=")?;
                Self::parse_openssl_date(date_str)
            });

        let valid_until = stdout
            .lines()
            .find(|line| line.starts_with("notAfter="))
            .and_then(|line| {
                let date_str = line.strip_prefix("notAfter=")?;
                Self::parse_openssl_date(date_str)
            });

        let days_remaining = valid_until.map(|until| {
            let now = Utc::now();
            (until - now).num_days()
        });

        let is_valid = valid_until
            .map(|until| until > Utc::now())
            .unwrap_or(false)
            && valid_from
                .map(|from| from < Utc::now())
                .unwrap_or(false);

        Ok(CertificateInfo {
            domain: domain.to_string(),
            valid_from,
            valid_until,
            days_remaining,
            is_valid,
        })
    }

    /// Parse OpenSSL date format
    fn parse_openssl_date(date_str: &str) -> Option<DateTime<Utc>> {
        // OpenSSL format: "Jan  1 00:00:00 2024 GMT"
        use chrono::NaiveDateTime;

        let date_str = date_str.trim().trim_end_matches(" GMT");

        // Try to parse
        NaiveDateTime::parse_from_str(date_str, "%b %e %H:%M:%S %Y")
            .ok()
            .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
    }

    /// Force renewal of certificates (restart Traefik)
    pub async fn force_renewal(&self) -> Result<()> {
        Command::new("docker")
            .args(&["restart", "traefik"])
            .current_dir(&self.project_root)
            .status()
            .context("Failed to restart Traefik")?;

        Ok(())
    }
}
