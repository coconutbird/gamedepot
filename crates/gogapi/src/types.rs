// Data types for GOG API responses.

use serde::{Deserialize, Deserializer};

/// Deserialize a value that may be a JSON string or integer into a `String`.
fn string_or_number<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrNumber {
        Str(String),
        Num(u64),
    }
    match StringOrNumber::deserialize(deserializer)? {
        StringOrNumber::Str(s) => Ok(s),
        StringOrNumber::Num(n) => Ok(n.to_string()),
    }
}

// ── Catalog / Search ────────────────────────────────────────────────

/// Top-level response from the GOG catalog search API.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogResponse {
    pub products: Vec<CatalogProduct>,
    pub pages: u32,
    pub product_count: u32,
}

/// A single product from the catalog search results.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogProduct {
    #[serde(deserialize_with = "string_or_number")]
    pub id: String,
    pub slug: String,
    pub title: String,
    pub product_type: String,
    #[serde(default)]
    pub developers: Vec<String>,
    #[serde(default)]
    pub publishers: Vec<String>,
    #[serde(default)]
    pub operating_systems: Vec<String>,
    pub price: Option<CatalogPrice>,
    pub release_date: Option<String>,
}

/// Price information from the catalog.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogPrice {
    pub final_price: Option<String>,
    pub base_price: Option<String>,
    pub discount_percentage: Option<u32>,
}

// ── Product Info ────────────────────────────────────────────────────

/// Response from the product details API.
#[derive(Debug, Deserialize)]
pub struct ProductResponse {
    #[serde(deserialize_with = "string_or_number")]
    pub id: String,
    pub title: String,
    pub slug: String,
    #[serde(default)]
    pub content_system_compatibility: Option<ContentSystemCompat>,
    pub links: Option<ProductLinks>,
    pub downloads: Option<ProductDownloads>,
}

/// OS compatibility flags.
#[derive(Debug, Deserialize)]
pub struct ContentSystemCompat {
    #[serde(default)]
    pub windows: bool,
    #[serde(default)]
    pub osx: bool,
    #[serde(default)]
    pub linux: bool,
}

/// Links associated with a product.
#[derive(Debug, Deserialize)]
pub struct ProductLinks {
    pub store: Option<String>,
    pub forum: Option<String>,
}

/// Download information for a product (requires auth for actual URLs).
#[derive(Debug, Deserialize)]
pub struct ProductDownloads {
    #[serde(default)]
    pub installers: Vec<Installer>,
}

/// An installer file available for download.
#[derive(Debug, Deserialize)]
pub struct Installer {
    pub id: String,
    pub name: String,
    pub os: String,
    pub language: String,
    #[serde(default)]
    pub language_full: String,
    pub version: Option<String>,
    #[serde(default)]
    pub total_size: u64,
    #[serde(default)]
    pub files: Vec<InstallerFile>,
}

/// A single file within an installer.
#[derive(Debug, Deserialize)]
pub struct InstallerFile {
    pub id: String,
    pub size: u64,
    pub downlink: String,
}

// ── Builds ──────────────────────────────────────────────────────────

/// Response from the builds API.
#[derive(Debug, Deserialize)]
pub struct BuildsResponse {
    pub total_count: u32,
    #[serde(default)]
    pub items: Vec<BuildItem>,
}

/// A single build entry.
#[derive(Debug, Deserialize)]
pub struct BuildItem {
    pub build_id: String,
    pub product_id: String,
    pub os: String,
    pub branch: Option<String>,
    pub version_name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub public: bool,
    pub date_published: String,
    pub generation: u32,
    pub link: Option<String>,
}

// ── Content-System V2 ────────────────────────────────────────────────

/// V2 repository manifest (top-level meta for a build).
///
/// Fetched from the build's `link` URL, zlib-compressed.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2Repository {
    pub base_product_id: String,
    #[serde(default)]
    pub install_directory: String,
    #[serde(default)]
    pub depots: Vec<V2DepotEntry>,
    pub platform: Option<String>,
    #[serde(default)]
    pub products: Vec<V2Product>,
    #[serde(default)]
    pub dependencies: Vec<String>,
}

