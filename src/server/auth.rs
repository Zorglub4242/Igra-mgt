/// Authentication middleware for IGRA Web UI

use axum::{
    body::Body,
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// Check if request has valid authentication token
pub async fn auth_middleware(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    // Get token from Authorization header
    let auth_header = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok());

    let token = match auth_header {
        Some(header) => {
            // Support both "Bearer TOKEN" and just "TOKEN"
            if let Some(t) = header.strip_prefix("Bearer ") {
                Some(t)
            } else {
                Some(header)
            }
        }
        None => None,
    };

    // Get expected token from environment
    let expected_token = std::env::var("IGRA_WEB_TOKEN").ok();

    match (token, expected_token) {
        (Some(provided), Some(expected)) if provided == expected => {
            // Token is valid, proceed with request
            Ok(next.run(request).await)
        }
        (None, None) => {
            // No token configured, allow access (development mode)
            eprintln!("⚠️  Warning: IGRA_WEB_TOKEN not set - authentication disabled!");
            Ok(next.run(request).await)
        }
        _ => {
            // Invalid or missing token
            Err(unauthorized_response())
        }
    }
}

fn unauthorized_response() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({
            "success": false,
            "error": "Unauthorized - invalid or missing authentication token"
        })),
    )
        .into_response()
}

/// Generate a random secure token
pub fn generate_token() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    const TOKEN_LEN: usize = 32;
    let mut rng = rand::thread_rng();

    (0..TOKEN_LEN)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_token() {
        let token = generate_token();
        assert_eq!(token.len(), 32);
        assert!(token.chars().all(|c| c.is_alphanumeric()));
    }
}
