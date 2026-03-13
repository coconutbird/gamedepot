//! Steam Web API client.
//!
//! Provides access to Steam's Web API for querying owned games,
//! player summaries, and resolving vanity URLs.
//!
//! # Example
//!
//! ```no_run
//! use steamapi::SteamApi;
//!
//! let api = SteamApi::new("YOUR_API_KEY");
//!
//! // Resolve a vanity URL to a Steam ID
//! let steam_id = api.resolve_vanity_url("gabelogannewell").unwrap();
//!
//! // List owned games
//! let games = api.owned_games(&steam_id, true).unwrap();
//! for game in &games {
//!     println!("{}: {}", game.appid, game.name.as_deref().unwrap_or("?"));
//! }
//! ```

mod api;
pub mod error;
pub mod types;

pub use error::SteamError;

/// Steam Web API client.
#[derive(Debug, Clone)]
pub struct SteamApi {
    api_key: Option<String>,
}

impl SteamApi {
    /// Create a new client with the given API key.
    #[must_use]
    pub fn new(api_key: &str) -> Self {
        Self {
            api_key: Some(api_key.to_owned()),
        }
    }

    /// Create a client without an API key.
    ///
    /// Most endpoints require a key, so this is mainly useful for
    /// building up the client before setting the key.
    #[must_use]
    pub fn without_key() -> Self {
        Self { api_key: None }
    }

    /// Set the API key.
    #[must_use]
    pub fn with_key(mut self, key: &str) -> Self {
        self.api_key = Some(key.to_owned());
        self
    }

    /// Return the API key, or an error if none is set.
    fn require_key(&self) -> Result<&str, SteamError> {
        self.api_key
            .as_deref()
            .ok_or_else(|| SteamError::ApiKeyRequired("set STEAM_API_KEY".to_owned()))
    }
}
