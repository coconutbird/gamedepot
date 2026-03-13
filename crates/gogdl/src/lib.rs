// GOG game downloader using the GOG REST API.

pub mod api;
pub mod auth;
mod error;
pub mod types;

pub use error::GogError;

/// Target platform for downloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    MacOS,
    Linux,
}

impl Platform {
    /// Returns the OS string the GOG API expects.
    #[must_use]
    pub fn as_gog_str(self) -> &'static str {
        match self {
            Self::Windows => "windows",
            Self::MacOS => "osx",
            Self::Linux => "linux",
        }
    }
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Windows => write!(f, "Windows"),
            Self::MacOS => write!(f, "macOS"),
            Self::Linux => write!(f, "Linux"),
        }
    }
}

/// Structured progress update during a download.
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    /// Bytes downloaded so far.
    pub current_bytes: u64,
    /// Total bytes expected.
    pub total_bytes: u64,
}

/// Information about a GOG product.
#[derive(Debug, Clone)]
pub struct AppInfo {
    /// GOG product ID.
    pub product_id: String,
    /// Human-readable product name.
    pub name: Option<String>,
    /// Latest build ID (if builds are available).
    pub build_id: Option<String>,
    /// Whether the product supports Windows.
    pub windows: bool,
    /// Whether the product supports macOS.
    pub macos: bool,
    /// Whether the product supports Linux.
    pub linux: bool,
}

/// GOG game downloader.
///
/// Talks directly to the GOG REST API — no external binary needed.
/// Authentication is only required for downloading; searching and
/// querying product info work without a token.
///
/// Locale, country, and currency default to en-US / US / USD but can
/// be overridden with the builder methods.
pub struct GogDl {
    client: api::Client,
    platform: Option<Platform>,
    token_store: Option<auth::TokenStore>,
}

impl GogDl {
    /// Create a new `GogDl` instance with default locale settings.
    #[must_use]
    pub fn new() -> Self {
        Self {
            client: api::Client::new(),
            platform: None,
            token_store: None,
        }
    }

    /// Set the target platform for downloads and build queries.
    #[must_use]
    pub fn with_platform(mut self, platform: Platform) -> Self {
        self.platform = Some(platform);
        self
    }

    /// Set a GOG refresh token for authenticated requests.
    ///
    /// The token will be automatically exchanged for short-lived
    /// access tokens as needed.
    #[must_use]
    pub fn with_refresh_token(mut self, refresh_token: impl Into<String>) -> Self {
        self.token_store = Some(auth::TokenStore::new(refresh_token));
        self
    }

    /// Override the locale (e.g. `"de-DE"`).
    #[must_use]
    pub fn with_locale(mut self, locale: impl Into<String>) -> Self {
        self.client.locale = locale.into();
        self
    }

    /// Override the country code (e.g. `"DE"`).
    #[must_use]
    pub fn with_country(mut self, country_code: impl Into<String>) -> Self {
        self.client.country_code = country_code.into();
        self
    }

    /// Override the currency code (e.g. `"EUR"`).
    #[must_use]
    pub fn with_currency(mut self, currency_code: impl Into<String>) -> Self {
        self.client.currency_code = currency_code.into();
        self
    }

    /// Search the GOG catalog.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request or JSON parsing fails.
    pub fn search(&self, query: &str) -> Result<Vec<types::CatalogProduct>, GogError> {
        let resp = self.client.search(query)?;
        Ok(resp.products)
    }

    /// Fetch product info from the GOG API.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request or JSON parsing fails.
    pub fn app_info(&self, product_id: &str) -> Result<AppInfo, GogError> {
        let product = self.client.product_info(product_id)?;

        let (windows, macos, linux) = if let Some(ref compat) = product.content_system_compatibility
        {
            (compat.windows, compat.osx, compat.linux)
        } else {
            (false, false, false)
        };

        // Try to get the latest build ID for the target platform.
        let build_id = self.latest_build_id(product_id);

        Ok(AppInfo {
            product_id: product.id.to_string(),
            name: Some(product.title),
            build_id,
            windows,
            macos,
            linux,
        })
    }

    /// Fetch available builds for a product on the configured platform.
    ///
    /// # Errors
    ///
    /// Returns an error if no platform is set, or the request fails.
    pub fn builds(&self, product_id: &str) -> Result<Vec<types::BuildItem>, GogError> {
        let os = self
            .platform
            .ok_or_else(|| GogError::Other("no platform set".into()))?
            .as_gog_str();
        let resp = self.client.builds(product_id, os)?;
        Ok(resp.items)
    }

    /// List or search products owned by the authenticated user.
    ///
    /// Pass `None` to list all owned products, or `Some("query")` to
    /// filter by name. Results are paginated (page is 1-based).
    ///
    /// # Errors
    ///
    /// Returns an error if no refresh token is set, the token exchange
    /// fails, or the request fails.
    pub fn owned_products(
        &mut self,
        search: Option<&str>,
        page: u32,
    ) -> Result<Vec<types::OwnedProduct>, GogError> {
        self.ensure_authed()?;
        let resp = self.client.owned_products(search, page)?;
        Ok(resp.products)
    }

    /// Return the current refresh token (it rotates on every exchange).
    ///
    /// Useful for persisting the latest token to disk so the user
    /// doesn't have to re-authenticate.
    ///
    /// # Errors
    ///
    /// Returns an error if no token store is configured.
    pub fn refresh_token(&self) -> Result<&str, GogError> {
        self.token_store
            .as_ref()
            .map(auth::TokenStore::refresh_token)
            .ok_or_else(|| GogError::AuthRequired("no refresh token configured".into()))
    }

    /// Get the latest build ID for the configured platform, if any.
    fn latest_build_id(&self, product_id: &str) -> Option<String> {
        let os = self.platform?.as_gog_str();
        let resp = self.client.builds(product_id, os).ok()?;
        resp.items.first().map(|b| b.build_id.clone())
    }

    /// Ensure the API client has a valid access token.
    fn ensure_authed(&mut self) -> Result<(), GogError> {
        let store = self
            .token_store
            .as_mut()
            .ok_or_else(|| GogError::AuthRequired("no refresh token configured".into()))?;
        let token = store.access_token()?.to_owned();
        self.client.token = Some(token);
        Ok(())
    }
}

impl Default for GogDl {
    fn default() -> Self {
        Self::new()
    }
}
