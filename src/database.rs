use rusqlite::Connection;
use std::path::Path;
use crate::Error;
use crate::collection::Collection;
use std::any::type_name;
use std::sync::Arc;

pub struct Database {
  conn: Arc<Connection>,
}

impl Database {
  pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self, Error> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS collection_meta (
                name TEXT PRIMARY KEY,
                key_type TEXT NOT NULL,
                value_type TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS kv_store (
                collection TEXT,
                key BLOB,
                value BLOB NOT NULL,
                PRIMARY KEY(collection, key)
            );
        "#)?;
    Ok(Database { conn: Arc::new(conn) })
  }

  pub fn get_collection<K, V>(&mut self, name: &str) -> Result<Collection<K, V>, Error>
  where
    K: Eq + serde::Serialize + serde::de::DeserializeOwned,
    V: serde::Serialize + serde::de::DeserializeOwned,
  {
    let expected_key_type = type_name::<K>().to_string();
    let expected_value_type = type_name::<V>().to_string();

    let mut stmt = self.conn.prepare("SELECT key_type, value_type FROM collection_meta WHERE name = ?")?;
    let mut rows = stmt.query([name])?;

    if let Some(row) = rows.next()? {
      let db_key_type: String = row.get(0)?;
      let db_value_type: String = row.get(1)?;
      if db_key_type != expected_key_type || db_value_type != expected_value_type {
        return Err(Error::TypeMismatch {
          expected_key: expected_key_type,
          expected_value: expected_value_type,
          got_key: db_key_type,
          got_value: db_value_type,
        });
      }
    } else {
      self.conn.execute(
        "INSERT INTO collection_meta (name, key_type, value_type) VALUES (?, ?, ?)",
        [name, &expected_key_type, &expected_value_type],
      )?;
    }

    Ok(Collection::new(self.conn.clone(), name.to_string()))
  }
}
