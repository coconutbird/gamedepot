//! `ISteamUserStats` interface methods.

use std::fmt::Write;

use super::{API_BASE, get};
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
        let url = format!(
            "{API_BASE}/ISteamUserStats/GetGlobalAchievementPercentagesForApp/v0002/\
             ?gameid={app_id}&format=json",
        );
        let envelope: GlobalAchievementsEnvelope = get(&url)?;
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
        let key = self.require_key()?;
        let mut url = format!(
            "{API_BASE}/ISteamUserStats/GetPlayerAchievements/v0001/\
             ?appid={app_id}&key={key}&steamid={steam_id}&format=json",
        );
        if let Some(lang) = language {
            let _ = write!(url, "&l={lang}");
        }
        let envelope: PlayerAchievementsEnvelope = get(&url)?;
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
        let key = self.require_key()?;
        let url = format!(
            "{API_BASE}/ISteamUserStats/GetUserStatsForGame/v0002/\
             ?appid={app_id}&key={key}&steamid={steam_id}&format=json",
        );
        let envelope: UserStatsEnvelope = get(&url)?;
        Ok(envelope.playerstats.stats)
    }
}
