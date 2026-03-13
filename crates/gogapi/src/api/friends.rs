//! Friends list endpoints (chat.gog.com).

use super::{CHAT_URL, get_json_authed};
use crate::error::GogError;
use crate::types;

impl super::Client {
    /// Fetch the friends list for a user.
    pub fn friends(&self, user_id: &str) -> Result<types::FriendsResponse, GogError> {
        let token = self.require_token()?;
        let url = format!("{CHAT_URL}/{user_id}/friends");
        get_json_authed(&url, token)
    }
}
