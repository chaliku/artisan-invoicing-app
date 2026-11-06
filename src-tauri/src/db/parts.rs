use rusqlite::params;
use std::path::Path;
use tauri::Manager;

use super::DB;
use crate::errors::InvoiceError;
use crate::models::Part;

#[tauri::command]
#[specta::specta]
pub fn add_part(
    name: String,
    low_bound: Option<f64>,
    high_bound: Option<f64>,
    image_path: Option<String>,
) -> Result<i64, InvoiceError> {
    let mut db = DB
        .lock()
        .map_err(|_| InvoiceError::IoError("DB lock poisoned".into()))?;
    let conn = db
        .as_mut()
        .ok_or_else(|| InvoiceError::DatabaseError("Database not initialized".into()))?;

    conn.execute(
        "INSERT INTO parts (name, low_bound, high_bound, image_path) VALUES (?1, ?2, ?3, ?4)",
        params![name, low_bound, high_bound, image_path],
    )
    .map_err(|e| InvoiceError::DatabaseError(format!("Failed to insert part: {e}")))?;

    Ok(conn.last_insert_rowid())
}

#[tauri::command]
#[specta::specta]
pub fn get_parts() -> Result<Vec<Part>, InvoiceError> {
    let mut db = DB
        .lock()
        .map_err(|_| InvoiceError::IoError("DB lock poisoned".into()))?;
    let conn = db
        .as_mut()
        .ok_or_else(|| InvoiceError::DatabaseError("Database not initialized".into()))?;

    let mut stmt = conn
        .prepare("SELECT id, name, low_bound, high_bound, image_path FROM parts")
        .map_err(|e| InvoiceError::DatabaseError(format!("Failed to prepare statement: {e}")))?;

    let parts = stmt
        .query_map([], |row| {
            Ok(Part {
                id: row.get(0)?,
                name: row.get(1)?,
                low_bound: row.get(2)?,
                high_bound: row.get(3)?,
                image_path: row.get(4)?,
            })
        })
        .map_err(|e| InvoiceError::DatabaseError(format!("Failed to query parts: {e}")))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| InvoiceError::DatabaseError(format!("Failed to collect parts: {e}")))?;

    Ok(parts)
}

#[tauri::command]
#[specta::specta]
pub fn update_part(
    id: i64,
    name: String,
    low_bound: Option<f64>,
    high_bound: Option<f64>,
    image_path: Option<String>,
) -> Result<(), InvoiceError> {
    let mut db = DB
        .lock()
        .map_err(|_| InvoiceError::IoError("DB lock poisoned".into()))?;
    let conn = db
        .as_mut()
        .ok_or_else(|| InvoiceError::DatabaseError("Database not initialized".into()))?;

    let rows_affected = conn
        .execute(
            "UPDATE parts SET name = ?1, low_bound = ?2, high_bound = ?3, image_path = ?4 WHERE id = ?5",
            params![name, low_bound, high_bound, image_path, id],
        )
        .map_err(|e| InvoiceError::DatabaseError(format!("Failed to update part: {e}")))?;

    if rows_affected == 0 {
        return Err(InvoiceError::DatabaseError("Part not found".into()));
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn delete_part(id: i64) -> Result<(), InvoiceError> {
    let mut db = DB
        .lock()
        .map_err(|_| InvoiceError::IoError("DB lock poisoned".into()))?;
    let conn = db
        .as_mut()
        .ok_or_else(|| InvoiceError::DatabaseError("Database not initialized".into()))?;

    let rows_affected = conn
        .execute("DELETE FROM parts WHERE id = ?1", params![id])
        .map_err(|e| InvoiceError::DatabaseError(format!("Failed to delete part: {e}")))?;

    if rows_affected == 0 {
        return Err(InvoiceError::DatabaseError("Part not found".into()));
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn save_part_image(
    app_handle: tauri::AppHandle,
    source_path: String,
) -> Result<String, InvoiceError> {
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| InvoiceError::IoError(format!("Failed to get app data dir: {e}")))?;

    let images_dir = data_dir.join("images");
    std::fs::create_dir_all(&images_dir)
        .map_err(|e| InvoiceError::IoError(format!("Failed to create images dir: {e}")))?;

    let source = Path::new(&source_path);
    let extension = source
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("png");

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| InvoiceError::IoError(format!("Failed to get timestamp: {e}")))?
        .as_millis();

    let filename = format!("image_{}.{}", timestamp, extension);
    let dest_path = images_dir.join(&filename);

    std::fs::copy(&source_path, &dest_path)
        .map_err(|e| InvoiceError::IoError(format!("Failed to copy image: {e}")))?;

    dest_path
        .to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| InvoiceError::IoError("Invalid image path".into()))
}
