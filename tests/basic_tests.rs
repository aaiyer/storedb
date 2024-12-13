use storedb::{Database, Error};
use serde::{Serialize, Deserialize};
use tempfile::NamedTempFile;

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone)]
struct TestVal {
  data: String,
}

#[test]
fn test_collection_creation_and_use() -> Result<(), Error> {
  let temp_file = NamedTempFile::new().unwrap();
  let db_path = temp_file.path().to_str().unwrap();
  let mut db = Database::new(db_path)?;

  let mut coll = db.get_collection::<String, String>("test_coll")?;

  {
    let mut tx = coll.begin()?;
    tx.set("key1".to_string(), "value1".to_string())?;
    tx.commit()?;
  }

  {
    let tx = coll.begin()?;
    let val = tx.get("key1".to_string())?;
    assert_eq!(val, Some("value1".to_string()));
  }

  Ok(())
}

#[test]
fn test_collection_type_mismatch() -> Result<(), Error> {
  let temp_file = NamedTempFile::new().unwrap();
  let db_path = temp_file.path().to_str().unwrap();
  let mut db = Database::new(db_path)?;

  // Create with (String, String)
  let mut coll = db.get_collection::<String,String>("mismatch")?;
  {
    let mut tx = coll.begin()?;
    tx.set("k".to_string(), "v".to_string())?;
    tx.commit()?;
  }

  // Try retrieving as (u32, TestVal)
  let err = db.get_collection::<u32, TestVal>("mismatch").unwrap_err();
  match err {
    Error::TypeMismatch { .. } => (),
    _ => panic!("Expected TypeMismatch error"),
  }

  Ok(())
}
