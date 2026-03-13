use serde::Deserialize;

// ── IPlayerService / GetOwnedGames ─────────────────────────────────

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

/// A game the user owns (or recently played) on Steam.
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
    pub img_logo_url: Option<String>,
    #[serde(default)]
    pub has_community_visible_stats: Option<bool>,
}

// ── IPlayerService / GetRecentlyPlayedGames ─────────────────────────

#[derive(Debug, Deserialize)]
pub(crate) struct RecentlyPlayedEnvelope {
    pub response: RecentlyPlayedResponse,
}

#[derive(Debug, Deserialize)]
pub struct RecentlyPlayedResponse {
    #[serde(default)]
    pub total_count: u32,
    #[serde(default)]
    pub games: Vec<OwnedGame>,
}

// ── IPlayerService / GetSteamLevel ──────────────────────────────────

#[derive(Debug, Deserialize)]
pub(crate) struct SteamLevelEnvelope {
    pub response: SteamLevelResponse,
}

#[derive(Debug, Deserialize)]
pub struct SteamLevelResponse {
    #[serde(default)]
    pub player_level: u32,
}

// ── ISteamUser / GetPlayerSummaries ─────────────────────────────────

#[derive(Debug, Deserialize)]
pub(crate) struct PlayerSummariesEnvelope {
    pub response: PlayerSummariesResponse,
}

#[derive(Debug, Deserialize)]
pub struct PlayerSummariesResponse {
    #[serde(default)]
    pub players: Vec<PlayerSummary>,
}

/// Full profile summary for a Steam user.
#[derive(Debug, Deserialize)]
pub struct PlayerSummary {
    // ── public ──
    pub steamid: String,
    #[serde(default)]
    pub personaname: Option<String>,
    #[serde(default)]
    pub profileurl: Option<String>,
    #[serde(default)]
    pub avatar: Option<String>,
    #[serde(default)]
    pub avatarmedium: Option<String>,
    #[serde(default)]
    pub avatarfull: Option<String>,
    /// 0=Offline, 1=Online, 2=Busy, 3=Away, 4=Snooze, 5=Trade, 6=Play.
    #[serde(default)]
    pub personastate: Option<u8>,
    /// 1=not visible, 3=public.
    #[serde(default)]
    pub communityvisibilitystate: Option<u8>,
    #[serde(default)]
    pub profilestate: Option<u8>,
    #[serde(default)]
    pub lastlogoff: Option<u64>,
    #[serde(default)]
    pub commentpermission: Option<u8>,
    // ── private (only if profile is public) ──
    #[serde(default)]
    pub realname: Option<String>,
    #[serde(default)]
    pub primaryclanid: Option<String>,
    #[serde(default)]
    pub timecreated: Option<u64>,
    #[serde(default)]
    pub gameid: Option<String>,
    #[serde(default)]
    pub gameserverip: Option<String>,
    #[serde(default)]
    pub gameextrainfo: Option<String>,
    #[serde(default)]
    pub loccountrycode: Option<String>,
    #[serde(default)]
    pub locstatecode: Option<String>,
    #[serde(default)]
    pub loccityid: Option<u32>,
}

// ── ISteamUser / GetFriendList ──────────────────────────────────────

#[derive(Debug, Deserialize)]
pub(crate) struct FriendListEnvelope {
    pub friendslist: FriendListResponse,
}

#[derive(Debug, Deserialize)]
pub struct FriendListResponse {
    #[serde(default)]
    pub friends: Vec<Friend>,
}

/// A friend on a Steam user's friend list.
#[derive(Debug, Deserialize)]
pub struct Friend {
    pub steamid: String,
    #[serde(default)]
    pub relationship: Option<String>,
    #[serde(default)]
    pub friend_since: Option<u64>,
}

// ── ISteamUser / GetPlayerBans ──────────────────────────────────────

#[derive(Debug, Deserialize)]
pub(crate) struct PlayerBansEnvelope {
    pub players: Vec<PlayerBan>,
}

/// Ban information for a Steam user.
#[derive(Debug, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct PlayerBan {
    #[serde(rename = "SteamId")]
    pub steam_id: String,
    #[serde(rename = "CommunityBanned")]
    pub community_banned: bool,
    #[serde(rename = "VACBanned")]
    pub vac_banned: bool,
    #[serde(rename = "NumberOfVACBans")]
    pub number_of_vac_bans: u32,
    #[serde(rename = "DaysSinceLastBan")]
    pub days_since_last_ban: u32,
    #[serde(rename = "NumberOfGameBans")]
    pub number_of_game_bans: u32,
    #[serde(rename = "EconomyBan")]
    pub economy_ban: String,
}

// ── ISteamUser / ResolveVanityURL ───────────────────────────────────

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

// ── ISteamNews / GetNewsForApp ──────────────────────────────────────

#[derive(Debug, Deserialize)]
pub(crate) struct NewsEnvelope {
    pub appnews: AppNews,
}

#[derive(Debug, Deserialize)]
pub struct AppNews {
    #[serde(default)]
    pub appid: u64,
    #[serde(default)]
    pub newsitems: Vec<NewsItem>,
}

/// A news item for a Steam app.
#[derive(Debug, Deserialize)]
pub struct NewsItem {
    #[serde(default)]
    pub gid: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub contents: Option<String>,
    #[serde(default)]
    pub feedlabel: Option<String>,
    #[serde(default)]
    pub date: Option<u64>,
    #[serde(default)]
    pub feedname: Option<String>,
    #[serde(default)]
    pub appid: Option<u64>,
}

// ── ISteamUserStats / GetGlobalAchievementPercentagesForApp ─────────

#[derive(Debug, Deserialize)]
pub(crate) struct GlobalAchievementsEnvelope {
    pub achievementpercentages: GlobalAchievementsResponse,
}

#[derive(Debug, Deserialize)]
pub struct GlobalAchievementsResponse {
    #[serde(default)]
    pub achievements: Vec<AchievementPercentage>,
}

/// Global unlock percentage for a single achievement.
#[derive(Debug, Deserialize)]
pub struct AchievementPercentage {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub percent: f64,
}

// ── ISteamUserStats / GetPlayerAchievements ─────────────────────────

#[derive(Debug, Deserialize)]
pub(crate) struct PlayerAchievementsEnvelope {
    pub playerstats: PlayerAchievementsResponse,
}

#[derive(Debug, Deserialize)]
pub struct PlayerAchievementsResponse {
    #[serde(default)]
    pub achievements: Vec<PlayerAchievement>,
}

/// A single achievement for a player.
#[derive(Debug, Deserialize)]
pub struct PlayerAchievement {
    pub apiname: String,
    #[serde(default)]
    pub achieved: u8,
    #[serde(default)]
    pub unlocktime: Option<u64>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

// ── ISteamUserStats / GetUserStatsForGame ───────────────────────────

#[derive(Debug, Deserialize)]
pub(crate) struct UserStatsEnvelope {
    pub playerstats: UserStatsResponse,
}

#[derive(Debug, Deserialize)]
pub struct UserStatsResponse {
    #[serde(default)]
    pub stats: Vec<PlayerStat>,
}

/// A single stat value for a player in a game.
#[derive(Debug, Deserialize)]
pub struct PlayerStat {
    pub name: String,
    #[serde(default)]
    pub value: f64,
}
