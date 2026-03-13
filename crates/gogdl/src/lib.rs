// GOG game downloader using the GOG REST API.

pub mod api;
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
}

impl GogDl {
    /// Create a new `GogDl` instance with default locale settings.
    #[must_use]
    pub fn new() -> Self {
        Self {
            client: api::Client::new(),
            platform: None,
        }
    }

    /// Set the target platform for downloads and build queries.
    #[must_use]
    pub fn with_platform(mut self, platform: Platform) -> Self {
        self.platform = Some(platform);
        self
    }

    /// Set an `OAuth2` access token for authenticated requests.
    #[must_use]
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.client.token = Some(token.into());
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

    /// Get the latest build ID for the configured platform, if any.
    fn latest_build_id(&self, product_id: &str) -> Option<String> {
        let os = self.platform?.as_gog_str();
        let resp = self.client.builds(product_id, os).ok()?;
        resp.items.first().map(|b| b.build_id.clone())
    }
}

impl Default for GogDl {
    fn default() -> Self {
        Self::new()
    }
}
