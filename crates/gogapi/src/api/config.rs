//! Galaxy client configuration (cfg.gog.com).

use super::{CFG_URL, get_json};
use crate::error::GogError;
use crate::types;

impl super::Client {
    /// Fetch the Galaxy client configuration.
    pub fn galaxy_config() -> Result<types::GalaxyConfig, GogError> {
        let url = format!("{CFG_URL}/desktop-galaxy-client/config.json");
        get_json(&url)
    }
}
