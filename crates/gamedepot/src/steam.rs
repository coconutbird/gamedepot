// Adapter that wraps the standalone `steamcmd` crate and implements the
// `Depot` trait.

use std::path::Path;

use crate::depot::{AppInfo, AppStatus, Depot, DepotError, SearchResult};

/// Re-export steamcmd types that callers need for construction.
pub use steamcmd::{Login, Platform};

/// A [`Depot`] backed by `SteamCMD`.
#[derive(Debug, Clone)]
pub struct SteamDepot {
    cmd: steamcmd::SteamCmd,
}

impl SteamDepot {
    /// Create from an explicit steamcmd binary path.
    ///
    /// # Errors
    ///
    /// Returns an error if the path does not exist.
    pub fn new(path: impl Into<std::path::PathBuf>) -> Result<Self, DepotError> {
        let cmd = steamcmd::SteamCmd::new(path).map_err(|e| DepotError::Other(e.to_string()))?;
        Ok(Self { cmd })
    }

    /// Locate steamcmd on `$PATH` or at `~/steamcmd`.
    ///
    /// # Errors
    ///
    /// Returns an error if steamcmd is not found.
    pub fn locate() -> Result<Self, DepotError> {
        let cmd = steamcmd::SteamCmd::locate().map_err(map_err)?;
        Ok(Self { cmd })
    }

    /// Download and install steamcmd to `~/steamcmd`.
    ///
    /// # Errors
    ///
    /// Returns an error if the download or extraction fails.
    pub fn install() -> Result<Self, DepotError> {
        let cmd = steamcmd::SteamCmd::install().map_err(map_err)?;
        Ok(Self { cmd })
    }

    /// Try to locate steamcmd; if not found, download and install it.
    ///
    /// # Errors
    ///
    /// Returns an error if steamcmd cannot be located or installed.
    pub fn install_or_locate() -> Result<Self, DepotError> {
        let cmd = steamcmd::SteamCmd::install_or_locate().map_err(map_err)?;
        Ok(Self { cmd })
    }

    /// Set the login credentials.
    #[must_use]
    pub fn with_login(self, login: Login) -> Self {
        Self {
            cmd: self.cmd.with_login(login),
        }
    }

    /// Set the target platform override.
    #[must_use]
    pub fn with_platform(self, platform: Platform) -> Self {
        Self {
            cmd: self.cmd.with_platform(platform),
        }
    }

    /// Search the Steam Store without needing a steamcmd instance.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request or response parsing fails.
    pub fn search_store(query: &str) -> Result<Vec<SearchResult>, DepotError> {
        search_steam_store(query)
    }

    /// List locally installed apps without needing a steamcmd instance.
    ///
    /// # Errors
    ///
    /// Returns an error if the steamapps directory cannot be read.
    pub fn list_installed() -> Result<Vec<AppStatus>, DepotError> {
        list_installed_apps()
    }

    /// Download with a per-line progress callback.
    ///
    /// # Errors
    ///
    /// Returns an error if the download fails.
    pub fn download_with_progress(
        &self,
        app_id: &str,
        install_dir: &Path,
        validate: bool,
        on_info: impl FnOnce(&AppInfo),
        on_progress: impl FnMut(&steamcmd::DownloadProgress),
    ) -> Result<AppInfo, DepotError> {
        let info = self
            .cmd
            .download_with_progress(
                app_id,
                install_dir,
                validate,
                |steam_info| {
                    on_info(&AppInfo {
                        app_id: steam_info.app_id.clone(),
                        name: steam_info.name.clone(),
                        build_id: steam_info.build_id.clone(),
                    });
                },
                on_progress,
            )
            .map_err(map_err)?;
        Ok(AppInfo {
            app_id: info.app_id,
            name: info.name,
            build_id: info.build_id,
        })
    }
}

impl Depot for SteamDepot {
    fn download(&self, app_id: &str, install_dir: &Path, validate: bool) -> Result<(), DepotError> {
        self.cmd
            .download(app_id, install_dir, validate)
            .map_err(map_err)?;
        Ok(())
    }

    fn app_info(&self, app_id: &str) -> Result<AppInfo, DepotError> {
        let info = self.cmd.app_info(app_id).map_err(map_err)?;
        Ok(AppInfo {
            app_id: info.app_id,
            name: info.name,
            build_id: info.build_id,
        })
    }

