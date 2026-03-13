/// Errors that can occur when interacting with the Steam Web API.
#[derive(Debug, thiserror::Error)]
pub enum SteamError {
    /// HTTP request failed.
    #[error("http error: {0}")]
    Http(String),

    /// Failed to parse a JSON response.
    #[error("failed to parse response: {0}")]
    Parse(String),

    /// API key is required but not provided.
    #[error("API key required: {0}")]
    ApiKeyRequired(String),

    /// The requested resource was not found.
    #[error("not found: {0}")]
    NotFound(String),

    /// Any other error.
    #[error("{0}")]
    Other(String),
}
