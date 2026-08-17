use lazymcp::{Json, LazyMcp, State, tool};

use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Default)]
struct Database {
    records: Mutex<HashMap<i32, String>>,
}

/// Save record to DB
#[tool]
async fn save_record(id: i32, value: String, db: State<Database>) -> String {
    let mut storage = db.records.lock().unwrap();
    storage.insert(id, value.clone());
    format!("Record {id} successfully saved with '{value}'")
}

/// Get record from DB
#[tool]
async fn get_record(
    /// ID of record
    id: i32,
    db: State<Database>,
) -> Result<String, String> {
    let storage = db.records.lock().unwrap();

    match storage.get(&id) {
        Some(val) => Ok(format!("Found record for ID {id}: '{val}'")),
        None => Err(format!("Record with ID {id} not found in database")),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = Database::default();

    LazyMcp::new("db-manager", "0.1.0")
        .with_state(db)
        .with_tool(save_record_tool)
        .with_tool(get_record_tool)
        .serve_stdio()
        .await?;

    Ok(())
}
