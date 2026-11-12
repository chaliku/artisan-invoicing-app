use rusqlite::params;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};
use crate::errors::InvoiceError;
use crate::models::{calculate_total, Invoice, InvoicePayload};

pub mod parts;

static DB: Mutex<Option<rusqlite::Connection>> = Mutex::new(None);

// ─────────────────────────────────────────────
// Centralized Latin Validation
// ─────────────────────────────────────────────
pub fn validate_latin(text: &str, field_name: &str) -> Result<(), InvoiceError> {
    if text.chars().any(|c| matches!(c, '\u{0600}'..='\u{06FF}')) {
        return Err(InvoiceError::InvalidLatinText(field_name.to_string()));
    }
    Ok(())
}

pub fn initialize_db(app_handle: &tauri::AppHandle) -> Result<(), InvoiceError> {
    let data_dir = app_handle
    .path()
    .app_data_dir()
    .map_err(|e| InvoiceError::IoError(e.to_string()))?;
    std::fs::create_dir_all(&data_dir)
    .map_err(|e| InvoiceError::IoError(format!("Failed to create data dir: {e}")))?;
    let db_path = data_dir.join("plumber.db");
    let conn = rusqlite::Connection::open(&db_path)
    .map_err(|e| InvoiceError::DatabaseError(format!("Failed to open DB: {e}")))?;

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS clients (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            phone TEXT
    );
    CREATE TABLE IF NOT EXISTS invoices (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        client_id INTEGER,
        date TEXT NOT NULL,
        labor_fee REAL NOT NULL,
        transport_fee REAL NOT NULL,
        total REAL NOT NULL,
        FOREIGN KEY(client_id) REFERENCES clients(id)
    );
    CREATE TABLE IF NOT EXISTS invoice_items (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        invoice_id INTEGER,
        name TEXT NOT NULL,
        price REAL NOT NULL,
        receipt_image_path TEXT,
        note TEXT,
        FOREIGN KEY(invoice_id) REFERENCES invoices(id)
    );
    CREATE TABLE IF NOT EXISTS parts (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL,
        low_bound REAL,
        high_bound REAL,
        image_path TEXT
    );
    CREATE TABLE IF NOT EXISTS settings (
        key TEXT PRIMARY KEY,
        value TEXT
    );
    CREATE TABLE IF NOT EXISTS work_photos (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        invoice_id INTEGER NOT NULL,
        location_number INTEGER NOT NULL,
        before_path TEXT,
        after_path TEXT,
        FOREIGN KEY(invoice_id) REFERENCES invoices(id)
    );
    ",
    )
    .map_err(|e| InvoiceError::DatabaseError(format!("Failed to create tables: {e}")))?;

    let _ = conn.execute("ALTER TABLE parts ADD COLUMN image_path TEXT;", []);
    let mut db = DB
    .lock()
    .map_err(|_| InvoiceError::IoError("DB lock poisoned".into()))?;
    *db = Some(conn);
    Ok(())
}

