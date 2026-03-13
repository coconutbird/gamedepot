// Standalone wrapper around the SteamCMD binary.

pub mod cmd;
mod error;
pub mod install;
mod parse;
pub mod runner;

pub use error::SteamCmdError;
pub use runner::Session;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Phase of a download or update operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateState {
    /// Reconfiguring app metadata.
    Reconfiguring,
    /// Validating existing files.
    Validating,
    /// Pre-allocating disk space.
    Preallocating,
    /// Downloading new content.
    Downloading,
    /// Verifying downloaded content.
    Verifying,
    /// Staging files before final placement.
    Staging,
    /// Committing changes to disk.
    Committing,
    /// An unrecognized state.
    Unknown,
}

impl UpdateState {
    /// Parse from the text label steamcmd outputs (e.g. `"downloading"`).
    #[must_use]
    pub fn from_label(s: &str) -> Self {
        match s.trim() {
            "reconfiguring" => Self::Reconfiguring,
            "validating" => Self::Validating,
            "preallocating" => Self::Preallocating,
            "downloading" => Self::Downloading,
            "verifying update" => Self::Verifying,
            "staging" => Self::Staging,
            "committing" => Self::Committing,
            _ => Self::Unknown,
        }
    }
}

impl std::fmt::Display for UpdateState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Reconfiguring => write!(f, "reconfiguring"),
            Self::Validating => write!(f, "validating"),
            Self::Preallocating => write!(f, "preallocating"),
            Self::Downloading => write!(f, "downloading"),
            Self::Verifying => write!(f, "verifying"),
            Self::Staging => write!(f, "staging"),
            Self::Committing => write!(f, "committing"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

/// Structured progress update from a download or update operation.
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    /// Current phase of the operation.
    pub state: UpdateState,
    /// Completion percentage (0.0–100.0).
    pub percent: f64,
    /// Bytes processed so far.
    pub current_bytes: u64,
    /// Total bytes expected.
    pub total_bytes: u64,
}

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

/// A boxed callback that receives a Steam Guard / TOTP prompt and
/// returns the auth code entered by the user.
type AuthHandler = Box<dyn FnMut(&str) -> String>;

/// Wrapper around the steamcmd binary.
///
/// Holds a lazily-initialised session that is reused across calls.
/// The session is spawned on the first command that needs it and
/// kept alive until the `SteamCmd` is dropped (or [`quit`](Self::quit)
/// is called explicitly).
///
/// # Thread safety
///
/// All session-backed methods require `&mut self`, so the borrow
/// checker prevents concurrent use at compile time.  The underlying
/// child process communicates over raw stdin/stdout pipes that are
/// **not** safe to share across threads.
pub struct SteamCmd {
    path: PathBuf,
    login: Login,
    platform: Option<Platform>,
    session: Option<Session>,
    auth_handler: Option<AuthHandler>,
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
            session: None,
            auth_handler: None,
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
                session: None,
                auth_handler: None,
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
                session: None,
                auth_handler: None,
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
            session: None,
            auth_handler: None,
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

    /// Set a callback for handling Steam Guard / TOTP prompts during
    /// login. The callback receives the prompt text and must return
    /// the auth code.
    #[must_use]
    pub fn with_auth_handler(mut self, handler: impl FnMut(&str) -> String + 'static) -> Self {
        self.auth_handler = Some(Box::new(handler));
        self
    }

    /// Returns the path to the steamcmd binary.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Return a reference to the live session, spawning one if needed.
    ///
    /// # Errors
    ///
    /// Returns an error if the process cannot be spawned or login fails.
    pub fn session(&mut self) -> Result<&mut Session, SteamCmdError> {
        if self.session.is_none() {
            let session = match self.auth_handler {
                Some(ref mut handler) => Session::start(
                    &self.path,
                    &self.login,
                    self.platform,
                    Some(handler.as_mut()),
                )?,
                None => Session::start(&self.path, &self.login, self.platform, None)?,
            };
            self.session = Some(session);
        }

        // We just ensured `self.session` is `Some` above.
        self.session
            .as_mut()
            .ok_or_else(|| SteamCmdError::Other("session unexpectedly missing".into()))
    }

    /// Shut down the running session (if any) and release the child
    /// process.  A new session will be spawned on the next command.
    ///
    /// # Errors
    ///
    /// Returns an error if the process cannot be waited on.
    pub fn quit(&mut self) -> Result<(), SteamCmdError> {
        if let Some(session) = self.session.take() {
            session.quit()?;
        }
        Ok(())
    }

