// HTTP client for the GOG REST API.

use crate::error::GogError;
use crate::types::{BuildsResponse, CatalogResponse, OwnedProductsResponse, ProductResponse};

const CATALOG_URL: &str = "https://catalog.gog.com/v1/catalog";
const PRODUCT_URL: &str = "https://api.gog.com/products";
const BUILDS_URL: &str = "https://content-system.gog.com/products";
const EMBED_URL: &str = "https://embed.gog.com";

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

    /// Search the GOG catalog for products matching a query.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request or JSON parsing fails.
    pub fn search(&self, query: &str) -> Result<CatalogResponse, GogError> {
        let url = format!(
            "{CATALOG_URL}?limit=20&order=desc:score\
             &productType=in:game,pack,dlc,extras\
             &countryCode={}&locale={}&currencyCode={}\
             &query={query}",
            self.country_code, self.locale, self.currency_code,
        );

        get_json(&url)
    }

    /// Fetch detailed product information by product ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request or JSON parsing fails.
    pub fn product_info(&self, product_id: &str) -> Result<ProductResponse, GogError> {
        let url = format!(
            "{PRODUCT_URL}/{product_id}?locale={}\
             &expand=downloads,expanded_dlcs",
            self.locale,
        );
        get_json(&url)
    }

    /// Fetch available builds for a product on a given OS.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request or JSON parsing fails.
    pub fn builds(&self, product_id: &str, os: &str) -> Result<BuildsResponse, GogError> {
        let url = format!("{BUILDS_URL}/{product_id}/os/{os}/builds?generation=2");
        get_json(&url)
    }

    /// List products owned by the authenticated user.
    ///
    /// Optionally filter by a search string. Results are paginated;
    /// pass the page number (1-based).
    ///
    /// # Errors
    ///
    /// Returns an error if no token is set, or the request fails.
    pub fn owned_products(
        &self,
        search: Option<&str>,
        page: u32,
    ) -> Result<OwnedProductsResponse, GogError> {
        let token = self
            .token
            .as_deref()
            .ok_or_else(|| GogError::AuthRequired("owned_products requires a token".into()))?;
        let mut url = format!("{EMBED_URL}/account/getFilteredProducts?mediaType=1&page={page}");
        if let Some(q) = search {
            url.push_str("&search=");
            url.push_str(q);
        }
        get_json_authed(&url, token)
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

fn get_json<T: serde::de::DeserializeOwned>(url: &str) -> Result<T, GogError> {
    reqwest::blocking::get(url)
        .map_err(|e| GogError::Http(e.to_string()))?
        .json()
        .map_err(|e| GogError::Parse(e.to_string()))
}

fn get_json_authed<T: serde::de::DeserializeOwned>(url: &str, token: &str) -> Result<T, GogError> {
    reqwest::blocking::Client::new()
        .get(url)
        .bearer_auth(token)
        .send()
        .map_err(|e| GogError::Http(e.to_string()))?
        .json()
        .map_err(|e| GogError::Parse(e.to_string()))
}
