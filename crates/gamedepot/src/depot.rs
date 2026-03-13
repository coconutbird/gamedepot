// Generic depot interface for game download backends.

use std::path::Path;

/// Information about an app from a remote depot.
#[derive(Debug, Clone)]
pub struct AppInfo {
    /// App identifier (numeric string for Steam, slug for GOG, etc.).
    pub app_id: String,
    /// Human-readable app name.
    pub name: Option<String>,
    /// Remote build identifier used to detect updates.
    pub build_id: Option<String>,
    /// Canonical install directory name (e.g. "`HaloWarsDE`").
    pub install_dir: Option<String>,
}

/// A search result from the store.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// App identifier.
    pub app_id: String,
    /// Human-readable app name.
    pub name: String,
    /// Whether the app supports Windows.
    pub windows: bool,
    /// Whether the app supports macOS.
    pub macos: bool,
    /// Whether the app supports Linux.
    pub linux: bool,
}

/// Local install status of an app.
#[derive(Debug, Clone)]
pub struct AppStatus {
    /// App identifier.
    pub app_id: String,
    /// Human-readable app name.
    pub name: Option<String>,
    /// Local build identifier.
    pub build_id: Option<String>,
    /// Size on disk in bytes, if known.
    pub size_on_disk: Option<u64>,
    /// Whether the app is fully installed.
    pub installed: bool,
}

impl AppStatus {
    /// Compare local build ID against remote to check for updates.
    #[must_use]
    pub fn needs_update(&self, remote: &AppInfo) -> Option<bool> {
        match (&self.build_id, &remote.build_id) {
            (Some(local), Some(remote)) => Some(local != remote),
            _ => None,
        }
    }
}

/// Errors that can occur in any depot backend.
#[derive(Debug, thiserror::Error)]
pub enum DepotError {
    /// The depot tool binary was not found.
    #[error("{tool} not found: {details}")]
    ToolNotFound { tool: String, details: String },

    /// The depot tool exited with an error.
    #[error("{tool} failed (exit {code}): {stderr}")]
    ToolFailed {
        tool: String,
        code: i32,
        stderr: String,
    },

    /// An I/O error occurred.
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),

    /// Any other error.
    #[error("{0}")]
    Other(String),
}

/// A backend that can download and query games/apps.
pub trait Depot {
    /// Download or update an app into the given directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the download fails.
    fn download(&mut self, app_id: &str, install_dir: &Path) -> Result<(), DepotError>;

    /// Query remote app info (name, latest build ID, etc.).
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    fn app_info(&mut self, app_id: &str) -> Result<AppInfo, DepotError>;

    /// Check the local install status of an app.
    ///
    /// # Errors
    ///
    /// Returns an error if the status check fails.
    fn app_status(&mut self, app_id: &str) -> Result<AppStatus, DepotError>;

    /// Search the store for apps matching the given query.
    ///
    /// # Errors
    ///
    /// Returns an error if the search request fails.
    fn search(&self, query: &str) -> Result<Vec<SearchResult>, DepotError>;

    /// Validate existing files for an installed app.
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails.
    fn validate(&mut self, app_id: &str, install_dir: &Path) -> Result<(), DepotError>;

    /// Update an installed app to the latest version.
    ///
    /// # Errors
    ///
    /// Returns an error if the update fails.
    fn update(&mut self, app_id: &str, install_dir: &Path) -> Result<(), DepotError>;
}
