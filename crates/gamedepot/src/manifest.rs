// Manifest tracking for installed games.
//
// Stores install metadata at ~/.gamedepot/manifests.toml so we can
// look up installs by app ID or path, across both Steam and GOG.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Which storefront/depot an install came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DepotKind {
    Steam,
    Gog,
}

impl std::fmt::Display for DepotKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Steam => write!(f, "steam"),
            Self::Gog => write!(f, "gog"),
        }
    }
}

/// A single tracked game install.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Install {
    /// App/product identifier.
    pub app_id: String,
    /// Human-readable name.
    pub name: Option<String>,
    /// Build identifier at time of install/update.
    pub build_id: Option<String>,
    /// Which depot this came from.
    pub depot: DepotKind,
    /// Absolute path to the install directory.
    pub path: PathBuf,
    /// When the game was first installed (ISO 8601).
    pub installed_at: String,
    /// When the game was last updated/validated (ISO 8601).
    pub updated_at: String,
}

/// The top-level manifest file.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Manifest {
    /// All tracked installs.
    #[serde(default)]
    pub installs: Vec<Install>,
}

impl Manifest {
    /// Return the path to `~/.gamedepot/manifests.toml`.
    ///
    /// # Errors
    ///
    /// Returns an error if the home directory cannot be determined.
    pub fn path() -> io::Result<PathBuf> {
        let home = home_dir()?;
        Ok(home.join(".gamedepot").join("manifests.toml"))
    }

    /// Load the manifest from disk, returning an empty manifest if the
    /// file doesn't exist yet.
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

    /// Save the manifest to disk, creating `~/.gamedepot/` if needed.
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

    /// Add or update an install entry. If an entry with the same
    /// `app_id`, `depot`, and `path` already exists, it is updated
    /// in place. Otherwise a new entry is appended.
    pub fn upsert(&mut self, install: Install) {
        if let Some(existing) = self.installs.iter_mut().find(|i| {
            i.app_id == install.app_id && i.depot == install.depot && i.path == install.path
        }) {
            existing.name = install.name;
            existing.build_id = install.build_id;
            existing.updated_at = install.updated_at;
        } else {
            self.installs.push(install);
        }
    }

    /// Remove an install entry by app ID, depot, and path.
    pub fn remove(&mut self, app_id: &str, depot: &DepotKind, path: &Path) {
        self.installs
            .retain(|i| !(i.app_id == app_id && i.depot == *depot && i.path == path));
    }

    /// Find all installs for a given app ID.
    #[must_use]
    pub fn find_by_id(&self, app_id: &str) -> Vec<&Install> {
        self.installs
            .iter()
            .filter(|i| i.app_id == app_id)
            .collect()
    }

    /// Find an install by its directory path.
    #[must_use]
    pub fn find_by_path(&self, path: &Path) -> Option<&Install> {
        self.installs.iter().find(|i| i.path == path)
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
