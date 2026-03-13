// OAuth2 token management for the GOG API.

use crate::error::GogError;
use serde::Deserialize;
use std::time::Instant;

const TOKEN_URL: &str = "https://auth.gog.com/token";
const CLIENT_ID: &str = "46899977096215655";
const CLIENT_SECRET: &str = "9d85c43b1482497dbbce61f6e4aa173a433796eeae2ca8c5f6129f2dc4de46d9";
const REDIRECT_URI: &str = "https://embed.gog.com/on_login_success?origin=client";

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

    /// Return the URL the user should open in their browser to log in.
    #[must_use]
    pub fn login_url() -> String {
        format!(
            "https://auth.gog.com/auth\
             ?client_id={CLIENT_ID}\
             &redirect_uri={}\
             &response_type=code\
             &layout=client2",
            urlencoding::encode(REDIRECT_URI),
        )
    }

    /// Exchange an authorization code for tokens.
    ///
    /// `input` can be the raw code, the full redirect URL, or anything
    /// in between — the code will be extracted automatically.
    ///
    /// # Errors
    ///
    /// Returns an error if no code can be extracted, or the token
    /// exchange request fails.
    pub fn from_authorization_code(input: &str) -> Result<Self, GogError> {
        let code = Self::extract_code(input)?;

        let url = format!(
            "{TOKEN_URL}?client_id={CLIENT_ID}\
             &client_secret={CLIENT_SECRET}\
             &grant_type=authorization_code\
             &code={code}\
             &redirect_uri={}",
            urlencoding::encode(REDIRECT_URI),
        );

        let body: String = ureq::get(&url)
            .call()
            .map_err(|e| GogError::AuthFailed(e.to_string()))?
            .body_mut()
            .read_to_string()
            .map_err(|e| GogError::AuthFailed(e.to_string()))?;

        let resp: TokenResponse =
            serde_json::from_str(&body).map_err(|e| GogError::AuthFailed(e.to_string()))?;

        let lifetime = resp.expires_in.saturating_sub(60);

        Ok(Self {
            refresh_token: resp.refresh_token,
            access: Some(AccessToken {
                token: resp.access_token,
                expires_at: Instant::now() + std::time::Duration::from_secs(lifetime),
            }),
        })
    }

    /// Extract the authorization code from user input.
    ///
    /// Accepts:
    /// - A bare code (e.g. `"abc123def456"`)
    /// - A full URL (e.g. `"https://embed.gog.com/on_login_success?origin=client&code=abc123"`)
    /// - A URL fragment pasted with extra whitespace
    fn extract_code(input: &str) -> Result<String, GogError> {
        let input = input.trim();

        // If it looks like a URL, try to pull the `code` query param.
        if input.starts_with("http://") || input.starts_with("https://") {
            // Find `code=` in the query string.
            if let Some(query_start) = input.find('?') {
                let query = &input[query_start + 1..];
                for pair in query.split('&') {
                    if let Some(value) = pair.strip_prefix("code=") {
                        let code = value.trim();
                        if code.is_empty() {
                            break;
                        }
                        return Ok(code.to_owned());
                    }
                }
            }
            return Err(GogError::AuthFailed(
                "URL does not contain a 'code' parameter".into(),
            ));
        }

        // Otherwise treat the whole thing as a bare code.
        if input.is_empty() {
            return Err(GogError::AuthFailed("empty authorization code".into()));
        }
        Ok(input.to_owned())
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
