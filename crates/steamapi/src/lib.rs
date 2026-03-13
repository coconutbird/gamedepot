//! Steam Web API client.
//!
//! Wraps the public Steam Web API endpoints:
//!
//! - **`ISteamUser`** — player summaries, friend lists, bans, vanity URLs
//! - **`IPlayerService`** — owned games, recently played, Steam level
//! - **`ISteamUserStats`** — achievements, stats, global percentages
//! - **`ISteamNews`** — app news
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
pub use types::{
    AchievementPercentage, Friend, NewsItem, OwnedGame, PlayerAchievement, PlayerBan, PlayerStat,
    PlayerSummary,
};

use api::API_BASE;

/// Steam Web API client.
#[derive(Debug, Clone)]
pub struct SteamApi {
    client: reqwest::blocking::Client,
    api_key: Option<String>,
}

impl SteamApi {
    /// Create a new client with the given API key.
    #[must_use]
    pub fn new(api_key: &str) -> Self {
        Self {
            client: reqwest::blocking::Client::new(),
            api_key: Some(api_key.to_owned()),
        }
    }

    /// Create a client without an API key.
    ///
    /// Most endpoints require a key, so this is mainly useful for
    /// building up the client before setting the key.
    #[must_use]
    pub fn without_key() -> Self {
        Self {
            client: reqwest::blocking::Client::new(),
            api_key: None,
        }
    }

    /// Set the API key.
    #[must_use]
    pub fn with_key(mut self, key: &str) -> Self {
        self.api_key = Some(key.to_owned());
        self
    }

    /// GET an endpoint that requires an API key.
    ///
    /// Automatically appends `key` and `format=json` query parameters.
    fn get<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        params: &[(&str, &str)],
    ) -> Result<T, SteamError> {
        let key = self
            .api_key
            .as_deref()
            .ok_or_else(|| SteamError::ApiKeyRequired("set STEAM_API_KEY".to_owned()))?;
        self.client
            .get(format!("{API_BASE}{path}"))
            .query(&[("key", key), ("format", "json")])
            .query(params)
            .send()
            .map_err(|e| SteamError::Http(e.to_string()))?
            .json()
            .map_err(|e| SteamError::Parse(e.to_string()))
    }

    /// GET an endpoint that does **not** require an API key.
    fn get_public<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        params: &[(&str, &str)],
    ) -> Result<T, SteamError> {
        self.client
            .get(format!("{API_BASE}{path}"))
            .query(&[("format", "json")])
            .query(params)
            .send()
            .map_err(|e| SteamError::Http(e.to_string()))?
            .json()
            .map_err(|e| SteamError::Parse(e.to_string()))
    }
}
