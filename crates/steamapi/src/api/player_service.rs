//! `IPlayerService` interface methods.

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
        let appinfo = u8::from(include_appinfo).to_string();
        let envelope: OwnedGamesEnvelope = self.get(
            "/IPlayerService/GetOwnedGames/v0001/",
            &[
                ("steamid", steam_id),
                ("include_appinfo", &appinfo),
                ("include_played_free_games", "1"),
            ],
        )?;
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
        let count_str = count.map(|c| c.to_string());
        let mut params: Vec<(&str, &str)> = vec![("steamid", steam_id)];
        if let Some(ref c) = count_str {
            params.push(("count", c));
        }
        let envelope: RecentlyPlayedEnvelope =
            self.get("/IPlayerService/GetRecentlyPlayedGames/v0001/", &params)?;
        Ok(envelope.response.games)
    }

    /// Get the Steam level of a user.
    ///
    /// # Errors
    ///
    /// Returns an error if the API key is missing, the request fails,
    /// or the response cannot be parsed.
    pub fn steam_level(&self, steam_id: &str) -> Result<u32, SteamError> {
        let envelope: SteamLevelEnvelope = self.get(
            "/IPlayerService/GetSteamLevel/v1/",
            &[("steamid", steam_id)],
        )?;
        Ok(envelope.response.player_level)
    }
}
