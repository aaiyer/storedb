//! # StoreDB
//!
//! StoreDB is a disk-backed, transactional key-value database built using `rusqlite` in Rust. It provides a simple interface
//! for storing and retrieving serialized key-value pairs, ensuring consistency through transactional support.
//!
//! ## Key Features
//! - **Transactional Support**: Offers robust support for read-only and writable transactions, enabling safe modifications with commit and rollback capabilities.
//! - **Key-Value Storage**: Designed to store and retrieve serialized keys and values that implement `serde`'s `Serialize` and `Deserialize` traits, making it highly flexible for different data types.
//!
//! ## Example Usage
//! Here's a quick example demonstrating how to use StoreDB:
//! ```rust
//! use storedb::{Db, Error};
//! use serde::{Serialize, Deserialize};
//! use std::fs;
//!
//! #[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
//! struct User {
//!   id: u32,
//!   name: String,
//! }
//!
//! fn main() -> Result<(), Error> {
//!   // Remove existing database file if any
//!   let _ = fs::remove_file("example.db");
//!
//!   // Initialize the database
//!   let mut db: Db<u32, User> = Db::new("example.db")?;
//!
//!   // Start a write transaction
//!   let mut tx = db.begin()?;
//!
//!   // Insert users into the database
//!   let user1 = User { id: 1, name: "Alice".to_string() };
//!   let user2 = User { id: 2, name: "Bob".to_string() };
//!   tx.put(user1.id, user1.clone())?;
//!   tx.put(user2.id, user2.clone())?;
//!
//!   // Commit the transaction
//!   tx.commit()?;
//!
//!   // Start a read transaction
//!   let tx = db.begin()?;
//!
//!   // Check existence and retrieve a user
//!   let exists = tx.contains(1)?;
//!   println!("User with ID 1 exists: {}", exists);
//!
//!   if let Some(user) = tx.get(1)? {
//!     println!("Retrieved User: {:?}", user);
//!   }
//!
//!   // Retrieve all keys and entries
//!   let keys = tx.keys()?;
//!   println!("All User IDs: {:?}", keys);
//!   let entries = tx.scan()?;
//!   for (id, user) in entries {
//!     println!("ID: {}, User: {:?}", id, user);
//!   }
//!
//!   Ok(())
//! }
//! ```
//!
//! StoreDB aims to provide an efficient and easy-to-use interface for managing key-value data with strong transactional guarantees.

pub mod db;
pub mod err;
pub mod tx;

pub use db::*;
pub use err::*;
pub use tx::*;
