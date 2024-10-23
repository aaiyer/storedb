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
  println!("All User IDs before overwrite: {:?}", keys);

  drop(tx);

  // Demonstrate overwriting a user with `set`
  let mut tx = db.begin()?;
  let user_overwrite = User {
    id: 1,
    name: "Charlie".to_string(), // New name for user with ID 1
  };
  tx.set(user_overwrite.id, user_overwrite.clone())?; // Overwrite the existing user
  tx.commit()?;

  // Check the updated user
  let tx = db.begin()?;
  if let Some(user) = tx.get(1)? {
    println!("Updated User: {:?}", user); // Should print Charlie
  }

  drop(tx);

  // Show keys after overwriting
  let tx = db.begin()?;
  let keys = tx.keys()?;
  println!("All User IDs after overwrite: {:?}", keys);

  drop(tx);

  // Demonstrate error handling by attempting to insert an existing key
  let mut tx = db.begin()?;
  let user_duplicate = User {
    id: 1,
    name: "Dave".to_string(), // Attempting to add a duplicate ID
  };
  let result = tx.put(user_duplicate.id, user_duplicate.clone());
  match result {
    Ok(_) => println!("Inserted duplicate user successfully."),
    Err(e) => println!("Error inserting duplicate user: {}", e),
  }
  tx.rollback()?; // Rollback since insertion failed

  // Final state of the database
  let tx = db.begin()?;
  let entries = tx.scan()?;
  println!("Final state of the database:");
  for (id, user) in entries {
    println!("ID: {}, User: {:?}", id, user);
  }

  Ok(())
}
