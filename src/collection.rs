use crate::Error;
use crate::collection_tx::CollectionTx;
use rusqlite::Connection;
use std::marker::PhantomData;
use std::sync::Arc;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt;

pub struct Collection<K, V> {
  pub(crate) conn: Arc<Connection>,
  pub(crate) name: String,
  _phantom: PhantomData<(K, V)>,
}

impl<K, V> Collection<K, V>
where
  K: Eq + Serialize + DeserializeOwned,
  V: Serialize + DeserializeOwned,
{
  pub(crate) fn new(conn: Arc<Connection>, name: String) -> Self {
    Collection {
      conn,
      name,
      _phantom: PhantomData,
    }
  }

  pub fn begin(&mut self) -> Result<CollectionTx<K, V>, Error> {
    let tx = self.conn.unchecked_transaction()?;
    Ok(CollectionTx::new(tx, self.name.clone()))
  }
}

impl<K, V> fmt::Debug for Collection<K, V> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("Collection")
      .field("name", &self.name)
      .finish()
  }
}
