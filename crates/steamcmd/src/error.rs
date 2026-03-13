use std::path::PathBuf;

/// Errors that can occur when interacting with `SteamCMD`.
#[derive(Debug, thiserror::Error)]
pub enum SteamCmdError {
    /// `SteamCMD` binary was not found on the system.
    #[error("steamcmd not found: {0}")]
    NotFound(String),

    /// `SteamCMD` binary path does not exist.
    #[error("invalid steamcmd path: {}", path.display())]
    InvalidPath { path: PathBuf },

    /// Failed to spawn the steamcmd process.
    #[error("failed to run steamcmd: {0}")]
    Io(#[from] std::io::Error),

    /// `SteamCMD` exited with a non-zero status code.
    #[error("steamcmd exited with status {code}: {stderr}")]
    NonZeroExit { code: i32, stderr: String },

    /// Auto-install of steamcmd failed.
    #[error("steamcmd install failed: {0}")]
    InstallFailed(String),
}
