# StoreDB

StoreDB is a disk-backed transactional database that supports multiple named collections. Each collection is stored in a single table (`kv_store`) and separated by a `collection` column. Type metadata for each collection is stored in `collection_meta`.

## Features

- **Named Collections**: Organize keys/values in a single underlying table.
- **Type Safety**: Each collection enforces specific K,V types.
- **Transactional Support**: Atomic transactions per collection.
- **Disk-backed**: Uses SQLite with `rusqlite` and `postcard` for serialization.

## Example

```rust
use serde::{Serialize, Deserialize};
use storedb::{Database, Error};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
struct User {
    id: u32,
    name: String,
}

fn main() -> Result<(), Error> {
    let mut db = Database::new("example.db")?;
    let mut users = db.get_collection::<u32, User>("users")?;

    {
        let mut tx = users.begin()?;
        tx.put(1u32, User { id: 1, name: "Alice".into() })?;
        tx.put(2u32, User { id: 2, name: "Bob".into() })?;
        tx.commit()?;
    }

    let tx = users.begin()?;
    if let Some(user) = tx.get(1u32)? {
        println!("Retrieved User: {:?}", user);
    }
    let keys = tx.keys()?;
    println!("All User IDs: {:?}", keys);

    Ok(())
}
```
