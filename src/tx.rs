use std::marker::PhantomData;
use rusqlite::Transaction;
use serde::{Deserialize, Serialize};

use crate::err::Error;

/// Represents a transaction on the key-value database.
pub struct Tx<'a, K, V> {
  tx: Transaction<'a>,
  _phantom: PhantomData<(K, V)>,
}

impl<'a, K, V> Tx<'a, K, V>
where
  K: Eq + Serialize + for<'de> Deserialize<'de>,
  V: Serialize + for<'de> Deserialize<'de>,
{
  /// Creates a new transaction.
  pub(crate) fn new(tx: Transaction<'a>) -> Tx<'a, K, V> {
    Tx { tx, _phantom: PhantomData }
  }

  /// Cancels the transaction and rolls back any changes.
  pub fn cancel(self) -> Result<(), Error> {
    self.rollback()
  }

  /// Rolls back the transaction.
  pub fn rollback(self) -> Result<(), Error> {
    self.tx.rollback().map_err(Error::SqliteError)?;
    Ok(())
  }

  /// Commits the transaction and stores all changes.
  pub fn commit(self) -> Result<(), Error> {
    self.tx.commit().map_err(Error::SqliteError)?;
    Ok(())
  }

  /// Checks if a key exists in the database.
  pub fn contains(&self, key: K) -> Result<bool, Error> {
    let key_bytes = postcard::to_stdvec(&key).map_err(Error::SerializationError)?;
    let mut stmt = self
      .tx
      .prepare("SELECT 1 FROM kv_store WHERE key = ?")
      .map_err(Error::SqliteError)?;
    let exists = stmt.exists(&[&key_bytes]).map_err(Error::SqliteError)?;
    Ok(exists)
  }

  /// Fetches a key from the database.
  pub fn get(&self, key: K) -> Result<Option<V>, Error> {
    let key_bytes = postcard::to_stdvec(&key).map_err(Error::SerializationError)?;
    let mut stmt = self
      .tx
      .prepare("SELECT value FROM kv_store WHERE key = ?")
      .map_err(Error::SqliteError)?;
    let mut rows = stmt.query(&[&key_bytes]).map_err(Error::SqliteError)?;
    if let Some(row) = rows.next().map_err(Error::SqliteError)? {
      let value_bytes: Vec<u8> = row.get(0).map_err(Error::SqliteError)?;
      let value = postcard::from_bytes(&value_bytes).map_err(Error::SerializationError)?;
      Ok(Some(value))
    } else {
      Ok(None)
    }
  }

  /// Inserts or updates a key in the database.
  pub fn set(&mut self, key: K, val: V) -> Result<(), Error> {
    let key_bytes = postcard::to_stdvec(&key).map_err(Error::SerializationError)?;
    let val_bytes = postcard::to_stdvec(&val).map_err(Error::SerializationError)?;
    self.tx
      .execute(
        "INSERT OR REPLACE INTO kv_store (key, value) VALUES (?, ?)",
        &[&key_bytes, &val_bytes],
      )
      .map_err(Error::SqliteError)?;
    Ok(())
  }

  /// Inserts a key if it doesn't exist in the database.
  pub fn put(&mut self, key: K, val: V) -> Result<(), Error> {
    let key_bytes = postcard::to_stdvec(&key).map_err(Error::SerializationError)?;
    let val_bytes = postcard::to_stdvec(&val).map_err(Error::SerializationError)?;
    let result = self.tx.execute(
      "INSERT INTO kv_store (key, value) VALUES (?, ?)",
      &[&key_bytes, &val_bytes],
    );
    match result {
      Ok(_) => Ok(()),
      Err(rusqlite::Error::SqliteFailure(e, _))
      if e.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_PRIMARYKEY =>
        {
          Err(Error::KeyAlreadyExists)
        }
      Err(e) => Err(Error::SqliteError(e)),
    }
  }

  /// Deletes a key from the database.
  pub fn del(&mut self, key: K) -> Result<(), Error> {
    let key_bytes = postcard::to_stdvec(&key).map_err(Error::SerializationError)?;
    self.tx
      .execute("DELETE FROM kv_store WHERE key = ?", &[&key_bytes])
      .map_err(Error::SqliteError)?;
    Ok(())
  }

  /// Retrieves all keys from the database.
  pub fn keys(&self) -> Result<Vec<K>, Error> {
    let mut stmt = self
      .tx
      .prepare("SELECT key FROM kv_store")
      .map_err(Error::SqliteError)?;
    let rows = stmt
      .query_map([], |row| {
        let key_bytes: Vec<u8> = row.get(0)?;
        let key = postcard::from_bytes(&key_bytes).map_err(|e| {
          rusqlite::Error::UserFunctionError(Box::new(Error::SerializationError(e)))
        })?;
        Ok(key)
      })
      .map_err(Error::SqliteError)?;

    let mut keys = Vec::new();
    for key_result in rows {
      keys.push(key_result.map_err(|e| match e {
        rusqlite::Error::UserFunctionError(err) => *err.downcast::<Error>().unwrap(),
        _ => Error::SqliteError(e),
      })?);
    }
    Ok(keys)
  }

  /// Retrieves all key-value pairs from the database.
  pub fn scan(&self) -> Result<Vec<(K, V)>, Error> {
    let mut stmt = self
      .tx
      .prepare("SELECT key, value FROM kv_store")
      .map_err(Error::SqliteError)?;
    let rows = stmt
      .query_map([], |row| {
        let key_bytes: Vec<u8> = row.get(0)?;
        let value_bytes: Vec<u8> = row.get(1)?;
        let key = postcard::from_bytes(&key_bytes).map_err(|e| {
          rusqlite::Error::UserFunctionError(Box::new(Error::SerializationError(e)))
        })?;
        let value = postcard::from_bytes(&value_bytes).map_err(|e| {
          rusqlite::Error::UserFunctionError(Box::new(Error::SerializationError(e)))
        })?;
        Ok((key, value))
      })
      .map_err(Error::SqliteError)?;

    let mut entries = Vec::new();
    for entry_result in rows {
      entries.push(entry_result.map_err(|e| match e {
        rusqlite::Error::UserFunctionError(err) => *err.downcast::<Error>().unwrap(),
        _ => Error::SqliteError(e),
      })?);
    }
    Ok(entries)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde::{Deserialize, Serialize};
  use tempfile::NamedTempFile;
  use crate::Db;

  #[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone)]
  struct TestKey {
    id: u32,
  }

  #[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone)]
  struct TestValue {
    data: String,
  }

  #[test]
  fn test_set_and_get() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();
    let mut db: Db<u32, String> = Db::new(db_path).unwrap();

    // Insert a key-value pair
    let mut tx = db.begin().unwrap();
    tx.set(1, "value1".to_string()).unwrap();
    tx.commit().unwrap();

    // Retrieve the inserted value
    let tx = db.begin().unwrap();
    let value = tx.get(1).unwrap();
    assert_eq!(value, Some("value1".to_string()));
  }

  #[test]
  fn test_put_existing_key() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();
    let mut db: Db<String, String> = Db::new(db_path).unwrap();

    // Insert a key-value pair using `put`
    let mut tx = db.begin().unwrap();
    tx.put("key1".to_string(), "value1".to_string()).unwrap();
    tx.commit().unwrap();

    // Attempt to insert the same key again using `put`, expect an error
    let mut tx = db.begin().unwrap();
    let result = tx.put("key1".to_string(), "value2".to_string());
    assert!(matches!(result, Err(Error::KeyAlreadyExists)));
    tx.rollback().unwrap();

    // Verify that the original value remains unchanged
    let tx = db.begin().unwrap();
    let value = tx.get("key1".to_string()).unwrap();
    assert_eq!(value, Some("value1".to_string()));
  }

  #[test]
  fn test_del() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();
    let mut db: Db<u32, String> = Db::new(db_path).unwrap();

    // Insert a key-value pair
    let mut tx = db.begin().unwrap();
    tx.set(1, "value1".to_string()).unwrap();
    tx.commit().unwrap();

    // Delete the key
    let mut tx = db.begin().unwrap();
    tx.del(1).unwrap();
    tx.commit().unwrap();

    // Verify deletion
    let tx = db.begin().unwrap();
    let value = tx.get(1).unwrap();
    assert_eq!(value, None);
  }

  #[test]
  fn test_contains() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();
    let mut db: Db<String, String> = Db::new(db_path).unwrap();

    // Insert a key-value pair
    let mut tx = db.begin().unwrap();
    tx.set("key1".to_string(), "value1".to_string()).unwrap();
    tx.commit().unwrap();

    // Check existence
    let tx = db.begin().unwrap();
    assert!(tx.contains("key1".to_string()).unwrap());
    assert!(!tx.contains("key2".to_string()).unwrap());
  }

  #[test]
  fn test_keys() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();
    let mut db: Db<String, String> = Db::new(db_path).unwrap();

    // Insert multiple key-value pairs
    let mut tx = db.begin().unwrap();
    tx.set("key1".to_string(), "value1".to_string()).unwrap();
    tx.set("key2".to_string(), "value2".to_string()).unwrap();
    tx.set("key3".to_string(), "value3".to_string()).unwrap();
    tx.commit().unwrap();

    // Retrieve all keys
    let tx = db.begin().unwrap();
    let keys = tx.keys().unwrap();
    assert_eq!(keys.len(), 3);
    assert!(keys.contains(&"key1".to_string()));
    assert!(keys.contains(&"key2".to_string()));
    assert!(keys.contains(&"key3".to_string()));
  }

  #[test]
  fn test_scan() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();
    let mut db: Db<u32, String> = Db::new(db_path).unwrap();

    // Insert multiple key-value pairs
    let mut tx = db.begin().unwrap();
    tx.set(1, "value1".to_string()).unwrap();
    tx.set(2, "value2".to_string()).unwrap();
    tx.set(3, "value3".to_string()).unwrap();
    tx.commit().unwrap();

    // Retrieve all key-value pairs
    let tx = db.begin().unwrap();
    let entries = tx.scan().unwrap();
    assert_eq!(entries.len(), 3);
    assert!(entries.contains(&(1, "value1".to_string())));
    assert!(entries.contains(&(2, "value2".to_string())));
    assert!(entries.contains(&(3, "value3".to_string())));
  }

  #[test]
  fn test_transaction_commit_and_rollback() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();
    let mut db: Db<String, String> = Db::new(db_path).unwrap();

    // Begin a transaction and insert a key, then commit
    let mut tx = db.begin().unwrap();
    tx.set("key1".to_string(), "value1".to_string()).unwrap();
    tx.commit().unwrap();

    // Verify insertion
    let tx = db.begin().unwrap();
    let value = tx.get("key1".to_string()).unwrap();
    assert_eq!(value, Some("value1".to_string()));
    drop(tx);

    // Begin another transaction, insert a key, then rollback
    let mut tx = db.begin().unwrap();
    tx.set("key2".to_string(), "value2".to_string()).unwrap();
    tx.rollback().unwrap();

    // Verify that "key2" was not inserted
    let tx = db.begin().unwrap();
    let value = tx.get("key2".to_string()).unwrap();
    assert_eq!(value, None);
  }
}
