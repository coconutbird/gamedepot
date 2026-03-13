//! `ISteamUserStats` interface methods.

use crate::SteamApi;
use crate::error::SteamError;
use crate::types::{
    AchievementPercentage, GlobalAchievementsEnvelope, PlayerAchievement,
    PlayerAchievementsEnvelope, PlayerStat, UserStatsEnvelope,
};

impl SteamApi {
    /// Get global achievement unlock percentages for an app.
    ///
    /// Does **not** require an API key.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or the response cannot be parsed.
    pub fn global_achievement_percentages(
        &self,
        app_id: u64,
    ) -> Result<Vec<AchievementPercentage>, SteamError> {
        let app_id_str = app_id.to_string();
        let envelope: GlobalAchievementsEnvelope = self.get_public(
            "/ISteamUserStats/GetGlobalAchievementPercentagesForApp/v0002/",
            &[("gameid", &app_id_str)],
        )?;
        Ok(envelope.achievementpercentages.achievements)
    }

    /// Get a player's achievements for a specific game.
    ///
    /// # Errors
    ///
    /// Returns an error if the API key is missing, the request fails,
    /// or the response cannot be parsed.
    pub fn player_achievements(
        &self,
        steam_id: &str,
        app_id: u64,
        language: Option<&str>,
    ) -> Result<Vec<PlayerAchievement>, SteamError> {
        let app_id_str = app_id.to_string();
        let mut params: Vec<(&str, &str)> = vec![("appid", &app_id_str), ("steamid", steam_id)];
        if let Some(lang) = language {
            params.push(("l", lang));
        }
        let envelope: PlayerAchievementsEnvelope =
            self.get("/ISteamUserStats/GetPlayerAchievements/v0001/", &params)?;
        Ok(envelope.playerstats.achievements)
    }

    /// Get a player's stats for a specific game.
    ///
    /// # Errors
    ///
    /// Returns an error if the API key is missing, the request fails,
    /// or the response cannot be parsed.
    pub fn user_stats_for_game(
        &self,
        steam_id: &str,
        app_id: u64,
    ) -> Result<Vec<PlayerStat>, SteamError> {
        let app_id_str = app_id.to_string();
        let envelope: UserStatsEnvelope = self.get(
            "/ISteamUserStats/GetUserStatsForGame/v0002/",
            &[("appid", &app_id_str), ("steamid", steam_id)],
        )?;
        Ok(envelope.playerstats.stats)
    }
}
