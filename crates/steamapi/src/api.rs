use crate::SteamApi;
use crate::error::SteamError;
use crate::types::{OwnedGamesEnvelope, PlayerSummariesEnvelope, PlayerSummary, VanityEnvelope};

const API_BASE: &str = "https://api.steampowered.com";

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
    ) -> Result<Vec<crate::types::OwnedGame>, SteamError> {
        let key = self.require_key()?;
        let url = format!(
            "{API_BASE}/IPlayerService/GetOwnedGames/v0001/\
             ?key={key}&steamid={steam_id}\
             &include_appinfo={}\
             &include_played_free_games=1\
             &format=json",
            u8::from(include_appinfo),
        );
        let body: String = ureq::get(&url)
            .call()
            .map_err(|e| SteamError::Http(e.to_string()))?
            .body_mut()
            .read_to_string()
            .map_err(|e| SteamError::Http(e.to_string()))?;
        let envelope: OwnedGamesEnvelope =
            serde_json::from_str(&body).map_err(|e| SteamError::Parse(e.to_string()))?;
        Ok(envelope.response.games)
    }

    /// Look up a player's profile summary by Steam ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the API key is missing, the request fails,
    /// or the response cannot be parsed.
    pub fn player_summary(&self, steam_id: &str) -> Result<Option<PlayerSummary>, SteamError> {
        let key = self.require_key()?;
        let url = format!(
            "{API_BASE}/ISteamUser/GetPlayerSummaries/v0002/\
             ?key={key}&steamids={steam_id}&format=json",
        );
        let body: String = ureq::get(&url)
            .call()
            .map_err(|e| SteamError::Http(e.to_string()))?
            .body_mut()
            .read_to_string()
            .map_err(|e| SteamError::Http(e.to_string()))?;
        let envelope: PlayerSummariesEnvelope =
            serde_json::from_str(&body).map_err(|e| SteamError::Parse(e.to_string()))?;
        Ok(envelope.response.players.into_iter().next())
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
        let body: String = ureq::get(&url)
            .call()
            .map_err(|e| SteamError::Http(e.to_string()))?
            .body_mut()
            .read_to_string()
            .map_err(|e| SteamError::Http(e.to_string()))?;
        let envelope: VanityEnvelope =
            serde_json::from_str(&body).map_err(|e| SteamError::Parse(e.to_string()))?;
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
