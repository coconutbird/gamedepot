use serde::Deserialize;

// ── GetOwnedGames ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub(crate) struct OwnedGamesEnvelope {
    pub response: OwnedGamesResponse,
}

#[derive(Debug, Deserialize)]
pub struct OwnedGamesResponse {
    #[serde(default)]
    pub game_count: u32,
    #[serde(default)]
    pub games: Vec<OwnedGame>,
}

/// A game the user owns on Steam.
#[derive(Debug, Deserialize)]
pub struct OwnedGame {
    pub appid: u64,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub playtime_forever: u32,
    #[serde(default)]
    pub playtime_2weeks: Option<u32>,
    #[serde(default)]
    pub img_icon_url: Option<String>,
    #[serde(default)]
    pub has_community_visible_stats: Option<bool>,
}

// ── GetPlayerSummaries ─────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub(crate) struct PlayerSummariesEnvelope {
    pub response: PlayerSummariesResponse,
}

#[derive(Debug, Deserialize)]
pub struct PlayerSummariesResponse {
    #[serde(default)]
    pub players: Vec<PlayerSummary>,
}

/// Summary info about a Steam user.
#[derive(Debug, Deserialize)]
pub struct PlayerSummary {
    pub steamid: String,
    #[serde(default)]
    pub personaname: Option<String>,
    #[serde(default)]
    pub profileurl: Option<String>,
    #[serde(default)]
    pub avatar: Option<String>,
}

// ── ResolveVanityURL ───────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub(crate) struct VanityEnvelope {
    pub response: VanityResponse,
}

#[derive(Debug, Deserialize)]
pub struct VanityResponse {
    /// 1 = success, 42 = no match.
    pub success: u32,
    #[serde(default)]
    pub steamid: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}
