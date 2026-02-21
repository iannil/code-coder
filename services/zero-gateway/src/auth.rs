//! Authentication module for Zero Gateway.
//!
//! Supports JWT tokens and API keys.

use anyhow::Result;
use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// JWT claims structure.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// Expiration time (Unix timestamp)
    pub exp: usize,
    /// Issued at (Unix timestamp)
    pub iat: usize,
    /// User roles
    pub roles: Vec<String>,
}

/// Authentication state shared across requests.
#[derive(Clone)]
pub struct AuthState {
    pub jwt_secret: Arc<String>,
    pub token_expiry_secs: u64,
}

impl AuthState {
    /// Create a new auth state with the given JWT secret.
    pub fn new(jwt_secret: impl Into<String>, token_expiry_secs: u64) -> Self {
        Self {
            jwt_secret: Arc::new(jwt_secret.into()),
            token_expiry_secs,
        }
    }

    /// Generate a new JWT token for a user.
    pub fn generate_token(&self, user_id: &str, roles: Vec<String>) -> Result<String> {
        let now = chrono::Utc::now().timestamp() as usize;
        let exp = now + self.token_expiry_secs as usize;

        let claims = Claims {
            sub: user_id.to_string(),
            exp,
            iat: now,
            roles,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )?;

        Ok(token)
    }

    /// Validate a JWT token and return the claims.
    pub fn validate_token(&self, token: &str) -> Result<Claims> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &Validation::default(),
        )?;

        Ok(token_data.claims)
    }
}

/// User info extracted from authentication.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: String,
    pub roles: Vec<String>,
}

/// Authentication middleware.
pub async fn auth_middleware(
    auth_state: axum::extract::State<AuthState>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract token from Authorization header
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    let token = match auth_header.as_deref() {
        Some(h) if h.starts_with("Bearer ") => &h[7..],
        Some(h) if h.starts_with("ApiKey ") => {
            // API Key authentication - for now, just validate format
            let api_key = &h[7..];
            if api_key.len() < 32 {
                return Err(StatusCode::UNAUTHORIZED);
            }
            // In production, validate against stored API keys
            request.extensions_mut().insert(AuthUser {
                user_id: format!("apikey:{}", &api_key[..8]),
                roles: vec!["user".into()],
            });
            return Ok(next.run(request).await);
        }
        _ => return Err(StatusCode::UNAUTHORIZED),
    };

    // Validate JWT token
    match auth_state.validate_token(token) {
        Ok(claims) => {
            request.extensions_mut().insert(AuthUser {
                user_id: claims.sub,
                roles: claims.roles,
            });
            Ok(next.run(request).await)
        }
        Err(_) => Err(StatusCode::UNAUTHORIZED),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_roundtrip() {
        let auth = AuthState::new("test-secret-key-32-bytes-long!!", 3600);
        let token = auth.generate_token("user123", vec!["admin".into()]).unwrap();
        let claims = auth.validate_token(&token).unwrap();
        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.roles, vec!["admin"]);
    }

    #[test]
    fn test_invalid_token() {
        let auth = AuthState::new("test-secret-key-32-bytes-long!!", 3600);
        let result = auth.validate_token("invalid-token");
        assert!(result.is_err());
    }
}
