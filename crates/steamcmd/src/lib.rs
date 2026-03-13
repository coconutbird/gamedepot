// Standalone wrapper around the SteamCMD binary.

pub mod cmd;
mod error;
pub mod install;
mod parse;
mod runner;

pub use error::SteamCmdError;
pub use runner::Output;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Target platform for downloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    MacOS,
    Linux,
}

impl Platform {
    /// Returns the string value steamcmd expects for
    /// `@sSteamCmdForcePlatformType`.
    #[must_use]
    pub fn as_steamcmd_str(self) -> &'static str {
        match self {
            Self::Windows => "windows",
            Self::MacOS => "macos",
            Self::Linux => "linux",
        }
    }
}

/// Login credentials for steamcmd.
#[derive(Debug, Clone)]
pub enum Login {
    /// Anonymous login — works for free games and dedicated servers.
    Anonymous,
    /// Authenticated login with username and password.
    Credentials { username: String, password: String },
}

/// Information about a Steam app from `app_info_print`.
#[derive(Debug, Clone)]
pub struct AppInfo {
    pub app_id: String,
    pub name: Option<String>,
    pub build_id: Option<String>,
    pub raw: HashMap<String, String>,
}

/// Local install status from `app_status`.
#[derive(Debug, Clone)]
pub struct AppStatus {
    pub app_id: String,
    pub name: Option<String>,
    pub install_dir: Option<String>,
    pub build_id: Option<String>,
    pub size_on_disk: Option<u64>,
    pub state_flags: Option<u32>,
    pub update_success: Option<bool>,
}

impl AppStatus {
    /// Returns `true` if the app appears fully installed (state flags == 4).
    #[must_use]
    pub fn is_installed(&self) -> bool {
        self.state_flags == Some(4)
    }

    /// Compare local build ID against remote to check for updates.
    #[must_use]
    pub fn needs_update(&self, remote: &AppInfo) -> Option<bool> {
        match (&self.build_id, &remote.build_id) {
            (Some(local), Some(remote)) => Some(local != remote),
            _ => None,
        }
    }
}

/// Wrapper around the steamcmd binary.
#[derive(Debug, Clone)]
pub struct SteamCmd {
    path: PathBuf,
    login: Login,
    platform: Option<Platform>,
}

impl SteamCmd {
    /// Create a `SteamCmd` instance from an explicit binary path.
    ///
    /// # Errors
    ///
    /// Returns an error if the path does not exist.
    pub fn new(path: impl Into<PathBuf>) -> Result<Self, SteamCmdError> {
        let path = path.into();
        if !path.exists() {
            return Err(SteamCmdError::InvalidPath { path });
        }
        Ok(Self {
            path,
            login: Login::Anonymous,
            platform: None,
        })
    }

    /// Locate steamcmd on `$PATH` or at `~/steamcmd`.
    ///
    /// # Errors
    ///
    /// Returns an error if steamcmd is not found.
    pub fn locate() -> Result<Self, SteamCmdError> {
        // Check $PATH first.
        if let Ok(path) = which::which("steamcmd") {
            return Ok(Self {
                path,
                login: Login::Anonymous,
                platform: None,
            });
        }

        // Check the default install location.
        let install_dir = install::default_install_dir()?;
        let bin = install::binary_path(&install_dir);
        if bin.exists() {
            return Ok(Self {
                path: bin,
                login: Login::Anonymous,
                platform: None,
            });
        }

        Err(SteamCmdError::NotFound(
            "steamcmd not found on $PATH or at ~/steamcmd".into(),
        ))
    }

    /// Download and install steamcmd to `~/steamcmd`.
    ///
    /// # Errors
    ///
    /// Returns an error if the download or extraction fails.
    pub fn install() -> Result<Self, SteamCmdError> {
        let install_dir = install::default_install_dir()?;
        let bin = install::install(&install_dir)?;
        Ok(Self {
            path: bin,
            login: Login::Anonymous,
            platform: None,
        })
    }

    /// Try to locate steamcmd; if not found, download and install it.
    ///
    /// # Errors
    ///
    /// Returns an error if steamcmd cannot be located or installed.
    pub fn install_or_locate() -> Result<Self, SteamCmdError> {
        Self::locate().or_else(|_| Self::install())
    }

    /// Set the login credentials.
    #[must_use]
    pub fn with_login(mut self, login: Login) -> Self {
        self.login = login;
        self
    }

    /// Set the target platform override.
    #[must_use]
    pub fn with_platform(mut self, platform: Platform) -> Self {
        self.platform = Some(platform);
        self
    }

    /// Returns the path to the steamcmd binary.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Download or update an app.
    ///
    /// # Errors
    ///
    /// Returns an error if steamcmd fails.
    pub fn download(
        &self,
        app_id: &str,
        install_dir: &Path,
        validate: bool,
    ) -> Result<(), SteamCmdError> {
        let args = cmd::CommandBuilder::new()
            .maybe_platform(self.platform)
            .force_install_dir(install_dir)
            .login(&self.login)
            .app_update(app_id, validate)
            .quit()
            .build();

        runner::run(&self.path, &args)?;
        Ok(())
    }

    /// Query app info from Steam's servers.
    ///
    /// The command is issued twice as a workaround for a known steamcmd
    /// bug where the first call may produce no output.
    ///
    /// # Errors
    ///
    /// Returns an error if steamcmd fails.
    pub fn app_info(&self, app_id: &str) -> Result<AppInfo, SteamCmdError> {
        let args = cmd::CommandBuilder::new()
            .login(&self.login)
            .app_info_update()
            .app_info_print(app_id)
            .app_info_print(app_id)
            .quit()
            .build();

        let output = runner::run(&self.path, &args)?;
        Ok(parse::parse_app_info(app_id, &output.stdout))
    }

    /// Check the local install status of an app.
    ///
    /// # Errors
    ///
    /// Returns an error if steamcmd fails.
    pub fn app_status(&self, app_id: &str) -> Result<AppStatus, SteamCmdError> {
        let args = cmd::CommandBuilder::new()
            .login(&self.login)
            .app_status(app_id)
            .quit()
            .build();

        let output = runner::run(&self.path, &args)?;
        Ok(parse::parse_app_status(app_id, &output.stdout))
    }
}
