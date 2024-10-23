use thiserror::Error;

/// Errors which can be emitted from a database.
#[derive(Error, Debug)]
pub enum Error {
  #[error("SQLite error: {0}")]
  SqliteError(#[from] rusqlite::Error),

  #[error("Serialization error: {0}")]
  SerializationError(#[from] postcard::Error),

  #[error("Key being inserted already exists")]
  KeyAlreadyExists,
}
