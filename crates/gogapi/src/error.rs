/// Errors that can occur when interacting with the GOG API.
#[derive(Debug, thiserror::Error)]
pub enum GogError {
    /// HTTP request failed.
    #[error("http error: {0}")]
    Http(String),

    /// Failed to parse a JSON response.
    #[error("failed to parse response: {0}")]
    Parse(String),

    /// I/O error (file writes, etc.).
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),

    /// Authentication required but no token provided.
    #[error("authentication required: {0}")]
    AuthRequired(String),

    /// Authentication token is invalid or expired.
    #[error("authentication failed: {0}")]
    AuthFailed(String),

    /// The requested product was not found.
    #[error("product not found: {0}")]
    NotFound(String),

    /// Any other error.
    #[error("{0}")]
    Other(String),
}
