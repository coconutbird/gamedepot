// HTTP client for the GOG REST API.

mod config;
mod content_system;
mod friends;
mod gameplay;
mod presence;
mod products;
mod users;

use crate::error::GogError;

pub(crate) const CATALOG_URL: &str = "https://catalog.gog.com/v1/catalog";
pub(crate) const PRODUCT_URL: &str = "https://api.gog.com/products";
pub(crate) const BUILDS_URL: &str = "https://content-system.gog.com/products";
pub(crate) const EMBED_URL: &str = "https://embed.gog.com";
pub(crate) const CDN_V2_META: &str = "https://cdn.gog.com/content-system/v2/meta";
pub(crate) const USERS_URL: &str = "https://users.gog.com/users";
pub(crate) const CHAT_URL: &str = "https://chat.gog.com/users";
pub(crate) const GAMEPLAY_URL: &str = "https://gameplay.gog.com/clients";
pub(crate) const PRESENCE_URL: &str = "https://presence.gog.com";
pub(crate) const CFG_URL: &str = "https://cfg.gog.com";

/// HTTP client for the GOG REST API.
///
/// Holds locale, country, and currency settings that are sent with
/// every request. Construct via [`Client::new`] and override defaults
/// with the builder methods.
pub struct Client {
    /// ISO 639-1 locale (e.g. `"en-US"`).
    pub locale: String,
    /// ISO 3166-1 alpha-2 country code (e.g. `"US"`).
    pub country_code: String,
    /// ISO 4217 currency code (e.g. `"USD"`).
    pub currency_code: String,
    /// `OAuth2` access token for authenticated endpoints.
    pub token: Option<String>,
}

impl Client {
    /// Create a new client with default locale settings (en-US / US / USD).
    #[must_use]
    pub fn new() -> Self {
        Self {
            locale: "en-US".into(),
            country_code: "US".into(),
            currency_code: "USD".into(),
            token: None,
        }
    }

    /// Override the locale (e.g. `"de-DE"`).
    #[must_use]
    pub fn with_locale(mut self, locale: impl Into<String>) -> Self {
        self.locale = locale.into();
        self
    }

    /// Override the country code (e.g. `"DE"`).
    #[must_use]
    pub fn with_country(mut self, country_code: impl Into<String>) -> Self {
        self.country_code = country_code.into();
        self
    }

    /// Override the currency code (e.g. `"EUR"`).
    #[must_use]
    pub fn with_currency(mut self, currency_code: impl Into<String>) -> Self {
        self.currency_code = currency_code.into();
        self
    }

    /// Set an `OAuth2` access token for authenticated requests.
    #[must_use]
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    pub(crate) fn require_token(&self) -> Result<&str, GogError> {
        self.token
            .as_deref()
            .ok_or_else(|| GogError::AuthRequired("this endpoint requires a token".into()))
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn get_json<T: serde::de::DeserializeOwned>(url: &str) -> Result<T, GogError> {
    reqwest::blocking::get(url)
        .map_err(|e| GogError::Http(e.to_string()))?
        .json()
        .map_err(|e| GogError::Parse(e.to_string()))
}

pub(crate) fn get_json_authed<T: serde::de::DeserializeOwned>(
    url: &str,
    token: &str,
) -> Result<T, GogError> {
    reqwest::blocking::Client::new()
        .get(url)
        .bearer_auth(token)
        .send()
        .map_err(|e| GogError::Http(e.to_string()))?
        .json()
        .map_err(|e| GogError::Parse(e.to_string()))
}

/// Fetch a URL that returns zlib-compressed JSON and deserialize it.
pub(crate) fn get_zlib_json<T: serde::de::DeserializeOwned>(url: &str) -> Result<T, GogError> {
    use flate2::read::ZlibDecoder;
    use std::io::Read;

    let bytes = reqwest::blocking::get(url)
        .map_err(|e| GogError::Http(e.to_string()))?
        .bytes()
        .map_err(|e| GogError::Http(e.to_string()))?;

    let mut decoder = ZlibDecoder::new(&bytes[..]);
    let mut json_str = String::new();
    decoder
        .read_to_string(&mut json_str)
        .map_err(|e| GogError::Parse(format!("zlib decompression failed: {e}")))?;

    serde_json::from_str(&json_str).map_err(|e| GogError::Parse(e.to_string()))
}

/// Download a chunk from the CDN and return the raw (compressed) bytes.
///
/// # Errors
///
/// Returns an error if the HTTP request fails.
pub fn download_chunk(url: &str) -> Result<Vec<u8>, GogError> {
    let bytes = reqwest::blocking::get(url)
        .map_err(|e| GogError::Http(e.to_string()))?
        .bytes()
        .map_err(|e| GogError::Http(e.to_string()))?;
    Ok(bytes.to_vec())
}
