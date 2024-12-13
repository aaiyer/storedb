use serde::{Deserialize, Serialize};
use std::fs;

use storedb::{Db, Error};

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

  // Start a transaction
  let mut tx = db.begin()?;

  // Insert some users
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
  println!("All User IDs before overwrite: {:?}", keys);

  drop(tx);

  // Overwrite a user with `set`
  let mut tx = db.begin()?;
  tx.set(1u32, User { id: 1, name: "Charlie".into() })?;
  tx.commit()?;

  let tx = db.begin()?;
  if let Some(user) = tx.get(1u32)? {
    println!("Updated User: {:?}", user);
  }
  drop(tx);

  let tx = db.begin()?;
  let keys = tx.keys()?;
  println!("All User IDs after overwrite: {:?}", keys);
  drop(tx);

  // Demonstrate error handling by attempting to put an existing key
  let mut tx = db.begin()?;
  let result = tx.put(1u32, User { id: 1, name: "Dave".into() });
  match result {
    Ok(_) => println!("Inserted duplicate user successfully."),
    Err(e) => println!("Error inserting duplicate user: {}", e),
  }
  tx.rollback()?;

  // Final state of the database
  let tx = db.begin()?;
  let entries = tx.scan()?;
  println!("Final state of the database:");
  for (id, user) in entries {
    println!("ID: {}, User: {:?}", id, user);
  }

  Ok(())
}
