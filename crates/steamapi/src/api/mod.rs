mod player_service;
mod steam_news;
mod steam_user;
mod steam_user_stats;

pub(crate) const API_BASE: &str = "https://api.steampowered.com";

/// Make a GET request and return the response body as a string.
pub(crate) fn get(url: &str) -> Result<String, crate::SteamError> {
    reqwest::blocking::get(url)
        .map_err(|e| crate::SteamError::Http(e.to_string()))?
        .text()
        .map_err(|e| crate::SteamError::Http(e.to_string()))
}
