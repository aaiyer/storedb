# StoreDB

StoreDB is a disk-backed transactional key-value database built using `rusqlite` in Rust. It provides a simple interface for storing and retrieving serialized key-value pairs with transactional support.

## Features

- **Transactional Support**: Supports read-only and writable transactions.
- **Key-Value Storage**: Stores serialized keys and values that implements serde's serialization traits.

## Example

```rust
use storedb::{Db, Error};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
struct User {
  id: u32,
  name: String,
}

fn main() -> Result<(), Error> {
  // Remove existing database file if any
  let _ = fs::remove_file("example.db");

  // Initialize the database
  let mut db: Db<u32, User> = Db::new("example.db")?;

  // Start a write transaction
  let mut tx = db.begin()?;

  // Create some users
  let user1 = User {
    id: 1,
    name: "Alice".to_string(),
  };
  let user2 = User {
    id: 2,
    name: "Bob".to_string(),
  };

  // Insert users into the database
  tx.put(user1.id, user1.clone())?;
  tx.put(user2.id, user2.clone())?;

  // Commit the transaction
  tx.commit()?;

  // Start a read transaction
  let tx = db.begin()?;

  // Check if a key exists
  let exists = tx.contains(1)?;
  println!("User with ID 1 exists: {}", exists);

  // Retrieve a user
  if let Some(user) = tx.get(1)? {
    println!("Retrieved User: {:?}", user);
  }

  // Retrieve all keys
  let keys = tx.keys()?;
  println!("All User IDs: {:?}", keys);

  // Retrieve all key-value pairs
  let entries = tx.scan()?;
  for (id, user) in entries {
    println!("ID: {}, User: {:?}", id, user);
  }

  // Demonstrate error handling by attempting to insert an existing key
  let mut tx = db.begin(true)?;
  let user_duplicate = User {
    id: 1,
    name: "Charlie".to_string(),
  };
  let result = tx.put(user_duplicate.id, user_duplicate.clone());
  match result {
    Ok(_) => println!("Inserted duplicate user successfully."),
    Err(e) => println!("Error inserting duplicate user: {}", e),
  }
  tx.rollback()?; // Rollback since insertion failed

  // Final state of the database
  let tx = db.begin(false)?;
  let entries = tx.scan()?;
  println!("Final state of the database:");
  for (id, user) in entries {
    println!("ID: {}, User: {:?}", id, user);
  }

  Ok(())
}
```
