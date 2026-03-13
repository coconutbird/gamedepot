//! Online presence endpoints (presence.gog.com).

use super::{PRESENCE_URL, get_json_authed};
use crate::error::GogError;
use crate::types;

impl super::Client {
    /// Set the user as online. Should be refreshed every 5 minutes.
    pub fn set_online(&self, user_id: &str) -> Result<(), GogError> {
        let token = self.require_token()?;
        let url = format!("{PRESENCE_URL}/users/{user_id}/status");
        reqwest::blocking::Client::new()
            .post(&url)
            .bearer_auth(token)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body("version=1.2.0.0")
            .send()
            .map_err(|e| GogError::Http(e.to_string()))?;
        Ok(())
    }

    /// Check which users from a list are currently online.
    pub fn statuses(&self, user_ids: &[&str]) -> Result<types::StatusesResponse, GogError> {
        let token = self.require_token()?;
        let ids = user_ids.join(",");
        let url = format!("{PRESENCE_URL}/statuses?user_id={ids}");
        get_json_authed(&url, token)
    }
}
