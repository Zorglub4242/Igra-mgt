use super::MetricFetcher;
/// Docker exec fetcher
///
/// Executes arbitrary commands inside containers to fetch metrics
use anyhow::{Context, Result};
use regex::Regex;

#[derive(Debug, Clone)]
pub struct DockerExecFetcher {
    pub command: String,
    pub regex_pattern: Option<String>,
}

impl DockerExecFetcher {
    pub fn new(command: String, regex_pattern: Option<String>) -> Self {
        Self {
            command,
            regex_pattern,
        }
    }

    /// Parse a metric value using regex from command output
    fn parse_with_regex(text: &str, pattern: &str) -> Option<f64> {
        let re = Regex::new(pattern).ok()?;
        let captures = re.captures(text)?;

        // Try to find first capture group, or use entire match
        let value_str = captures
            .get(1)
            .or_else(|| captures.get(0))
            .map(|m| m.as_str())?;

        value_str.parse::<f64>().ok()
    }
}

impl MetricFetcher for DockerExecFetcher {
    async fn fetch_raw(&self, container_name: &str) -> Result<String> {
        let output = tokio::process::Command::new("docker")
            .args(&["exec", container_name, "sh", "-c", &self.command])
            .output()
            .await
            .context("Failed to execute docker exec command")?;

        if !output.status.success() {
            return Ok(String::new());
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    async fn fetch_metric(&self, container_name: &str, _metric_name: &str) -> Result<Option<f64>> {
        let raw = self.fetch_raw(container_name).await?;

        if let Some(pattern) = &self.regex_pattern {
            Ok(Self::parse_with_regex(&raw, pattern))
        } else {
            // Try to parse entire output as number
            Ok(raw.trim().parse::<f64>().ok())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_with_regex() {
        let output = "Block height: 12345\nPeers: 42";

        assert_eq!(
            DockerExecFetcher::parse_with_regex(output, r"Block height: (\d+)"),
            Some(12345.0)
        );
        assert_eq!(
            DockerExecFetcher::parse_with_regex(output, r"Peers: (\d+)"),
            Some(42.0)
        );
        assert_eq!(
            DockerExecFetcher::parse_with_regex(output, r"Nonexistent: (\d+)"),
            None
        );
    }

    #[test]
    fn test_parse_simple_number() {
        let output = "123456";
        assert_eq!(output.trim().parse::<f64>().ok(), Some(123456.0));
    }
}