    fn app_status(&self, app_id: &str) -> Result<AppStatus, DepotError> {
        let status = self.cmd.app_status(app_id).map_err(map_err)?;
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

    fn list(&self) -> Result<Vec<AppStatus>, DepotError> {
        list_installed_apps()
    }

    fn validate(&self, app_id: &str, install_dir: &Path) -> Result<(), DepotError> {
        self.cmd
            .download(app_id, install_dir, true)
            .map_err(map_err)?;
        Ok(())
    }

    fn update(&self, app_id: &str, install_dir: &Path) -> Result<(), DepotError> {
        self.cmd
            .download(app_id, install_dir, false)
            .map_err(map_err)?;
        Ok(())
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
    let body: String = ureq::get(&url)
        .call()
        .map_err(|e| DepotError::Other(format!("store search request failed: {e}")))?
        .body_mut()
        .read_to_string()
        .map_err(|e| DepotError::Other(format!("failed to read search response: {e}")))?;
    let response: StoreSearchResponse = serde_json::from_str(&body)
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

/// Scan the local `steamapps` directory for installed apps via `.acf` files.
fn list_installed_apps() -> Result<Vec<AppStatus>, DepotError> {
    let steamapps = find_steamapps_dir()?;
    let mut apps = Vec::new();

    let entries = std::fs::read_dir(&steamapps)
        .map_err(|e| DepotError::Other(format!("failed to read {}: {e}", steamapps.display())))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "acf")
            && let Some(status) = parse_acf_file(&path)
        {
            apps.push(status);
        }
    }

    Ok(apps)
}

/// Find the default steamapps directory for the current OS.
fn find_steamapps_dir() -> Result<std::path::PathBuf, DepotError> {
    let home = dirs_for_steamapps()?;
    for candidate in &home {
        if candidate.is_dir() {
            return Ok(candidate.clone());
        }
    }
    Err(DepotError::Other(
        "could not find steamapps directory".into(),
    ))
}

/// Return candidate steamapps directories for the current OS.
fn dirs_for_steamapps() -> Result<Vec<std::path::PathBuf>, DepotError> {
    let home = home_dir()?;
    let mut dirs = Vec::new();

    #[cfg(target_os = "macos")]
    {
        dirs.push(home.join("Library/Application Support/Steam/steamapps"));
    }

    #[cfg(target_os = "linux")]
    {
        dirs.push(home.join(".steam/steam/steamapps"));
        dirs.push(home.join(".local/share/Steam/steamapps"));
    }

    #[cfg(target_os = "windows")]
    {
        dirs.push(std::path::PathBuf::from(
            "C:\\Program Files (x86)\\Steam\\steamapps",
        ));
        dirs.push(std::path::PathBuf::from(
            "C:\\Program Files\\Steam\\steamapps",
        ));
    }

    // Also check the steamcmd install location.
    if let Ok(install_dir) = steamcmd::install::default_install_dir() {
        dirs.push(install_dir.join("steamapps"));
    }

    Ok(dirs)
}

/// Get the user's home directory.
fn home_dir() -> Result<std::path::PathBuf, DepotError> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .map_err(|_| DepotError::Other("could not determine home directory".into()))
}

/// Parse a `.acf` manifest file into an `AppStatus`.
fn parse_acf_file(path: &Path) -> Option<AppStatus> {
    let content = std::fs::read_to_string(path).ok()?;
    let app_id = extract_acf_value(&content, "appid")?;
    let name = extract_acf_value(&content, "name");
    let build_id = extract_acf_value(&content, "buildid");
    let size_on_disk =
        extract_acf_value(&content, "SizeOnDisk").and_then(|s| s.parse::<u64>().ok());
    let state_flags = extract_acf_value(&content, "StateFlags").and_then(|s| s.parse::<u32>().ok());
    let installed = state_flags == Some(4);

    Some(AppStatus {
        app_id,
        name: Some(name.unwrap_or_default()),
        build_id,
        size_on_disk,
        installed,
    })
}

/// Extract a quoted value from a VDF/ACF key-value file.
fn extract_acf_value(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        // Match patterns like:   "appid"		"730"
        if let Some(rest) = trimmed.strip_prefix('"')
            && let Some(rest) = rest.strip_prefix(key)
            && let Some(rest) = rest.strip_prefix('"')
        {
            let rest = rest.trim();
            if let Some(rest) = rest.strip_prefix('"')
                && let Some(val) = rest.strip_suffix('"')
            {
                return Some(val.to_string());
            }
        }
    }
    None
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
