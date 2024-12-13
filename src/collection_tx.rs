use rusqlite::Transaction;
use std::marker::PhantomData;
use serde::{Serialize, de::DeserializeOwned};
use crate::Error;

pub struct CollectionTx<'a, K, V> {
  tx: Transaction<'a>,
  collection: String,
  _phantom: PhantomData<(K, V)>,
}

impl<'a, K, V> CollectionTx<'a, K, V>
where
  K: Eq + Serialize + DeserializeOwned,
  V: Serialize + DeserializeOwned,
{
  pub(crate) fn new(tx: Transaction<'a>, name: String) -> Self {
    CollectionTx {
      tx,
      collection: name,
      _phantom: PhantomData,
    }
  }

  pub fn cancel(self) -> Result<(), Error> {
    self.rollback()
  }

  pub fn rollback(self) -> Result<(), Error> {
    self.tx.rollback()?;
    Ok(())
  }

  pub fn commit(self) -> Result<(), Error> {
    self.tx.commit()?;
    Ok(())
  }

  pub fn contains<Q: Into<K>>(&self, key: Q) -> Result<bool, Error> {
    let key = key.into();
    let key_bytes = postcard::to_stdvec(&key)?;
    let mut stmt = self.tx.prepare("SELECT 1 FROM kv_store WHERE collection = ? AND key = ?")?;
    let exists = stmt.exists(rusqlite::params![&self.collection, &key_bytes])?;
    Ok(exists)
  }

  pub fn get<Q: Into<K>>(&self, key: Q) -> Result<Option<V>, Error> {
    let key = key.into();
    let key_bytes = postcard::to_stdvec(&key)?;
    let mut stmt = self.tx.prepare("SELECT value FROM kv_store WHERE collection = ? AND key = ?")?;
    let mut rows = stmt.query(rusqlite::params![&self.collection, &key_bytes])?;
    if let Some(row) = rows.next()? {
      let value_bytes: Vec<u8> = row.get(0)?;
      let value = postcard::from_bytes(&value_bytes)?;
      Ok(Some(value))
    } else {
      Ok(None)
    }
  }

  pub fn set<Q: Into<K>, W: Into<V>>(&mut self, key: Q, val: W) -> Result<(), Error> {
    let key = key.into();
    let val = val.into();
    let key_bytes = postcard::to_stdvec(&key)?;
    let val_bytes = postcard::to_stdvec(&val)?;
    self.tx.execute(
      "INSERT OR REPLACE INTO kv_store (collection, key, value) VALUES (?, ?, ?)",
      rusqlite::params![&self.collection, &key_bytes, &val_bytes],
    )?;
    Ok(())
  }

  pub fn put<Q: Into<K>, W: Into<V>>(&mut self, key: Q, val: W) -> Result<(), Error> {
    let key = key.into();
    let val = val.into();
    let key_bytes = postcard::to_stdvec(&key)?;
    let val_bytes = postcard::to_stdvec(&val)?;
    let result = self.tx.execute(
      "INSERT INTO kv_store (collection, key, value) VALUES (?, ?, ?)",
      rusqlite::params![&self.collection, &key_bytes, &val_bytes],
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
    let key_bytes = postcard::to_stdvec(&key)?;
    self.tx.execute(
      "DELETE FROM kv_store WHERE collection = ? AND key = ?",
      rusqlite::params![&self.collection, &key_bytes],
    )?;
    Ok(())
  }

  pub fn keys(&self) -> Result<Vec<K>, Error> {
    let mut stmt = self.tx.prepare("SELECT key FROM kv_store WHERE collection = ?")?;
    let rows = stmt.query_map([&self.collection], |row| {
      let key_bytes: Vec<u8> = row.get(0)?;
      let key = postcard::from_bytes(&key_bytes)
        .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(Error::SerializationError(e))))?;
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
    let mut stmt = self.tx.prepare("SELECT key, value FROM kv_store WHERE collection = ?")?;
    let rows = stmt.query_map([&self.collection], |row| {
      let key_bytes: Vec<u8> = row.get(0)?;
      let value_bytes: Vec<u8> = row.get(1)?;

      let key = postcard::from_bytes(&key_bytes)
        .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(Error::SerializationError(e))))?;
      let value = postcard::from_bytes(&value_bytes)
        .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(Error::SerializationError(e))))?;
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
    self.tx.execute("DELETE FROM kv_store WHERE collection = ?", [&self.collection])?;
    Ok(())
  }

  pub fn count(&self) -> Result<usize, Error> {
    let mut stmt = self.tx.prepare("SELECT COUNT(*) FROM kv_store WHERE collection = ?")?;
    let cnt: i64 = stmt.query_row([&self.collection], |row| row.get(0))?;
    Ok(cnt as usize)
  }
}
