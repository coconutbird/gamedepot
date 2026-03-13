// Adapter that wraps the standalone `steamcmd` crate and implements the
// `Depot` trait.

use std::path::Path;

use crate::depot::{AppInfo, AppStatus, Depot, DepotError};

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
}

impl Depot for SteamDepot {
    fn download(&self, app_id: &str, install_dir: &Path, validate: bool) -> Result<(), DepotError> {
        self.cmd
            .download(app_id, install_dir, validate)
            .map_err(map_err)
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
