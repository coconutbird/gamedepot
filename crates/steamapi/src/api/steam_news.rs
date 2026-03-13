//! `ISteamNews` interface methods.

use super::{API_BASE, get};
use crate::SteamApi;
use crate::error::SteamError;
use crate::types::{NewsEnvelope, NewsItem};

impl SteamApi {
    /// Get the latest news for an app.
    ///
    /// Does **not** require an API key.
    ///
    /// - `count`: number of news items to return.
    /// - `max_length`: maximum length of each news entry (0 for full text).
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or the response cannot be parsed.
    pub fn news_for_app(
        &self,
        app_id: u64,
        count: Option<u32>,
        max_length: Option<u32>,
    ) -> Result<Vec<NewsItem>, SteamError> {
        let count = count.unwrap_or(10);
        let max_length = max_length.unwrap_or(0);
        let url = format!(
            "{API_BASE}/ISteamNews/GetNewsForApp/v0002/\
             ?appid={app_id}&count={count}&maxlength={max_length}&format=json",
        );
        let envelope: NewsEnvelope = get(&url)?;
        Ok(envelope.appnews.newsitems)
    }
}
