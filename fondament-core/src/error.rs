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
}

pub type Result<T> = std::result::Result<T, FondamentError>;
