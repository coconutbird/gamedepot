#![allow(clippy::missing_errors_doc)]
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

/// Progress update during file verification.
#[derive(Debug, Clone)]
pub struct VerifyProgress {
    /// Number of files checked so far.
    pub checked: u64,
    /// Total number of files to check.
    pub total: u64,
    /// Number of files that passed verification.
    pub valid: u64,
    /// Number of files that need re-downloading.
    pub invalid: u64,
    /// The file currently being checked (if any).
    pub current_file: Option<String>,
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

    /// Return the URL the user should open in their browser to log in.
    #[must_use]
    pub fn login_url() -> String {
        auth::TokenStore::login_url()
    }

    /// Complete login using the authorization code or redirect URL.
    ///
    /// `input` can be:
    /// - The full redirect URL (e.g. `https://embed.gog.com/on_login_success?...&code=XYZ`)
    /// - Just the bare code value
    ///
    /// On success the instance is ready for authenticated requests.
    ///
    /// # Errors
    ///
    /// Returns an error if the code cannot be extracted or the token
    /// exchange fails.
    pub fn login_with_code(&mut self, input: &str) -> Result<(), GogError> {
        let store = auth::TokenStore::from_authorization_code(input)?;
        self.token_store = Some(store);
        Ok(())
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
            product_id: product.id,
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
    pub fn refresh_token(&self) -> Result<&str, GogError> {
        self.token_store
            .as_ref()
            .map(auth::TokenStore::refresh_token)
            .ok_or_else(|| GogError::AuthRequired("no refresh token configured".into()))
    }

    /// Fetch full product info with all expand fields.
    pub fn product_info_full(
        &self,
        product_id: &str,
    ) -> Result<types::ProductResponseFull, GogError> {
        self.client.product_info_full(product_id)
    }

    /// Fetch multiple products at once (up to 50).
    pub fn products_batch(&self, ids: &[&str]) -> Result<Vec<types::ProductResponse>, GogError> {
        self.client.products_batch(ids)
    }

    /// Get a secure download URL for an installer file.
    pub fn downlink(
        &mut self,
        product_id: &str,
        dl_path: &str,
    ) -> Result<types::DownlinkResponse, GogError> {
        self.ensure_authed()?;
        self.client.downlink(product_id, dl_path)
    }

    /// Fetch user profile information.
    pub fn user_info(&mut self, user_id: &str) -> Result<types::UserInfo, GogError> {
        self.ensure_authed()?;
        self.client.user_info(user_id)
    }

    /// Fetch the friends list for a user.
    pub fn friends(&mut self, user_id: &str) -> Result<types::FriendsResponse, GogError> {
        self.ensure_authed()?;
        self.client.friends(user_id)
    }

    /// Fetch achievements for a product and user.
    pub fn achievements(
        &mut self,
        product_id: &str,
        user_id: &str,
    ) -> Result<types::AchievementsResponse, GogError> {
        self.ensure_authed()?;
        self.client.achievements(product_id, user_id)
    }

    /// Set the user as online.
    pub fn set_online(&mut self, user_id: &str) -> Result<(), GogError> {
        self.ensure_authed()?;
        self.client.set_online(user_id)
    }

    /// Check which users from a list are currently online.
    pub fn statuses(&mut self, user_ids: &[&str]) -> Result<types::StatusesResponse, GogError> {
        self.ensure_authed()?;
        self.client.statuses(user_ids)
    }

    /// Fetch the Galaxy client configuration (no auth needed).
    pub fn galaxy_config() -> Result<types::GalaxyConfig, GogError> {
        api::Client::galaxy_config()
    }

    /// Resolve all files for a product into download-ready metadata.
    ///
    /// Fetches the latest V2 build, resolves depot manifests, obtains
    /// authenticated CDN URLs, and returns a flat list of files with
    /// pre-built chunk URLs. No data is downloaded or written to disk.
    ///
    /// # Errors
    ///
    /// Returns an error if auth fails, no builds are available, or
    /// any manifest fetch fails.
    pub fn resolve_files(
        &mut self,
        product_id: &str,
        language: &str,
    ) -> Result<Vec<types::ResolvedFile>, GogError> {
        self.ensure_authed()?;

        let os = self
            .platform
            .ok_or_else(|| GogError::Other("no platform set".into()))?
            .as_gog_str();

        // 1. Get the latest V2 build.
        let builds = self.client.builds(product_id, os)?;
        let build = builds
            .items
            .iter()
            .find(|b| b.generation == 2)
            .ok_or_else(|| GogError::NotFound("no V2 build available".into()))?;
        let link = build
            .link
            .as_deref()
            .ok_or_else(|| GogError::NotFound("build has no manifest link".into()))?;

        // 2. Fetch the repository manifest.
        let repo = self.client.v2_repository(link)?;

        // 3. Filter depots by language (include "*" = neutral).
        let matching_depots: Vec<_> = repo
            .depots
            .iter()
            .filter(|d| d.languages.iter().any(|l| l == "*" || l == language))
            .collect();

        if matching_depots.is_empty() {
            return Err(GogError::NotFound(format!(
                "no depots for language '{language}'"
            )));
        }

        // 4. Get authenticated CDN URLs via secure_link (one per product_id).
        let mut secure_urls: std::collections::HashMap<String, types::SecureUrl> =
            std::collections::HashMap::new();
        for depot_entry in &matching_depots {
            if !secure_urls.contains_key(&depot_entry.product_id) {
                let sl = self.client.secure_link(&depot_entry.product_id)?;
                if let Some(first) = sl.urls.into_iter().next() {
                    secure_urls.insert(depot_entry.product_id.clone(), first);
                }
            }
        }

        // 5. Walk depot manifests and build resolved file list.
        let mut files = Vec::new();
        for depot_entry in &matching_depots {
            let manifest = self.client.v2_depot_manifest(&depot_entry.manifest)?;
            let secure = secure_urls.get(&depot_entry.product_id);

            for item in manifest.depot.items {
                if item.item_type != "DepotFile" || item.chunks.is_empty() {
                    continue;
                }

                let path = item.path.replace('\\', "/");
                let chunks = item
                    .chunks
                    .iter()
                    .map(|c| {
                        let galaxy = api::Client::galaxy_path(&c.compressed_md5);
                        let url = secure
                            .map(|s| s.build_chunk_url(&galaxy))
                            .unwrap_or_default();
                        types::ResolvedChunk {
                            url,
                            compressed_size: c.compressed_size,
                            size: c.size,
                            md5: c.md5.clone(),
                            compressed_md5: c.compressed_md5.clone(),
                        }
                    })
                    .collect();

                files.push(types::ResolvedFile {
                    rel_path: path,
                    md5: item.md5,
                    chunks,
                });
            }
        }

        Ok(files)
    }

    /// Download and decompress a single chunk.
    ///
    /// Fetches the compressed data from the CDN, decompresses it with
    /// zlib, and returns the raw bytes. The caller is responsible for
    /// parallelism, ordering, and writing to disk.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request or decompression fails.
    pub fn download_chunk(chunk: &types::ResolvedChunk) -> Result<Vec<u8>, GogError> {
        use flate2::read::ZlibDecoder;
        use std::io::Read;

        let compressed = api::download_chunk(&chunk.url)?;

        let mut decoder = ZlibDecoder::new(&compressed[..]);
        #[allow(clippy::cast_possible_truncation)]
        let chunk_len = chunk.size as usize;
        let mut buf = vec![0u8; chunk_len];
        decoder
            .read_exact(&mut buf)
            .map_err(|e| GogError::Other(format!("chunk decompression failed: {e}")))?;

        Ok(buf)
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
