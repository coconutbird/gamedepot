//! User profile endpoints (users.gog.com).

use super::{USERS_URL, get_json_authed};
use crate::error::GogError;
use crate::types;

impl super::Client {
    /// Fetch user profile information.
    pub fn user_info(&self, user_id: &str) -> Result<types::UserInfo, GogError> {
        let token = self.require_token()?;
        let url = format!("{USERS_URL}/{user_id}");
        get_json_authed(&url, token)
    }
}