/// A depot reference inside a V2 repository manifest.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2DepotEntry {
    /// Language codes this depot covers (`"*"` = all, `"en"` = English, etc.).
    #[serde(default)]
    pub languages: Vec<String>,
    /// Hash used to fetch the depot manifest from the CDN.
    pub manifest: String,
    pub product_id: String,
    /// Total uncompressed size in bytes.
    #[serde(default)]
    pub size: u64,
}

/// Product metadata inside a V2 repository.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2Product {
    pub name: String,
    pub product_id: String,
}

/// A V2 depot manifest listing all files and their chunks.
#[derive(Debug, Deserialize)]
pub struct V2DepotManifest {
    pub depot: V2Depot,
}

/// The inner depot object containing the file list.
#[derive(Debug, Deserialize)]
pub struct V2Depot {
    #[serde(default)]
    pub items: Vec<V2DepotItem>,
}

/// A single file (or directory) in a V2 depot.
#[derive(Debug, Deserialize)]
pub struct V2DepotItem {
    /// Relative file path (uses backslashes on Windows depots).
    #[serde(default)]
    pub path: String,
    /// `"DepotFile"` or `"DepotDirectory"`.
    #[serde(rename = "type")]
    pub item_type: String,
    /// Chunks that make up this file (empty for directories).
    #[serde(default)]
    pub chunks: Vec<V2Chunk>,
    /// MD5 hash of the complete file.
    pub md5: Option<String>,
    /// Optional flags (e.g. `"hidden"`, `"support"`, `"executable"`).
    #[serde(default)]
    pub flags: Vec<String>,
}

/// A single chunk of a V2 depot file.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2Chunk {
    /// MD5 of the compressed chunk (used as the CDN path).
    pub compressed_md5: String,
    /// Size of the compressed chunk in bytes.
    pub compressed_size: u64,
    /// MD5 of the uncompressed data.
    pub md5: String,
    /// Uncompressed size in bytes.
    pub size: u64,
}

/// Response from the downlink endpoint (secure CDN URL).
#[derive(Debug, Deserialize)]
pub struct DownlinkResponse {
    pub downlink: String,
    #[serde(default)]
    pub checksum: Option<String>,
}

/// Response from the `secure_link` endpoint.
#[derive(Debug, Deserialize)]
pub struct SecureLinkResponse {
    pub urls: Vec<SecureUrl>,
}

/// A single secure URL entry with a template and parameters.
#[derive(Debug, Clone, Deserialize)]
pub struct SecureUrl {
    /// URL template with `{key}` placeholders.
    pub url_format: String,
    /// Key-value parameters to substitute into the template.
    pub parameters: std::collections::HashMap<String, serde_json::Value>,
}

impl SecureUrl {
    /// Build the final download URL for a V2 chunk.
    ///
    /// `galaxy_path` should be in the form `ab/cd/abcdef...`.
    #[must_use]
    pub fn build_chunk_url(&self, galaxy_path: &str) -> String {
        let mut url = self.url_format.clone();

        for (key, val) in &self.parameters {
            let placeholder = format!("{{{key}}}");
            if key == "path" {
                // Append the chunk sub-path to the base path.
                let base = val.as_str().unwrap_or("/");
                let full = if base.ends_with('/') {
                    format!("{base}{galaxy_path}")
                } else {
                    format!("{base}/{galaxy_path}")
                };
                url = url.replace(&placeholder, &full);
            } else {
                let s = match val {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    other => other.to_string(),
                };
                url = url.replace(&placeholder, &s);
            }
        }

        url
    }
}

// ── Owned Library ─────────────────────────────────────────────────────

/// Response from the owned-products endpoint
/// (`/account/getFilteredProducts`).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnedProductsResponse {
    pub page: u32,
    pub total_products: u32,
    pub total_pages: u32,
    pub products_per_page: u32,
    #[serde(default)]
    pub products: Vec<OwnedProduct>,
}

