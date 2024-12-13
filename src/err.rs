use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
  #[error("SQLite error: {0}")]
  SqliteError(#[from] rusqlite::Error),

  #[error("Serialization error: {0}")]
  SerializationError(#[from] postcard::Error),

  #[error("Key being inserted already exists")]
  KeyAlreadyExists,

  #[error("Collection type mismatch: expected key={expected_key}, value={expected_value}, got key={got_key}, value={got_value}")]
  TypeMismatch {
    expected_key: String,
    expected_value: String,
    got_key: String,
    got_value: String,
  },
}
