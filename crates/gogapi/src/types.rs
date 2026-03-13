// Data types for GOG API responses.

use serde::Deserialize;

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
    pub id: u64,
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
    pub id: u64,
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
    pub id: u64,
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
#[allow(non_snake_case)]
pub struct WorksOn {
    #[serde(default)]
    pub Windows: bool,
    #[serde(default)]
    pub Mac: bool,
    #[serde(default)]
    pub Linux: bool,
}