/// A product the user owns.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnedProduct {
    #[serde(deserialize_with = "string_or_number")]
    pub id: String,
    pub title: String,
    pub slug: String,
    #[serde(default)]
    pub category: Option<String>,
    pub rating: Option<u32>,
    #[serde(default)]
    pub is_game: bool,
    #[serde(default)]
    pub is_movie: bool,
    #[serde(default)]
    pub is_coming_soon: bool,
    #[serde(default)]
    pub works_on: Option<WorksOn>,
}

/// Platform support flags from the owned-products response.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct WorksOn {
    #[serde(default)]
    pub windows: bool,
    #[serde(default)]
    pub mac: bool,
    #[serde(default)]
    pub linux: bool,
}

// ── Users (users.gog.com) ──────────────────────────────────────────

/// User profile from `users.gog.com/users/{user_id}`.
#[derive(Debug, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub created_date: Option<String>,
    pub avatar: Option<UserAvatar>,
    #[serde(default)]
    pub is_employee: bool,
}

/// Avatar URLs for a user.
#[derive(Debug, Deserialize)]
pub struct UserAvatar {
    pub gog_image_id: Option<String>,
    pub small: Option<String>,
    pub small_2x: Option<String>,
    pub medium: Option<String>,
    pub medium_2x: Option<String>,
    pub large: Option<String>,
    pub large_2x: Option<String>,
}

// ── Friends (chat.gog.com) ─────────────────────────────────────────

/// Response from `chat.gog.com/users/{user_id}/friends`.
#[derive(Debug, Deserialize)]
pub struct FriendsResponse {
    #[serde(default)]
    pub items: Vec<Friend>,
}

/// A friend entry.
#[derive(Debug, Deserialize)]
pub struct Friend {
    pub user_id: String,
    pub username: String,
    #[serde(default)]
    pub is_employee: bool,
    pub images: Option<FriendImages>,
}

/// Avatar images for a friend.
#[derive(Debug, Deserialize)]
pub struct FriendImages {
    pub medium: Option<String>,
    pub medium_2x: Option<String>,
}

// ── Achievements (gameplay.gog.com) ────────────────────────────────

/// Response from the achievements endpoint.
#[derive(Debug, Deserialize)]
pub struct AchievementsResponse {
    pub total_count: u32,
    pub limit: u32,
    #[serde(default)]
    pub page_token: Option<String>,
    #[serde(default)]
    pub items: Vec<Achievement>,
}

/// A single achievement.
#[derive(Debug, Deserialize)]
pub struct Achievement {
    pub achievement_id: String,
    pub achievement_key: String,
    #[serde(default)]
    pub visible: bool,
    pub name: String,
    pub description: String,
    pub image_url_unlocked: Option<String>,
    pub image_url_locked: Option<String>,
    pub date_unlocked: Option<String>,
}

// ── Presence (presence.gog.com) ────────────────────────────────────

/// Response from `GET presence.gog.com/statuses`.
#[derive(Debug, Deserialize)]
pub struct StatusesResponse {
    pub total_count: u32,
    pub limit: u32,
    #[serde(default)]
    pub items: Vec<UserStatus>,
}

/// A single online-status entry.
#[derive(Debug, Deserialize)]
pub struct UserStatus {
    pub user_id: String,
    pub client_id: Option<String>,
    #[serde(default)]
    pub data: serde_json::Value,
}

// ── Galaxy Config (cfg.gog.com) ────────────────────────────────────

/// Response from `cfg.gog.com/desktop-galaxy-client/config.json`.
#[derive(Debug, Deserialize)]
pub struct GalaxyConfig {
    pub status: Option<String>,
    pub channel: Option<String>,
    #[serde(default)]
    pub end_points: Option<GalaxyEndpoints>,
}

