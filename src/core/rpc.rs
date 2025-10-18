/// RPC endpoint testing

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde_json::json;

pub struct RpcTester {
    client: Client,
}

#[derive(Debug)]
pub struct RpcTestResult {
    pub success: bool,
    pub block_number: Option<String>,
    pub response_time_ms: u128,
    pub error: Option<String>,
}

impl RpcTester {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .danger_accept_invalid_certs(true) // For self-signed certs
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Test RPC endpoint with eth_blockNumber call
    pub async fn test_endpoint(&self, url: &str, token: Option<&str>) -> Result<RpcTestResult> {
        let start = std::time::Instant::now();

        // Construct full URL with token if provided
        let full_url = if let Some(t) = token {
            format!("{}/{}", url.trim_end_matches('/'), t)
        } else {
            url.to_string()
        };

        // eth_blockNumber request
        let payload = json!({
            "jsonrpc": "2.0",
            "method": "eth_blockNumber",
            "params": [],
            "id": 1
        });

        let response = match self.client
            .post(&full_url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                let elapsed = start.elapsed().as_millis();
                return Ok(RpcTestResult {
                    success: false,
                    block_number: None,
                    response_time_ms: elapsed,
                    error: Some(format!("Request failed: {}", e)),
                });
            }
        };

        let elapsed = start.elapsed().as_millis();
        let status = response.status();

        if !status.is_success() {
            return Ok(RpcTestResult {
                success: false,
                block_number: None,
                response_time_ms: elapsed,
                error: Some(format!("HTTP {}", status)),
            });
        }

        let json: serde_json::Value = match response.json().await {
            Ok(j) => j,
            Err(e) => {
                return Ok(RpcTestResult {
                    success: false,
                    block_number: None,
                    response_time_ms: elapsed,
                    error: Some(format!("Invalid JSON response: {}", e)),
                });
            }
        };

        if let Some(error) = json.get("error") {
            return Ok(RpcTestResult {
                success: false,
                block_number: None,
                response_time_ms: elapsed,
                error: Some(format!("RPC error: {}", error)),
            });
        }

        let block_number = json
            .get("result")
            .and_then(|r| r.as_str())
            .map(|s| s.to_string());

        Ok(RpcTestResult {
            success: true,
            block_number,
            response_time_ms: elapsed,
            error: None,
        })
    }

    /// Test both HTTP and HTTPS endpoints
    pub async fn test_both_endpoints(
        &self,
        domain: &str,
        token: &str,
    ) -> Result<(RpcTestResult, RpcTestResult)> {
        let http_url = format!("http://{}:8545", domain);
        let https_url = format!("https://{}:8545", domain);

        let http_result = self.test_endpoint(&http_url, Some(token)).await?;
        let https_result = self.test_endpoint(&https_url, Some(token)).await?;

        Ok((http_result, https_result))
    }
}
