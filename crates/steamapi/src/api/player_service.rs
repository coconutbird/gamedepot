//! `IPlayerService` interface methods.

use std::fmt::Write;

use super::{API_BASE, get};
use crate::SteamApi;
use crate::error::SteamError;
use crate::types::{OwnedGame, OwnedGamesEnvelope, RecentlyPlayedEnvelope, SteamLevelEnvelope};

impl SteamApi {
    /// Fetch the list of games owned by `steam_id`.
    ///
    /// Requires an API key. Set `include_appinfo` to `true` to get game
    /// names and icon URLs (otherwise only `appid` is populated).
    ///
    /// # Errors
    ///
    /// Returns an error if the API key is missing, the request fails,
    /// or the response cannot be parsed.
    pub fn owned_games(
        &self,
        steam_id: &str,
        include_appinfo: bool,
    ) -> Result<Vec<OwnedGame>, SteamError> {
        let key = self.require_key()?;
        let url = format!(
            "{API_BASE}/IPlayerService/GetOwnedGames/v0001/\
             ?key={key}&steamid={steam_id}\
             &include_appinfo={}\
             &include_played_free_games=1\
             &format=json",
            u8::from(include_appinfo),
        );
        let envelope: OwnedGamesEnvelope = get(&url)?;
        Ok(envelope.response.games)
    }

    /// Fetch the list of games recently played by `steam_id`.
    ///
    /// Returns games played in the last two weeks. Optionally limit
    /// the number of results with `count`.
    ///
    /// # Errors
    ///
    /// Returns an error if the API key is missing, the request fails,
    /// or the response cannot be parsed.
    pub fn recently_played_games(
        &self,
        steam_id: &str,
        count: Option<u32>,
    ) -> Result<Vec<OwnedGame>, SteamError> {
        let key = self.require_key()?;
        let mut url = format!(
            "{API_BASE}/IPlayerService/GetRecentlyPlayedGames/v0001/\
             ?key={key}&steamid={steam_id}&format=json",
        );
        if let Some(c) = count {
            let _ = write!(url, "&count={c}");
        }
        let envelope: RecentlyPlayedEnvelope = get(&url)?;
        Ok(envelope.response.games)
    }

    /// Get the Steam level of a user.
    ///
    /// # Errors
    ///
    /// Returns an error if the API key is missing, the request fails,
    /// or the response cannot be parsed.
    pub fn steam_level(&self, steam_id: &str) -> Result<u32, SteamError> {
        let key = self.require_key()?;
        let url = format!(
            "{API_BASE}/IPlayerService/GetSteamLevel/v1/\
             ?key={key}&steamid={steam_id}&format=json",
        );
        let envelope: SteamLevelEnvelope = get(&url)?;
        Ok(envelope.response.player_level)
    }
}