fn insert_invoice_to_db(
    payload: &InvoicePayload,
    date: &str,
) -> Result<i64, InvoiceError> {
    if payload.client_name.trim().is_empty() {
        return Err(InvoiceError::InvalidClientName);
    }
    validate_latin(&payload.client_name, "client_name")?;

    for item in &payload.items {
        if item.name.trim().is_empty() {
            return Err(InvoiceError::EmptyField("item_name".to_string()));
        }
        validate_latin(&item.name, "item_name")?;
        if let Some(note) = &item.note {
            if !note.trim().is_empty() {
                validate_latin(note, "item_note")?;
            }
        }
    }

    let mut db = DB
    .lock()
    .map_err(|_| InvoiceError::IoError("DB lock poisoned".into()))?;
    let conn = db
    .as_mut()
    .ok_or_else(|| InvoiceError::DatabaseError("Database not initialized".into()))?;
    let total = calculate_total(&payload.items, payload.labor_fee, payload.transport_fee)?;
    let tx = conn
    .transaction()
    .map_err(|e| InvoiceError::DatabaseError(format!("Failed to start transaction: {e}")))?;

    let client_id: i64 = {
        let mut stmt = tx
        .prepare("SELECT id FROM clients WHERE name = ?1")
        .map_err(|e| InvoiceError::DatabaseError(format!("Failed to prepare statement: {e}")))?;
        let mut rows = stmt
        .query(params![payload.client_name])
        .map_err(|e| InvoiceError::DatabaseError(format!("Failed to query client: {e}")))?;
        if let Some(row) = rows
            .next()
            .map_err(|e| InvoiceError::DatabaseError(format!("Failed to fetch client: {e}")))?
            {
                row.get(0)
                .map_err(|e| InvoiceError::DatabaseError(format!("Failed to get client id: {e}")))?
            } else {
                tx.execute(
                    "INSERT INTO clients (name, phone) VALUES (?1, NULL)",
                           params![payload.client_name],
                )
                .map_err(|e| InvoiceError::DatabaseError(format!("Failed to insert client: {e}")))?;
                tx.last_insert_rowid()
            }
    };

    tx.execute(
        "INSERT INTO invoices (client_id, date, labor_fee, transport_fee, total) VALUES (?1, ?2, ?3, ?4, ?5)",
               params![client_id, date, payload.labor_fee, payload.transport_fee, total],
    )
    .map_err(|e| InvoiceError::DatabaseError(format!("Failed to insert invoice: {e}")))?;

    let invoice_id = tx.last_insert_rowid();

    for item in &payload.items {
        tx.execute(
            "INSERT INTO invoice_items (invoice_id, name, price, receipt_image_path, note) VALUES (?1, ?2, ?3, ?4, ?5)",
                   params![invoice_id, item.name, item.price, item.receipt_image_path, item.note],
        )
        .map_err(|e| InvoiceError::DatabaseError(format!("Failed to insert item: {e}")))?;
    }

    // Insert work photos (if any)
    if let Some(photos) = &payload.work_photos {
        for photo in photos {
            tx.execute(
                "INSERT INTO work_photos (invoice_id, location_number, before_path, after_path) VALUES (?1, ?2, ?3, ?4)",
                       params![invoice_id, photo.location_number, photo.before_path, photo.after_path],
            )
            .map_err(|e| InvoiceError::DatabaseError(format!("Failed to insert work photo: {e}")))?;
        }
    }

    tx.commit()
    .map_err(|e| InvoiceError::DatabaseError(format!("Failed to commit transaction: {e}")))?;
    Ok(invoice_id)
}

#[tauri::command]
#[specta::specta]
pub fn save_invoice(payload: InvoicePayload, date: String) -> Result<i64, InvoiceError> {
    insert_invoice_to_db(&payload, &date)
}

#[tauri::command]
#[specta::specta]
pub fn save_and_generate_pdf(
    app_handle: AppHandle,
    payload: InvoicePayload,
    date: String,
) -> Result<String, InvoiceError> {
    let _invoice_id = insert_invoice_to_db(&payload, &date)?;
    let total = calculate_total(&payload.items, payload.labor_fee, payload.transport_fee)?;

    let invoice = Invoice {
        client_name: payload.client_name,
        items: payload.items,
        labor_fee: payload.labor_fee,
        transport_fee: payload.transport_fee,
        total,
    };

    let pdf_path = crate::pdf::generate_invoice_pdf(
        app_handle,
        invoice,
        "Facture".to_string(),
                                                    date,
    )?;
    Ok(pdf_path)
}

