//! Product and catalog endpoints (api.gog.com, embed.gog.com).

use super::{CATALOG_URL, EMBED_URL, PRODUCT_URL, get_json, get_json_authed};
use crate::error::GogError;
use crate::types;

impl super::Client {
    /// Search the GOG catalog for products matching a query.
    pub fn search(&self, query: &str) -> Result<types::CatalogResponse, GogError> {
        let url = format!(
            "{CATALOG_URL}?limit=20&order=desc:score\
             &productType=in:game,pack,dlc,extras\
             &countryCode={}&locale={}&currencyCode={}\
             &query={query}",
            self.country_code, self.locale, self.currency_code,
        );
        get_json(&url)
    }

    /// Fetch detailed product information by product ID.
    pub fn product_info(&self, product_id: &str) -> Result<types::ProductResponse, GogError> {
        let url = format!(
            "{PRODUCT_URL}/{product_id}?locale={}\
             &expand=downloads,expanded_dlcs",
            self.locale,
        );
        get_json(&url)
    }

    /// Fetch full product info with all expand fields.
    pub fn product_info_full(
        &self,
        product_id: &str,
    ) -> Result<types::ProductResponseFull, GogError> {
        let url = format!(
            "{PRODUCT_URL}/{product_id}?locale={}\
             &expand=downloads,expanded_dlcs,description,screenshots,videos,related_products,changelog",
            self.locale,
        );
        get_json(&url)
    }

    /// Fetch multiple products at once (up to 50).
    pub fn products_batch(&self, ids: &[&str]) -> Result<Vec<types::ProductResponse>, GogError> {
        let ids_str = ids.join(",");
        let url = format!(
            "{PRODUCT_URL}?ids={ids_str}&locale={}&expand=downloads,expanded_dlcs",
            self.locale,
        );
        get_json(&url)
    }

    /// Get a secure download URL for an installer file.
    ///
    /// `dl_path` is the path portion of a downlink URL, e.g.
    /// `"installer/en1installer3"`.
    pub fn downlink(
        &self,
        product_id: &str,
        dl_path: &str,
    ) -> Result<types::DownlinkResponse, GogError> {
        let token = self.require_token()?;
        let url = format!("{PRODUCT_URL}/{product_id}/downlink/{dl_path}");
        get_json_authed(&url, token)
    }

    /// List products owned by the authenticated user.
    pub fn owned_products(
        &self,
        search: Option<&str>,
        page: u32,
    ) -> Result<types::OwnedProductsResponse, GogError> {
        let token = self.require_token()?;
        let mut url = format!("{EMBED_URL}/account/getFilteredProducts?mediaType=1&page={page}");
        if let Some(q) = search {
            url.push_str("&search=");
            url.push_str(q);
        }
        get_json_authed(&url, token)
    }
}
