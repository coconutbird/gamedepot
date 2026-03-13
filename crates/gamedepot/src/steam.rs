// Adapter that wraps the standalone `steamcmd` crate and implements the
// `Depot` trait.

use std::path::Path;

use crate::depot::{AppInfo, AppStatus, Depot, DepotError, SearchResult};

/// Re-export steamcmd types that callers need for construction.
pub use steamcmd::{Login, Platform};

/// Re-export steamapi types that callers need.
pub use steamapi::types::{OwnedGame, PlayerSummary};

/// A [`Depot`] backed by `SteamCMD` and the Steam Web API.
///
/// `cmd` is optional — create with [`SteamDepot::api_only`] when you
/// only need Web API features (owned games, vanity URL resolution)
/// without requiring a local `steamcmd` binary.
pub struct SteamDepot {
    cmd: Option<steamcmd::SteamCmd>,
    api: Option<steamapi::SteamApi>,
}

impl SteamDepot {
    /// Create from an explicit steamcmd binary path.
    ///
    /// # Errors
    ///
    /// Returns an error if the path does not exist.
    pub fn new(path: impl Into<std::path::PathBuf>) -> Result<Self, DepotError> {
        let cmd = steamcmd::SteamCmd::new(path).map_err(|e| DepotError::Other(e.to_string()))?;
        Ok(Self {
            cmd: Some(cmd),
            api: None,
        })
    }

    /// Create a `SteamDepot` for Web API use only (no steamcmd needed).
    #[must_use]
    pub fn api_only() -> Self {
        Self {
            cmd: None,
            api: None,
        }
    }

    /// Locate steamcmd on `$PATH` or at `~/steamcmd`.
    ///
    /// # Errors
    ///
    /// Returns an error if steamcmd is not found.
    pub fn locate() -> Result<Self, DepotError> {
        let cmd = steamcmd::SteamCmd::locate().map_err(map_err)?;
        Ok(Self {
            cmd: Some(cmd),
            api: None,
        })
    }

    /// Download and install steamcmd to `~/steamcmd`.
    ///
    /// # Errors
    ///
    /// Returns an error if the download or extraction fails.
    pub fn install() -> Result<Self, DepotError> {
        let cmd = steamcmd::SteamCmd::install().map_err(map_err)?;
        Ok(Self {
            cmd: Some(cmd),
            api: None,
        })
    }

    /// Try to locate steamcmd; if not found, download and install it.
    ///
    /// # Errors
    ///
    /// Returns an error if steamcmd cannot be located or installed.
    pub fn install_or_locate() -> Result<Self, DepotError> {
        let cmd = steamcmd::SteamCmd::install_or_locate().map_err(map_err)?;
        Ok(Self {
            cmd: Some(cmd),
            api: None,
        })
    }

    /// Set the login credentials.
    #[must_use]
    pub fn with_login(mut self, login: Login) -> Self {
        if let Some(cmd) = self.cmd.take() {
            self.cmd = Some(cmd.with_login(login));
        }
        self
    }

    /// Set the target platform override.
    #[must_use]
    pub fn with_platform(mut self, platform: Platform) -> Self {
        if let Some(cmd) = self.cmd.take() {
            self.cmd = Some(cmd.with_platform(platform));
        }
        self
    }

    /// Set the Steam Web API key for library queries.
    #[must_use]
    pub fn with_api_key(mut self, key: &str) -> Self {
        self.api = Some(steamapi::SteamApi::new(key));
        self
    }

    /// Return a reference to the inner steamcmd, or an error if this
    /// depot was created with [`SteamDepot::api_only`].
    fn require_cmd(&mut self) -> Result<&mut steamcmd::SteamCmd, DepotError> {
        self.cmd.as_mut().ok_or_else(|| DepotError::ToolNotFound {
            tool: "steamcmd".into(),
            details: "this SteamDepot has no steamcmd — use locate() or install()".into(),
        })
    }

    /// Search the Steam Store without needing a steamcmd instance.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request or response parsing fails.
    pub fn search_store(query: &str) -> Result<Vec<SearchResult>, DepotError> {
        search_steam_store(query)
    }

    /// Download with a per-line progress callback.
    ///
    /// # Errors
    ///
    /// Returns an error if the download fails.
    /// Download with a progress callback.
    ///
    /// # Errors
    ///
    /// Returns an error if the download fails.
    pub fn download_with_progress(
        &mut self,
        app_id: &str,
        install_dir: &Path,
        on_progress: impl FnMut(&steamcmd::DownloadProgress),
    ) -> Result<(), DepotError> {
        self.require_cmd()?
            .download_with_progress(app_id, install_dir, on_progress)
            .map_err(map_err)
    }

    /// Validate with a progress callback.
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails.
    pub fn validate_with_progress(
        &mut self,
        app_id: &str,
        install_dir: &Path,
        on_progress: impl FnMut(&steamcmd::DownloadProgress),
    ) -> Result<(), DepotError> {
        self.require_cmd()?
            .validate_with_progress(app_id, install_dir, on_progress)
            .map_err(map_err)
    }

    // ── Steam Web API methods ──────────────────────────────────────

