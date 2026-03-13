mod player_service;
mod steam_news;
mod steam_user;
mod steam_user_stats;

pub(crate) const API_BASE: &str = "https://api.steampowered.com";

/// Make a GET request and deserialize the JSON response.
pub(crate) fn get<T: serde::de::DeserializeOwned>(url: &str) -> Result<T, crate::SteamError> {
    reqwest::blocking::get(url)
        .map_err(|e| crate::SteamError::Http(e.to_string()))?
        .json()
        .map_err(|e| crate::SteamError::Parse(e.to_string()))
}
