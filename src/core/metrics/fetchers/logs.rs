/// Logs-based metrics fetcher
///
/// Parses metrics from container logs using regex patterns

use anyhow::{Result, Context};
use super::MetricFetcher;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct LogsFetcher {
    pub lines: usize,
    pub pattern: Option<String>, // Now optional - can use per-metric patterns instead
}

impl LogsFetcher {
    pub fn new(pattern: String, lines: usize) -> Self {
        Self { lines, pattern: Some(pattern) }
    }

    /// Create a fetcher without a global pattern (for per-metric patterns)
    pub fn new_without_pattern(lines: usize) -> Self {
        Self { lines, pattern: None }
    }

    /// Parse a metric value using regex from logs
    /// Public so it can be used with per-metric patterns
    pub fn parse_with_regex(text: &str, pattern: &str) -> Option<f64> {
        let re = Regex::new(pattern).ok()?;

        // Search through lines in reverse order (newest first)
        for line in text.lines().rev() {
            if let Some(captures) = re.captures(line) {
                // Try to find first capture group, or use entire match
                let value_str = captures.get(1)
                    .or_else(|| captures.get(0))
                    .map(|m| m.as_str())?;

                if let Ok(value) = value_str.parse::<f64>() {
                    return Some(value);
                }
            }
        }

        None
    }
}

impl MetricFetcher for LogsFetcher {
    async fn fetch_raw(&self, container_name: &str) -> Result<String> {
        let output = tokio::process::Command::new("docker")
            .args(&[
                "logs",
                "--tail",
                &self.lines.to_string(),
                container_name,
            ])
            .output()
            .await
            .context("Failed to execute docker logs command")?;

        if !output.status.success() {
            return Ok(String::new());
        }

        // Logs might be in stderr or stdout
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        Ok(format!("{}\n{}", stdout, stderr))
    }

    async fn fetch_metric(&self, container_name: &str, _metric_name: &str) -> Result<Option<f64>> {
        let raw = self.fetch_raw(container_name).await?;

        // If fetcher has a global pattern, use it
        // Otherwise, return None (pattern will be provided per-metric by the registry)
        if let Some(pattern) = &self.pattern {
            Ok(Self::parse_with_regex(&raw, pattern))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_with_regex() {
        let logs = r#"
2025-10-25 10:00:00 INFO Starting service
2025-10-25 10:00:01 INFO Block height: 12345
2025-10-25 10:00:02 INFO Processing transactions
2025-10-25 10:00:03 INFO Block height: 12346
"#;

        // Should find the most recent match (newest first)
        assert_eq!(
            LogsFetcher::parse_with_regex(logs, r"Block height: (\d+)"),
            Some(12346.0)
        );

        assert_eq!(
            LogsFetcher::parse_with_regex(logs, r"Nonexistent: (\d+)"),
            None
        );
    }

    #[test]
    fn test_parse_floating_point() {
        let logs = "Sync progress: 85.7%";

        assert_eq!(
            LogsFetcher::parse_with_regex(logs, r"Sync progress: ([\d.]+)%"),
            Some(85.7)
        );
    }
}
