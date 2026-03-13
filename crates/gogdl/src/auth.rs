// OAuth2 token management for the GOG API.

use crate::error::GogError;
use serde::Deserialize;
use std::time::Instant;

const TOKEN_URL: &str = "https://auth.gog.com/token";
const CLIENT_ID: &str = "46899977096215655";
const CLIENT_SECRET: &str = "9d85c43b1482497dbbce61f6e4aa173a433796eeae2ca8c5f6129f2dc4de46d9";

/// Raw token response from the GOG auth endpoint.
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: u64,
}

/// A live access token with expiry tracking.
#[derive(Debug)]
struct AccessToken {
    token: String,
    expires_at: Instant,
}

impl AccessToken {
    fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }
}

/// Manages GOG `OAuth2` tokens.
///
/// Accepts a refresh token and transparently exchanges / refreshes it
/// for short-lived access tokens as needed. The refresh token is
/// updated on every exchange (GOG rotates them).
#[derive(Debug)]
pub struct TokenStore {
    refresh_token: String,
    access: Option<AccessToken>,
}

impl TokenStore {
    /// Create a new store from a refresh token.
    #[must_use]
    pub fn new(refresh_token: impl Into<String>) -> Self {
        Self {
            refresh_token: refresh_token.into(),
            access: None,
        }
    }

    /// Get a valid access token, refreshing if necessary.
    ///
    /// # Errors
    ///
    /// Returns an error if the token exchange request fails.
    pub fn access_token(&mut self) -> Result<&str, GogError> {
        if self.access.as_ref().is_none_or(AccessToken::is_expired) {
            self.refresh()?;
        }
        Ok(&self
            .access
            .as_ref()
            .ok_or_else(|| GogError::AuthFailed("no access token".into()))?
            .token)
    }

    /// Return the current refresh token (it rotates on every exchange).
    #[must_use]
    pub fn refresh_token(&self) -> &str {
        &self.refresh_token
    }

    /// Exchange the refresh token for a new access + refresh token pair.
    fn refresh(&mut self) -> Result<(), GogError> {
        let url = format!(
            "{TOKEN_URL}?client_id={CLIENT_ID}\
             &client_secret={CLIENT_SECRET}\
             &grant_type=refresh_token\
             &refresh_token={}",
            self.refresh_token,
        );
        let body: String = ureq::get(&url)
            .call()
            .map_err(|e| GogError::AuthFailed(e.to_string()))?
            .body_mut()
            .read_to_string()
            .map_err(|e| GogError::AuthFailed(e.to_string()))?;
        let resp: TokenResponse =
            serde_json::from_str(&body).map_err(|e| GogError::AuthFailed(e.to_string()))?;

        // Shave 60s off the expiry to avoid edge-case races.
        let lifetime = resp.expires_in.saturating_sub(60);

        self.refresh_token = resp.refresh_token;
        self.access = Some(AccessToken {
            token: resp.access_token,
            expires_at: Instant::now() + std::time::Duration::from_secs(lifetime),
        });
        Ok(())
    }
}
