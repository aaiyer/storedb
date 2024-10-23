use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

use crate::err::Error;
use crate::Tx;

/// Represents the key-value database.
pub struct Db<K, V> {
  conn: Connection,
  _phantom: PhantomData<(K, V)>,
}

impl<K, V> Db<K, V>
where
  K: Eq + Serialize + for<'de> Deserialize<'de>,
  V: Serialize + for<'de> Deserialize<'de>,
{
  /// Creates a new database instance at the specified path.
  pub fn new(db_path: &str) -> Result<Self, Error> {
    let conn = Connection::open(db_path).map_err(Error::SqliteError)?;
    conn
      .execute_batch(r#"
        CREATE TABLE IF NOT EXISTS kv_store (key BLOB PRIMARY KEY, value BLOB NOT NULL);
        PRAGMA application_id = 1111199999;
        PRAGMA journal_mode = wal;
        PRAGMA synchronous = normal;
        PRAGMA temp_store = memory;
        PRAGMA auto_vacuum = incremental;
        PRAGMA mmap_size = 2147418112;
      "#)
      .map_err(Error::SqliteError)?;
    Ok(Db { conn, _phantom: PhantomData })
  }

  /// Starts a new transaction.
  ///
  /// - `write`: If `true`, the transaction is writable; otherwise, it's read-only.
  pub fn begin(&mut self) -> Result<Tx<K, V>, Error> {
    Ok(Tx::new(self.conn.transaction().map_err(Error::SqliteError)?))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde::{Deserialize, Serialize};
  use tempfile::NamedTempFile;

  #[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone)]
  struct TestKey {
    id: u32,
  }

  #[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone)]
  struct TestValue {
    data: String,
  }

  #[test]
  fn test_db_creation() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();
    let db: Result<Db<TestKey, TestValue>, Error> = Db::new(db_path);
    assert!(db.is_ok());
  }

  #[test]
  fn test_transaction_begin() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();
    let mut db: Db<String, String> = Db::new(db_path).unwrap();

    let tx = db.begin();
    assert!(tx.is_ok());
  }

  #[test]
  fn test_table_creation() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();
    let mut db: Db<String, String> = Db::new(db_path).unwrap();

    // Check if table exists by inserting a key-value pair
    let mut tx = db.begin().unwrap();
    tx.set("key1".to_string(), "value1".to_string()).unwrap();
    tx.commit().unwrap();

    // Start a new transaction to verify insertion
    let tx = db.begin().unwrap();
    let value = tx.get("key1".to_string()).unwrap();
    assert_eq!(value, Some("value1".to_string()));
  }
}
