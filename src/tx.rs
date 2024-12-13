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
  pub(crate) fn new(tx: Transaction<'a>) -> Tx<'a, K, V> {
    Tx { tx, _phantom: PhantomData }
  }

  pub fn cancel(self) -> Result<(), Error> {
    self.rollback()
  }

  pub fn rollback(self) -> Result<(), Error> {
    self.tx.rollback().map_err(Error::SqliteError)?;
    Ok(())
  }

  pub fn commit(self) -> Result<(), Error> {
    self.tx.commit().map_err(Error::SqliteError)?;
    Ok(())
  }

  pub fn contains<Q: Into<K>>(&self, key: Q) -> Result<bool, Error> {
    let key = key.into();
    let key_bytes = postcard::to_stdvec(&key).map_err(Error::SerializationError)?;
    let mut stmt = self.tx.prepare("SELECT 1 FROM kv_store WHERE key = ?")?;
    let exists = stmt.exists([&key_bytes])?;
    Ok(exists)
  }

  pub fn get<Q: Into<K>>(&self, key: Q) -> Result<Option<V>, Error> {
    let key = key.into();
    let key_bytes = postcard::to_stdvec(&key).map_err(Error::SerializationError)?;
    let mut stmt = self.tx.prepare("SELECT value FROM kv_store WHERE key = ?")?;
    let mut rows = stmt.query([&key_bytes])?;
    if let Some(row) = rows.next()? {
      let value_bytes: Vec<u8> = row.get(0)?;
      let value = postcard::from_bytes(&value_bytes).map_err(Error::SerializationError)?;
      Ok(Some(value))
    } else {
      Ok(None)
    }
  }

  pub fn set<Q: Into<K>, W: Into<V>>(&mut self, key: Q, val: W) -> Result<(), Error> {
    let key = key.into();
    let val = val.into();
    let key_bytes = postcard::to_stdvec(&key).map_err(Error::SerializationError)?;
    let val_bytes = postcard::to_stdvec(&val).map_err(Error::SerializationError)?;
    self.tx.execute(
      "INSERT OR REPLACE INTO kv_store (key, value) VALUES (?, ?)",
      [&key_bytes, &val_bytes],
    )?;
    Ok(())
  }

  pub fn put<Q: Into<K>, W: Into<V>>(&mut self, key: Q, val: W) -> Result<(), Error> {
    let key = key.into();
    let val = val.into();
    let key_bytes = postcard::to_stdvec(&key).map_err(Error::SerializationError)?;
    let val_bytes = postcard::to_stdvec(&val).map_err(Error::SerializationError)?;
    let result = self.tx.execute(
      "INSERT INTO kv_store (key, value) VALUES (?, ?)",
      [&key_bytes, &val_bytes],
    );
    match result {
      Ok(_) => Ok(()),
      Err(rusqlite::Error::SqliteFailure(e, _)) if e.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_PRIMARYKEY => {
        Err(Error::KeyAlreadyExists)
      }
      Err(e) => Err(Error::SqliteError(e)),
    }
  }

  pub fn del<Q: Into<K>>(&mut self, key: Q) -> Result<(), Error> {
    let key = key.into();
    let key_bytes = postcard::to_stdvec(&key).map_err(Error::SerializationError)?;
    self.tx.execute("DELETE FROM kv_store WHERE key = ?", [&key_bytes])?;
    Ok(())
  }

  pub fn keys(&self) -> Result<Vec<K>, Error> {
    let mut stmt = self.tx.prepare("SELECT key FROM kv_store")?;
    let rows = stmt.query_map([], |row| {
      let key_bytes: Vec<u8> = row.get(0)?;
      let key = postcard::from_bytes(&key_bytes).map_err(|e| rusqlite::Error::UserFunctionError(Box::new(Error::SerializationError(e))))?;
      Ok(key)
    })?;

    let mut keys = Vec::new();
    for key_result in rows {
      let key = key_result.map_err(|e| match e {
        rusqlite::Error::UserFunctionError(err) => *err.downcast::<Error>().unwrap(),
        _ => Error::SqliteError(e),
      })?;
      keys.push(key);
    }
    Ok(keys)
  }

  pub fn scan(&self) -> Result<Vec<(K, V)>, Error> {
    let mut stmt = self.tx.prepare("SELECT key, value FROM kv_store")?;
    let rows = stmt.query_map([], |row| {
      let key_bytes: Vec<u8> = row.get(0)?;
      let value_bytes: Vec<u8> = row.get(1)?;

      let key = postcard::from_bytes(&key_bytes).map_err(|e| rusqlite::Error::UserFunctionError(Box::new(Error::SerializationError(e))))?;
      let value = postcard::from_bytes(&value_bytes).map_err(|e| rusqlite::Error::UserFunctionError(Box::new(Error::SerializationError(e))))?;
      Ok((key, value))
    })?;

    let mut entries = Vec::new();
    for entry_result in rows {
      entries.push(entry_result.map_err(|e| match e {
        rusqlite::Error::UserFunctionError(err) => *err.downcast::<Error>().unwrap(),
        _ => Error::SqliteError(e),
      })?);
    }
    Ok(entries)
  }

  pub fn clear(&mut self) -> Result<(), Error> {
    self.tx.execute("DELETE FROM kv_store", [])?;
    Ok(())
  }

  pub fn count(&self) -> Result<usize, Error> {
    let mut stmt = self.tx.prepare("SELECT COUNT(*) FROM kv_store")?;
    let cnt: i64 = stmt.query_row([], |row| row.get(0))?;
    Ok(cnt as usize)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde::{Deserialize, Serialize};
  use tempfile::NamedTempFile;
  use crate::Db;

  #[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone)]
  struct TestVal {
    data: String,
  }

  #[test]
  fn test_set_and_get() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();
    let mut db: Db<u32, String> = Db::new(db_path).unwrap();

    let mut tx = db.begin().unwrap();
    tx.set(1u32, "value1").unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let value = tx.get(1u32).unwrap();
    assert_eq!(value, Some("value1".to_string()));
  }

  #[test]
  fn test_put_existing_key() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();
    let mut db: Db<String, String> = Db::new(db_path).unwrap();

    let mut tx = db.begin().unwrap();
    tx.put("key1", "value1").unwrap();
    tx.commit().unwrap();

    let mut tx = db.begin().unwrap();
    let result = tx.put("key1", "value2");
    assert!(matches!(result, Err(Error::KeyAlreadyExists)));
    tx.rollback().unwrap();

    let tx = db.begin().unwrap();
    let value = tx.get("key1").unwrap();
    assert_eq!(value, Some("value1".to_string()));
  }

  #[test]
  fn test_del() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();
    let mut db: Db<u32, String> = Db::new(db_path).unwrap();

    let mut tx = db.begin().unwrap();
    tx.set(1u32, "value1").unwrap();
    tx.commit().unwrap();

    let mut tx = db.begin().unwrap();
    tx.del(1u32).unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let value = tx.get(1u32).unwrap();
    assert_eq!(value, None);
  }

  #[test]
  fn test_contains() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();
    let mut db: Db<String, String> = Db::new(db_path).unwrap();

    let mut tx = db.begin().unwrap();
    tx.set("key1", "value1").unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    assert!(tx.contains("key1").unwrap());
    assert!(!tx.contains("key2").unwrap());
  }

  #[test]
  fn test_keys() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();
    let mut db: Db<String, String> = Db::new(db_path).unwrap();

    let mut tx = db.begin().unwrap();
    tx.set("key1", "value1").unwrap();
    tx.set("key2", "value2").unwrap();
    tx.set("key3", "value3").unwrap();
    tx.commit().unwrap();

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

    let mut tx = db.begin().unwrap();
    tx.set(1u32, "value1").unwrap();
    tx.set(2u32, "value2").unwrap();
    tx.set(3u32, "value3").unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let entries = tx.scan().unwrap();
    assert_eq!(entries.len(), 3);
    assert!(entries.contains(&(1u32, "value1".to_string())));
    assert!(entries.contains(&(2u32, "value2".to_string())));
    assert!(entries.contains(&(3u32, "value3".to_string())));
  }

  #[test]
  fn test_clear_and_count() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();
    let mut db: Db<u32, String> = Db::new(db_path).unwrap();

    let mut tx = db.begin().unwrap();
    tx.set(1u32, "a").unwrap();
    tx.set(2u32, "b").unwrap();
    tx.set(3u32, "c").unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    assert_eq!(tx.count().unwrap(), 3);
    drop(tx);

    let mut tx = db.begin().unwrap();
    tx.clear().unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    assert_eq!(tx.count().unwrap(), 0);
  }

  #[test]
  fn test_transaction_commit_and_rollback() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();
    let mut db: Db<String, String> = Db::new(db_path).unwrap();

    // Begin a transaction and insert a key, then commit
    let mut tx = db.begin().unwrap();
    tx.set("key1", "value1").unwrap();
    tx.commit().unwrap();

    // Verify insertion
    let tx = db.begin().unwrap();
    let value = tx.get("key1").unwrap();
    assert_eq!(value, Some("value1".to_string()));
    drop(tx);

    // Begin another transaction, insert a key, then rollback
    let mut tx = db.begin().unwrap();
    tx.set("key2", "value2").unwrap();
    tx.rollback().unwrap();

    // Verify that "key2" was not inserted
    let tx = db.begin().unwrap();
    let value = tx.get("key2").unwrap();
    assert_eq!(value, None);
  }
}
