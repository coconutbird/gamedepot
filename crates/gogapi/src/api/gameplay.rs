//! Achievement endpoints (gameplay.gog.com).

use super::{GAMEPLAY_URL, get_json_authed};
use crate::error::GogError;
use crate::types;

impl super::Client {
    /// Fetch achievements for a product and user.
    pub fn achievements(
        &self,
        product_id: &str,
        user_id: &str,
    ) -> Result<types::AchievementsResponse, GogError> {
        let token = self.require_token()?;
        let url = format!("{GAMEPLAY_URL}/{product_id}/users/{user_id}/achievements");
        get_json_authed(&url, token)
    }
}