/// CDN / service endpoints from the Galaxy config.
#[derive(Debug, Deserialize)]
pub struct GalaxyEndpoints {
    pub files: Option<String>,
    pub products: Option<String>,
    pub users: Option<String>,
    pub auth: Option<String>,
    pub cdn: Option<String>,
    #[serde(rename = "productsDetails")]
    pub products_details: Option<String>,
    pub gameplay: Option<String>,
    #[serde(rename = "gog-api")]
    pub gog_api: Option<String>,
}

// ── Extended Product Info ──────────────────────────────────────────

/// Extended product response with all optional expand fields.
#[derive(Debug, Deserialize)]
pub struct ProductResponseFull {
    #[serde(deserialize_with = "string_or_number")]
    pub id: String,
    pub title: String,
    pub slug: String,
    pub purchase_link: Option<String>,
    #[serde(default)]
    pub content_system_compatibility: Option<ContentSystemCompat>,
    pub languages: Option<serde_json::Value>,
    pub links: Option<ProductLinks>,
    pub in_development: Option<InDevelopment>,
    #[serde(default)]
    pub is_secret: bool,
    pub game_type: Option<String>,
    #[serde(default)]
    pub is_pre_order: bool,
    pub release_date: Option<String>,
    pub images: Option<ProductImages>,
    pub downloads: Option<ProductDownloads>,
    #[serde(default)]
    pub expanded_dlcs: Vec<serde_json::Value>,
    pub description: Option<ProductDescription>,
    #[serde(default)]
    pub screenshots: Vec<Screenshot>,
    #[serde(default)]
    pub videos: Vec<serde_json::Value>,
    #[serde(default)]
    pub related_products: Vec<serde_json::Value>,
    pub changelog: Option<String>,
}

/// In-development status.
#[derive(Debug, Deserialize)]
pub struct InDevelopment {
    #[serde(default)]
    pub active: bool,
    pub until: Option<String>,
}

/// Product image URLs.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductImages {
    pub background: Option<String>,
    pub logo: Option<String>,
    pub logo2x: Option<String>,
    pub icon: Option<String>,
    pub sidebar_icon: Option<String>,
    pub sidebar_icon2x: Option<String>,
}

/// Product description fields.
#[derive(Debug, Deserialize)]
pub struct ProductDescription {
    pub lead: Option<String>,
    pub full: Option<String>,
    pub whats_cool_about_it: Option<String>,
}

/// A screenshot entry.
#[derive(Debug, Deserialize)]
pub struct Screenshot {
    pub image_id: Option<String>,
    pub formatter_template_url: Option<String>,
    #[serde(default)]
    pub formatted_images: Vec<FormattedImage>,
}

/// A formatted screenshot image.
#[derive(Debug, Deserialize)]
pub struct FormattedImage {
    pub formatter_name: Option<String>,
    pub image_url: Option<String>,
}

// ── Resolved download types ────────────────────────────────────────

/// A file resolved and ready for download, with pre-built chunk URLs.
#[derive(Debug, Clone)]
pub struct ResolvedFile {
    /// Relative path (forward slashes, no leading slash).
    pub rel_path: String,
    /// File-level MD5 hash, if the manifest provides one.
    pub md5: Option<String>,
    /// Ordered list of chunks that make up this file.
    pub chunks: Vec<ResolvedChunk>,
}

impl ResolvedFile {
    /// Total compressed (download) size of all chunks.
    #[must_use]
    pub fn compressed_size(&self) -> u64 {
        self.chunks.iter().map(|c| c.compressed_size).sum()
    }

    /// Total uncompressed (on-disk) size of all chunks.
    #[must_use]
    pub fn uncompressed_size(&self) -> u64 {
        self.chunks.iter().map(|c| c.size).sum()
    }
}

/// A single chunk resolved with its download URL.
#[derive(Debug, Clone)]
pub struct ResolvedChunk {
    /// Full download URL (already has CDN auth tokens baked in).
    pub url: String,
    /// Compressed (download) size in bytes.
    pub compressed_size: u64,
    /// Uncompressed size in bytes.
    pub size: u64,
    /// MD5 of the uncompressed data.
    pub md5: String,
    /// MD5 of the compressed data (used as CDN path component).
    pub compressed_md5: String,
}
