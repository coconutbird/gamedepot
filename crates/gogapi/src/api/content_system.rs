//! Content-system and CDN endpoints (content-system.gog.com, cdn.gog.com).

use super::{BUILDS_URL, CDN_V2_META, get_json, get_json_authed, get_zlib_json};
use crate::error::GogError;
use crate::types;

impl super::Client {
    /// Fetch available builds for a product on a given OS.
    pub fn builds(&self, product_id: &str, os: &str) -> Result<types::BuildsResponse, GogError> {
        let url = format!("{BUILDS_URL}/{product_id}/os/{os}/builds?generation=2");
        get_json(&url)
    }

    /// Get authenticated CDN URLs for downloading V2 chunks.
    ///
    /// Returns URL templates with embedded tokens. Use
    /// [`SecureUrl::build_chunk_url`] to construct the final URL for
    /// each chunk.
    pub fn secure_link(&self, product_id: &str) -> Result<types::SecureLinkResponse, GogError> {
        let token = self.require_token()?;
        let url = format!("{BUILDS_URL}/{product_id}/secure_link?_version=2&generation=2&path=/");
        get_json_authed(&url, token)
    }

    /// Fetch a V2 repository manifest (zlib-compressed JSON).
    ///
    /// `url` is the `link` field from a [`BuildItem`].
    pub fn v2_repository(&self, url: &str) -> Result<types::V2Repository, GogError> {
        get_zlib_json(url)
    }

    /// Fetch a V2 depot manifest by its hash.
    ///
    /// The hash comes from [`V2DepotEntry::manifest`].
    pub fn v2_depot_manifest(&self, hash: &str) -> Result<types::V2DepotManifest, GogError> {
        let url = format!("{CDN_V2_META}/{}/{}/{hash}", &hash[..2], &hash[2..4]);
        get_zlib_json(&url)
    }

    /// Build the galaxy-path for a hash: `ab/cd/abcdef...`.
    #[must_use]
    pub fn galaxy_path(hash: &str) -> String {
        format!("{}/{}/{hash}", &hash[..2], &hash[2..4])
    }
}
