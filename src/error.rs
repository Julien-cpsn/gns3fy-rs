use thiserror::Error;

/// Errors returned by this crate.
#[derive(Debug, Error)]
pub enum Error {
    /// The object has no `connector` assigned.
    #[error("Gns3Connector not assigned under 'connector'")]
    MissingConnector,

    /// Invalid or missing argument / attribute (the Python library raised `ValueError`).
    #[error("{0}")]
    InvalidInput(String),

    /// A referenced object (project, node, port, snapshot...) could not be found.
    #[error("{0}")]
    NotFound(String),

    /// The GNS3 server answered with a non-success HTTP status.
    #[error("{status}: {message}")]
    Api { status: u16, message: String },

    /// Transport level error (connection, TLS, timeout...).
    #[error("HTTP transport error: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON (de)serialization error, including values outside the allowed enums.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Local I/O error (e.g. reading an image to upload).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The server URL could not be parsed.
    #[error("invalid URL: {0}")]
    Url(#[from] url::ParseError),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub(crate) fn invalid(msg: impl Into<String>) -> Self {
        Error::InvalidInput(msg.into())
    }
    pub(crate) fn not_found(msg: impl Into<String>) -> Self {
        Error::NotFound(msg.into())
    }
}
