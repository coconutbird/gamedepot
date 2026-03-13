// HTTP client for the GOG REST API.

use crate::error::GogError;
use crate::types::{BuildsResponse, CatalogResponse, ProductResponse};

const CATALOG_URL: &str = "https://catalog.gog.com/v1/catalog";
const PRODUCT_URL: &str = "https://api.gog.com/products";
const BUILDS_URL: &str = "https://content-system.gog.com/products";

fn get_json<T: serde::de::DeserializeOwned>(url: &str) -> Result<T, GogError> {
    let body: String = ureq::get(url)
        .call()
        .map_err(|e| GogError::Http(e.to_string()))?
        .body_mut()
        .read_to_string()
        .map_err(|e| GogError::Parse(e.to_string()))?;
    serde_json::from_str(&body).map_err(|e| GogError::Parse(e.to_string()))
}

/// Search the GOG catalog for products matching a query.
///
/// # Errors
///
/// Returns an error if the HTTP request or JSON parsing fails.
pub fn search(query: &str) -> Result<CatalogResponse, GogError> {
    let url = format!(
        "{CATALOG_URL}?limit=20&order=desc:score\
         &productType=in:game,pack,dlc,extras\
         &countryCode=US&locale=en-US&currencyCode=USD\
         &query={query}"
    );
    get_json(&url)
}

/// Fetch detailed product information by product ID.
///
/// # Errors
///
/// Returns an error if the HTTP request or JSON parsing fails.
pub fn product_info(product_id: &str) -> Result<ProductResponse, GogError> {
    let url = format!(
        "{PRODUCT_URL}/{product_id}?locale=en_US\
         &expand=downloads,expanded_dlcs"
    );
    get_json(&url)
}

/// Fetch available builds for a product on a given OS.
///
/// # Errors
///
/// Returns an error if the HTTP request or JSON parsing fails.
pub fn builds(product_id: &str, os: &str) -> Result<BuildsResponse, GogError> {
    let url = format!("{BUILDS_URL}/{product_id}/os/{os}/builds?generation=2");
    get_json(&url)
}
