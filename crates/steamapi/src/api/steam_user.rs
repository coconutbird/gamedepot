//! `ISteamUser` interface methods.

use super::{API_BASE, get};
use crate::SteamApi;
use crate::error::SteamError;
use crate::types::{
    Friend, FriendListEnvelope, PlayerBan, PlayerBansEnvelope, PlayerSummariesEnvelope,
    PlayerSummary, VanityEnvelope,
};

impl SteamApi {
    /// Look up one or more players' profile summaries by Steam ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the API key is missing, the request fails,
    /// or the response cannot be parsed.
    pub fn player_summaries(&self, steam_ids: &[&str]) -> Result<Vec<PlayerSummary>, SteamError> {
        let key = self.require_key()?;
        let ids = steam_ids.join(",");
        let url = format!(
            "{API_BASE}/ISteamUser/GetPlayerSummaries/v0002/\
             ?key={key}&steamids={ids}&format=json",
        );
        let envelope: PlayerSummariesEnvelope = get(&url)?;
        Ok(envelope.response.players)
    }

    /// Look up a single player's profile summary by Steam ID.
    ///
    /// Convenience wrapper around [`player_summaries`](Self::player_summaries).
    ///
    /// # Errors
    ///
    /// Returns an error if the API key is missing, the request fails,
    /// or the response cannot be parsed.
    pub fn player_summary(&self, steam_id: &str) -> Result<Option<PlayerSummary>, SteamError> {
        Ok(self.player_summaries(&[steam_id])?.into_iter().next())
    }

    /// Return the friend list of a Steam user.
    ///
    /// The profile must be public for this to work.
    /// `relationship` can be `"friend"` or `"all"`.
    ///
    /// # Errors
    ///
    /// Returns an error if the API key is missing, the request fails,
    /// or the response cannot be parsed.
    pub fn friend_list(
        &self,
        steam_id: &str,
        relationship: &str,
    ) -> Result<Vec<Friend>, SteamError> {
        let key = self.require_key()?;
        let url = format!(
            "{API_BASE}/ISteamUser/GetFriendList/v0001/\
             ?key={key}&steamid={steam_id}&relationship={relationship}&format=json",
        );
        let envelope: FriendListEnvelope = get(&url)?;
        Ok(envelope.friendslist.friends)
    }

    /// Get ban information for one or more Steam IDs.
    ///
    /// # Errors
    ///
    /// Returns an error if the API key is missing, the request fails,
    /// or the response cannot be parsed.
    pub fn player_bans(&self, steam_ids: &[&str]) -> Result<Vec<PlayerBan>, SteamError> {
        let key = self.require_key()?;
        let ids = steam_ids.join(",");
        let url = format!(
            "{API_BASE}/ISteamUser/GetPlayerBans/v1/\
             ?key={key}&steamids={ids}&format=json",
        );
        let envelope: PlayerBansEnvelope = get(&url)?;
        Ok(envelope.players)
    }

    /// Resolve a Steam vanity URL name (custom profile URL) to a Steam ID.
    ///
    /// For example, `"gabelogannewell"` → `"76561197960287930"`.
    ///
    /// # Errors
    ///
    /// Returns an error if the API key is missing, the request fails,
    /// or the vanity name doesn't match any user.
    pub fn resolve_vanity_url(&self, vanity_name: &str) -> Result<String, SteamError> {
        let key = self.require_key()?;
        let url = format!(
            "{API_BASE}/ISteamUser/ResolveVanityURL/v0001/\
             ?key={key}&vanityurl={vanity_name}&format=json",
        );
        let envelope: VanityEnvelope = get(&url)?;
        if envelope.response.success == 1 {
            envelope
                .response
                .steamid
                .ok_or_else(|| SteamError::Parse("success=1 but no steamid".to_owned()))
        } else {
            let msg = envelope
                .response
                .message
                .unwrap_or_else(|| "no match".to_owned());
            Err(SteamError::NotFound(msg))
        }
    }
}
