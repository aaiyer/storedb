use serde::{Serialize, Deserialize};
use std::fs;
use storedb::{Database, Error};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
struct User {
  id: u32,
  name: String,
}

fn main() -> Result<(), Error> {
  let _ = fs::remove_file("example.db");

  let mut db = Database::new("example.db")?;
  let mut users = db.get_collection::<u32, User>("users")?;

  {
    let mut tx = users.begin()?;
    tx.put(1u32, User { id: 1, name: "Alice".into() })?;
    tx.put(2u32, User { id: 2, name: "Bob".into() })?;
    tx.commit()?;
  }

  {
    let tx = users.begin()?;
    println!("User with ID 1 exists: {}", tx.contains(1u32)?);
    if let Some(user) = tx.get(1u32)? {
      println!("Retrieved User: {:?}", user);
    }
    let keys = tx.keys()?;
    println!("All User IDs before overwrite: {:?}", keys);
  }

  {
    let mut tx = users.begin()?;
    tx.set(1u32, User { id: 1, name: "Charlie".into() })?;
    tx.commit()?;
  }

  {
    let tx = users.begin()?;
    if let Some(user) = tx.get(1u32)? {
      println!("Updated User: {:?}", user);
    }
    let keys = tx.keys()?;
    println!("All User IDs after overwrite: {:?}", keys);
  }

  {
    let mut tx = users.begin()?;
    let result = tx.put(1u32, User { id: 1, name: "Dave".into() });
    match result {
      Ok(_) => println!("Inserted duplicate user successfully."),
      Err(e) => println!("Error inserting duplicate user: {}", e),
    }
    tx.rollback()?;
  }

  {
    let tx = users.begin()?;
    let entries = tx.scan()?;
    println!("Final state of the database:");
    for (id, user) in entries {
      println!("ID: {}, User: {:?}", id, user);
    }
  }

  Ok(())
}