    /// Return a reference to the inner API client, or an error if no
    /// API key has been set.
    fn require_api(&self) -> Result<&steamapi::SteamApi, DepotError> {
        self.api
            .as_ref()
            .ok_or_else(|| DepotError::Other("Steam API key required — call with_api_key()".into()))
    }

    /// List games owned by the given Steam ID.
    ///
    /// Set `include_appinfo` to `true` to get game names and icons.
    ///
    /// # Errors
    ///
    /// Returns an error if no API key is set, the request fails, or
    /// the response cannot be parsed.
    pub fn owned_games(
        &self,
        steam_id: &str,
        include_appinfo: bool,
    ) -> Result<Vec<OwnedGame>, DepotError> {
        self.require_api()?
            .owned_games(steam_id, include_appinfo)
            .map_err(|e| DepotError::Other(e.to_string()))
    }

    /// Look up a player's profile summary by Steam ID.
    ///
    /// # Errors
    ///
    /// Returns an error if no API key is set or the request fails.
    pub fn player_summary(&self, steam_id: &str) -> Result<Option<PlayerSummary>, DepotError> {
        self.require_api()?
            .player_summary(steam_id)
            .map_err(|e| DepotError::Other(e.to_string()))
    }

    /// Resolve a Steam vanity URL name to a 64-bit Steam ID.
    ///
    /// # Errors
    ///
    /// Returns an error if no API key is set, the request fails, or
    /// the vanity name doesn't match any user.
    pub fn resolve_vanity_url(&self, vanity_name: &str) -> Result<String, DepotError> {
        self.require_api()?
            .resolve_vanity_url(vanity_name)
            .map_err(|e| DepotError::Other(e.to_string()))
    }
}

impl Depot for SteamDepot {
    fn download(&mut self, app_id: &str, install_dir: &Path) -> Result<(), DepotError> {
        self.require_cmd()?
            .download(app_id, install_dir)
            .map_err(map_err)
    }

    fn app_info(&mut self, app_id: &str) -> Result<AppInfo, DepotError> {
        let info = self.require_cmd()?.app_info(app_id).map_err(map_err)?;
        Ok(AppInfo {
            app_id: info.app_id,
            name: info.name,
            build_id: info.build_id,
        })
    }

    fn app_status(&mut self, app_id: &str) -> Result<AppStatus, DepotError> {
        let status = self.require_cmd()?.app_status(app_id).map_err(map_err)?;
        let installed = status.is_installed();
        Ok(AppStatus {
            app_id: status.app_id,
            name: status.name,
            build_id: status.build_id,
            size_on_disk: status.size_on_disk,
            installed,
        })
    }

    fn search(&self, query: &str) -> Result<Vec<SearchResult>, DepotError> {
        search_steam_store(query)
    }

    fn validate(&mut self, app_id: &str, install_dir: &Path) -> Result<(), DepotError> {
        self.require_cmd()?
            .validate(app_id, install_dir)
            .map_err(map_err)
    }

    fn update(&mut self, app_id: &str, install_dir: &Path) -> Result<(), DepotError> {
        self.require_cmd()?
            .download(app_id, install_dir)
            .map_err(map_err)
    }
}

/// Response from the Steam Store search API.
#[derive(serde::Deserialize)]
struct StoreSearchResponse {
    items: Vec<StoreSearchItem>,
}

/// A single item from the Steam Store search API.
#[derive(serde::Deserialize)]
struct StoreSearchItem {
    id: u64,
    name: String,
    platforms: StoreSearchPlatforms,
}

/// Platform availability from the Steam Store search API.
#[derive(serde::Deserialize)]
struct StoreSearchPlatforms {
    windows: bool,
    mac: bool,
    linux: bool,
}

/// Search the Steam Store API for apps matching the given query.
fn search_steam_store(query: &str) -> Result<Vec<SearchResult>, DepotError> {
    let url = format!(
        "https://store.steampowered.com/api/storesearch/?term={}&l=english&cc=US",
        urlencoded(query)
    );
    let response: StoreSearchResponse = reqwest::blocking::get(&url)
        .map_err(|e| DepotError::Other(format!("store search request failed: {e}")))?
        .json()
        .map_err(|e| DepotError::Other(format!("failed to parse search response: {e}")))?;
    Ok(response
        .items
        .into_iter()
        .map(|item| SearchResult {
            app_id: item.id.to_string(),
            name: item.name,
            windows: item.platforms.windows,
            macos: item.platforms.mac,
            linux: item.platforms.linux,
        })
        .collect())
}

/// Minimal percent-encoding for URL query parameters.
fn urlencoded(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => {
                out.push('%');
                out.push(char::from(b"0123456789ABCDEF"[(b >> 4) as usize]));
                out.push(char::from(b"0123456789ABCDEF"[(b & 0xF) as usize]));
            }
        }
    }
    out
}

fn map_err(e: steamcmd::SteamCmdError) -> DepotError {
    match e {
        steamcmd::SteamCmdError::NotFound(details) => DepotError::ToolNotFound {
            tool: "steamcmd".into(),
            details,
        },
        steamcmd::SteamCmdError::NonZeroExit { code, stderr } => DepotError::ToolFailed {
            tool: "steamcmd".into(),
            code,
            stderr,
        },
        steamcmd::SteamCmdError::Io(e) => DepotError::Io(e),
        other => DepotError::Other(other.to_string()),
    }
}
