# StoreDB

StoreDB is a disk-backed transactional key-value database built using `rusqlite` in Rust. It provides a simple interface for storing and retrieving serialized key-value pairs using `postcard` for serialization.

## Features

- **Transactional Support**: Supports read-only and writable transactions.
- **Key-Value Storage**: Stores serialized keys and values that implement serde's `Serialize` and `Deserialize`.
- **Convenient API**: Methods accept `Into<K>` and `Into<V>` for convenience.

## Example

```rust
use storedb::{Db, Error};
use serde::{Serialize, Deserialize};
use std::fs;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
struct User {
  id: u32,
  name: String,
}

fn main() -> Result<(), Error> {
  let _ = fs::remove_file("example.db");
  let mut db: Db<u32, User> = Db::new("example.db")?;

  // Start a write transaction
  let mut tx = db.begin()?;
  tx.put(1u32, User { id: 1, name: "Alice".into() })?;
  tx.put(2u32, User { id: 2, name: "Bob".into() })?;
  tx.commit()?;

  // Start a read transaction
  let tx = db.begin()?;
  let exists = tx.contains(1u32)?;
  println!("User with ID 1 exists: {}", exists);

  if let Some(user) = tx.get(1u32)? {
    println!("Retrieved User: {:?}", user);
  }

  let keys = tx.keys()?;
  println!("All User IDs: {:?}", keys);

  let entries = tx.scan()?;
  for (id, user) in entries {
    println!("ID: {}, User: {:?}", id, user);
  }

  Ok(())
}
