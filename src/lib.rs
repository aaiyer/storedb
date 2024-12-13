//! # StoreDB
//!
//! StoreDB is a disk-backed, transactional key-value database built using `rusqlite` in Rust.
//! It supports multiple named collections stored in a single underlying table.
//! Uses `postcard` for serialization.
//!
//! ## Example Usage
//! ```rust
//! use serde::{Serialize, Deserialize};
//! use storedb::{Database, Error};
//!
//! #[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
//! struct User {
//!   id: u32,
//!   name: String,
//! }
//!
//! fn main() -> Result<(), Error> {
//!   let mut db = Database::new("example.db")?;
//!   let mut users = db.get_collection::<u32, User>("users")?;
//!   let mut tx = users.begin()?;
//!
//!   tx.put(1u32, User { id: 1, name: "Alice".into() })?;
//!   tx.commit()?;
//!
//!   let tx = users.begin()?;
//!   let user = tx.get(1u32)?;
//!   println!("User: {:?}", user);
//!
//!   Ok(())
//! }
//! ```

mod database;
mod err;
mod collection;
mod collection_tx;

pub use database::*;
pub use err::*;
pub use collection::*;
pub use collection_tx::*;