// ─────────────────────────────────────────────
// Settings Commands (RESTORED)
// ─────────────────────────────────────────────
#[tauri::command]
#[specta::specta]
pub fn get_setting(key: String) -> Result<Option<String>, InvoiceError> {
    let mut db = DB
    .lock()
    .map_err(|_| InvoiceError::IoError("DB lock poisoned".into()))?;
    let conn = db
    .as_mut()
    .ok_or_else(|| InvoiceError::DatabaseError("Database not initialized".into()))?;
    let mut stmt = conn
    .prepare("SELECT value FROM settings WHERE key = ?1")
    .map_err(|e| InvoiceError::DatabaseError(format!("Failed to prepare statement: {e}")))?;
    let mut rows = stmt
    .query(rusqlite::params![key])
    .map_err(|e| InvoiceError::DatabaseError(format!("Failed to query setting: {e}")))?;
    if let Some(row) = rows
        .next()
        .map_err(|e| InvoiceError::DatabaseError(format!("Failed to fetch setting: {e}")))?
        {
            let value: String = row
            .get(0)
            .map_err(|e| InvoiceError::DatabaseError(format!("Failed to get setting value: {e}")))?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
}

#[tauri::command]
#[specta::specta]
pub fn set_setting(key: String, value: String) -> Result<(), InvoiceError> {
    if matches!(key.as_str(), "plumber_name" | "plumber_phone" | "plumber_address") {
        validate_latin(&value, &key)?;
    }
    let mut db = DB
    .lock()
    .map_err(|_| InvoiceError::IoError("DB lock poisoned".into()))?;
    let conn = db
    .as_mut()
    .ok_or_else(|| InvoiceError::DatabaseError("Database not initialized".into()))?;
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                 rusqlite::params![key, value],
    )
    .map_err(|e| InvoiceError::DatabaseError(format!("Failed to set setting: {e}")))?;
    Ok(())
}

// ─────────────────────────────────────────────
// Signature Commands (RESTORED)
// ─────────────────────────────────────────────
#[tauri::command]
#[specta::specta]
pub fn save_signature(
    app_handle: tauri::AppHandle,
    source_path: String,
) -> Result<String, InvoiceError> {
    let app_data_dir = app_handle
    .path()
    .app_data_dir()
    .map_err(|e| InvoiceError::IoError(format!("Path resolution error: {e}")))?;
    let dest_path = app_data_dir.join("signature.png");
    if !std::path::Path::new(&source_path).exists() {
        return Err(InvoiceError::IoError("Source file does not exist".into()));
    }
    std::fs::copy(&source_path, &dest_path)
    .map_err(|e| InvoiceError::IoError(format!("Failed to copy signature image: {e}")))?;
    set_setting("signature_path".to_string(), "signature.png".to_string())?;
    Ok(dest_path.to_string_lossy().to_string())
}

#[tauri::command]
#[specta::specta]
pub fn get_signature_path(app_handle: tauri::AppHandle) -> Result<Option<String>, InvoiceError> {
    let path_opt = get_setting("signature_path".to_string())?;
    if let Some(p) = path_opt {
        let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| InvoiceError::IoError(format!("Path resolution error: {e}")))?;
        let abs_path = app_data_dir.join(p).to_string_lossy().to_string();
        Ok(Some(abs_path))
    } else {
        Ok(None)
    }
}

// ─────────────────────────────────────────────
// Work Photo Command (Phase 6.5c)
// ─────────────────────────────────────────────
#[tauri::command]
#[specta::specta]
pub fn save_work_photo(
    app_handle: tauri::AppHandle,
    source_path: String,
) -> Result<String, InvoiceError> {
    let data_dir = app_handle
    .path()
    .app_data_dir()
    .map_err(|e| InvoiceError::IoError(format!("Failed to get app data dir: {e}")))?;
    let photos_dir = data_dir.join("photos");
    std::fs::create_dir_all(&photos_dir)
    .map_err(|e| InvoiceError::IoError(format!("Failed to create photos dir: {e}")))?;

    let source = std::path::Path::new(&source_path);
    let extension = source
    .extension()
    .and_then(|ext| ext.to_str())
    .unwrap_or("jpg");

    let timestamp = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map_err(|e| InvoiceError::IoError(format!("Failed to get timestamp: {e}")))?
    .as_millis();

    let filename = format!("photo_{}.{}", timestamp, extension);
    let dest_path = photos_dir.join(&filename);

    std::fs::copy(&source_path, &dest_path)
    .map_err(|e| InvoiceError::IoError(format!("Failed to copy photo: {e}")))?;

    Ok(dest_path.to_string_lossy().to_string())
}
