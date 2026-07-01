use thiserror::Error;

#[derive(Debug, Error)]
pub enum FondamentError {
    #[error("address parse error: {0}")]
    AddressParse(String),
    #[error("definition not found: {0}")]
    NotFound(String),
    #[error("circular extends detected in: {0}")]
    CircularExtends(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("yaml error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("farga error: {0}")]
    Farga(String),
    /// Schema validation error — emitted before serde parsing when a definition
    /// file violates a structural invariant (e.g. flat-string `always_on` entries).
    /// Carries the file path and a human-readable description of the violation.
    #[error("schema error in {0}: {1}")]
    Schema(String, String),
}

pub type Result<T> = std::result::Result<T, FondamentError>;
