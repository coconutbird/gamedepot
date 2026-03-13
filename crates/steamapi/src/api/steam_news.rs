//! `ISteamNews` interface methods.

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
        let app_id_str = app_id.to_string();
        let count_str = count.unwrap_or(10).to_string();
        let max_len_str = max_length.unwrap_or(0).to_string();
        let envelope: NewsEnvelope = self.get_public(
            "/ISteamNews/GetNewsForApp/v0002/",
            &[
                ("appid", &app_id_str),
                ("count", &count_str),
                ("maxlength", &max_len_str),
            ],
        )?;
        Ok(envelope.appnews.newsitems)
    }
}
