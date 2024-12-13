//! # StoreDB
//!
//! StoreDB is a disk-backed, transactional key-value database built using `rusqlite` in Rust. It uses `postcard` for serialization.
//! It provides a simple interface for storing and retrieving serialized key-value pairs.
//!
//! ## Key Features
//! - **Transactional Support**
//! - **Key-Value Storage**
//!
//! ## Example Usage
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
//!   let _ = fs::remove_file("example.db");
//!   let mut db: Db<u32, User> = Db::new("example.db")?;
//!   let mut tx = db.begin()?;
//!
//!   tx.put(1u32, User { id: 1, name: "Alice".into() })?;
//!   tx.commit()?;
//!
//!   let tx = db.begin()?;
//!   let user = tx.get(1u32)?;
//!   println!("User: {:?}", user);
//!
//!   Ok(())
//! }
//! ```

mod db;
mod err;
mod tx;

pub use db::*;
pub use err::*;
pub use tx::*;
