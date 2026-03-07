use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid UTF-8 in string: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("Color lookup index {index} out of bounds (table size {table_size})")]
    ColorLookupOutOfBounds { index: usize, table_size: usize },

    #[error("Rotation lookup index {index} out of bounds (table size {table_size})")]
    RotationLookupOutOfBounds { index: usize, table_size: usize },

    #[error("Unknown building version: {0}")]
    UnknownVersion(u8),
}

pub type Result<T> = std::result::Result<T, Error>;
