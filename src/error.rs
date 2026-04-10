use thiserror::Error;

#[derive(Debug, Error)]
pub enum CdbError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid filter: {0}")]
    InvalidFilter(String),
}

pub type Result<T> = std::result::Result<T, CdbError>;
