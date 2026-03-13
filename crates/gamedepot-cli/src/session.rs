// Persistent session state stored at ~/.gamedepot/session.toml.

use std::fs;
use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Top-level session file.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Session {
    /// GOG session data.
    #[serde(default)]
    pub gog: GogSession,

    /// Steam session data.
    #[serde(default)]
    pub steam: SteamSession,
}

/// GOG-specific session state.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct GogSession {
    /// The `OAuth2` refresh token (rotates on every use).
    pub refresh_token: Option<String>,
}

/// Steam-specific session state.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SteamSession {
    /// Steam Web API key.
    pub api_key: Option<String>,
    /// `SteamCMD` username.
    pub username: Option<String>,
    /// `SteamCMD` password (stored in plaintext — steamcmd requires it).
    pub password: Option<String>,
    /// 64-bit Steam ID (resolved from vanity URL on login).
    pub steam_id: Option<String>,
}

impl Session {
    /// Return the path to `~/.gamedepot/session.toml`.
    ///
    /// # Errors
    ///
    /// Returns an error if the home directory cannot be determined.
    pub fn path() -> io::Result<PathBuf> {
        let home = home_dir()?;
        Ok(home.join(".gamedepot").join("session.toml"))
    }

    /// Load the session from disk, returning defaults if the file
    /// doesn't exist yet.
    ///
    /// # Errors
    ///
    /// Returns an error if the file exists but can't be read or parsed.
    pub fn load() -> io::Result<Self> {
        let path = Self::path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = fs::read_to_string(&path)?;
        toml::from_str(&contents).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Save the session to disk, creating `~/.gamedepot/` if needed.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory can't be created or the file
    /// can't be written.
    pub fn save(&self) -> io::Result<()> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let contents = toml::to_string_pretty(self).map_err(io::Error::other)?;
        fs::write(&path, contents)
    }
}

/// Get the user's home directory.
fn home_dir() -> io::Result<PathBuf> {
    #[cfg(unix)]
    {
        std::env::var("HOME")
            .map(PathBuf::from)
            .map_err(|_| io::Error::new(io::ErrorKind::NotFound, "HOME not set"))
    }
    #[cfg(windows)]
    {
        std::env::var("USERPROFILE")
            .map(PathBuf::from)
            .map_err(|_| io::Error::new(io::ErrorKind::NotFound, "USERPROFILE not set"))
    }
}