    /// Get the 64-bit Steam ID of the currently logged-in user.
    ///
    /// Runs the `info` command and parses the `SteamID` field from the
    /// output (format `[U:1:XXXXX]`), converting it to a 64-bit ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the session cannot be started, the `info`
    /// command fails, or the Steam ID cannot be parsed from the output.
    pub fn steam_id(&mut self) -> Result<String, SteamCmdError> {
        let output = self.session()?.run_command("info")?;
        parse_steam_id(&output)
    }

    /// Download or update an app.
    ///
    /// # Errors
    ///
    /// Returns an error if steamcmd fails.
    pub fn download(&mut self, app_id: &str, install_dir: &Path) -> Result<(), SteamCmdError> {
        self.download_with_progress(app_id, install_dir, |_| {})
    }

    /// Download or update an app, calling `on_progress` for each
    /// parsed progress update as it arrives.
    ///
    /// # Errors
    ///
    /// Returns an error if steamcmd fails.
    pub fn download_with_progress(
        &mut self,
        app_id: &str,
        install_dir: &Path,
        mut on_progress: impl FnMut(&DownloadProgress),
    ) -> Result<(), SteamCmdError> {
        let session = self.session()?;
        session.run_command(&format!(
            "force_install_dir {}",
            install_dir.to_string_lossy()
        ))?;
        session.run_command_with_callback(&format!("app_update {app_id}"), |line| {
            if let Some(progress) = parse::parse_progress(line) {
                on_progress(&progress);
            }
        })?;
        Ok(())
    }

    /// Validate existing files for an installed app.
    ///
    /// # Errors
    ///
    /// Returns an error if steamcmd fails.
    pub fn validate(&mut self, app_id: &str, install_dir: &Path) -> Result<(), SteamCmdError> {
        self.validate_with_progress(app_id, install_dir, |_| {})
    }

    /// Validate existing files, calling `on_progress` for each
    /// parsed progress update as it arrives.
    ///
    /// # Errors
    ///
    /// Returns an error if steamcmd fails.
    pub fn validate_with_progress(
        &mut self,
        app_id: &str,
        install_dir: &Path,
        mut on_progress: impl FnMut(&DownloadProgress),
    ) -> Result<(), SteamCmdError> {
        let session = self.session()?;
        session.run_command(&format!(
            "force_install_dir {}",
            install_dir.to_string_lossy()
        ))?;
        session.run_command_with_callback(&format!("app_update {app_id} -validate"), |line| {
            if let Some(progress) = parse::parse_progress(line) {
                on_progress(&progress);
            }
        })?;
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
    pub fn app_info(&mut self, app_id: &str) -> Result<AppInfo, SteamCmdError> {
        let session = self.session()?;
        session.run_command("app_info_update 1")?;
        session.run_command(&format!("app_info_print {app_id}"))?;
        let output = session.run_command(&format!("app_info_print {app_id}"))?;
        Ok(parse::parse_app_info(app_id, &output))
    }

    /// Check the local install status of an app.
    ///
    /// # Errors
    ///
    /// Returns an error if steamcmd fails.
    pub fn app_status(&mut self, app_id: &str) -> Result<AppStatus, SteamCmdError> {
        let session = self.session()?;
        let output = session.run_command(&format!("app_status {app_id}"))?;
        Ok(parse::parse_app_status(app_id, &output))
    }
}

/// The base value added to a Steam3 account ID to get a 64-bit Steam ID.
const STEAM_ID_BASE: u64 = 76_561_197_960_265_728;

/// Parse a 64-bit Steam ID from steamcmd `info` output.
///
/// Looks for `SteamID:` followed by a Steam3 ID like `[U:1:12345678]`
/// and converts it to a 64-bit ID (`76561198012345678`).
fn parse_steam_id(output: &str) -> Result<String, SteamCmdError> {
    for line in output.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("SteamID:") {
            let rest = rest.trim();
            // Parse [U:1:XXXXX] format
            if let Some(inner) = rest.strip_prefix("[U:1:").and_then(|s| s.strip_suffix(']')) {
                let account_id: u64 = inner.parse().map_err(|_| {
                    SteamCmdError::Other(format!("failed to parse account ID: {inner}"))
                })?;
                return Ok((STEAM_ID_BASE + account_id).to_string());
            }
            return Err(SteamCmdError::Other(format!(
                "unexpected SteamID format: {rest}"
            )));
        }
    }
    Err(SteamCmdError::Other(
        "SteamID not found in info output".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_steam_id_from_info_output() {
        let output = "\
Account: coconutbird
SteamID: [U:1:52079950]
Email: test@example.com
Logon state: Logged On
";
        let id = parse_steam_id(output).unwrap();
        assert_eq!(id, "76561198012345678");
    }

    #[test]
    fn parse_steam_id_missing() {
        let output = "Account: coconutbird\nLogon state: Logged On\n";
        assert!(parse_steam_id(output).is_err());
    }
}
