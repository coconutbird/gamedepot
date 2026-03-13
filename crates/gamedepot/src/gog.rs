// Adapter that wraps the standalone `gogapi` crate and exposes GOG
// functionality through the `gamedepot` core library.

use crate::depot::DepotError;

/// Re-export gogapi types that callers need.
pub use gogapi::types::{CatalogProduct, OwnedProduct, WorksOn};
pub use gogapi::{AppInfo, GogError, Platform};

/// A depot backed by the GOG REST API.
///
/// Wraps [`gogapi::GogDl`] so the CLI (and other consumers) only
/// depend on `gamedepot`, not on `gogapi` directly.
pub struct GogDepot {
    inner: gogapi::GogDl,
}

impl GogDepot {
    /// Create a new `GogDepot` with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: gogapi::GogDl::new(),
        }
    }

    /// Set the target platform for downloads and build queries.
    #[must_use]
    pub fn with_platform(mut self, platform: Platform) -> Self {
        self.inner = self.inner.with_platform(platform);
        self
    }

    /// Set a GOG refresh token for authenticated requests.
    #[must_use]
    pub fn with_refresh_token(mut self, token: impl Into<String>) -> Self {
        self.inner = self.inner.with_refresh_token(token);
        self
    }

    /// Return the URL the user should open in their browser to log in.
    #[must_use]
    pub fn login_url() -> String {
        gogapi::GogDl::login_url()
    }

    /// Complete login using the authorization code or redirect URL.
    ///
    /// # Errors
    ///
    /// Returns an error if the code exchange fails.
    pub fn login_with_code(&mut self, input: &str) -> Result<(), DepotError> {
        self.inner
            .login_with_code(input)
            .map_err(|e| DepotError::Other(e.to_string()))
    }

    /// Search the GOG catalog.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub fn search(&self, query: &str) -> Result<Vec<CatalogProduct>, DepotError> {
        self.inner
            .search(query)
            .map_err(|e| DepotError::Other(e.to_string()))
    }

    /// Fetch product info from the GOG API.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub fn app_info(&self, product_id: &str) -> Result<AppInfo, DepotError> {
        self.inner
            .app_info(product_id)
            .map_err(|e| DepotError::Other(e.to_string()))
    }

    /// List or search products owned by the authenticated user.
    ///
    /// # Errors
    ///
    /// Returns an error if not authenticated or the request fails.
    pub fn owned_products(
        &mut self,
        search: Option<&str>,
        page: u32,
    ) -> Result<Vec<OwnedProduct>, DepotError> {
        self.inner
            .owned_products(search, page)
            .map_err(|e| DepotError::Other(e.to_string()))
    }

    /// Return the current refresh token (rotates on every exchange).
    ///
    /// # Errors
    ///
    /// Returns an error if no token store is configured.
    pub fn refresh_token(&self) -> Result<&str, DepotError> {
        self.inner
            .refresh_token()
            .map_err(|e| DepotError::Other(e.to_string()))
    }
}

impl Default for GogDepot {
    fn default() -> Self {
        Self::new()
    }
}
