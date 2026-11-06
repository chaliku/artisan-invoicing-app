use rusqlite::params;
use std::sync::Mutex;
use tauri::Manager;

use crate::errors::InvoiceError;
use crate::models::{calculate_total, InvoicePayload};

pub mod parts;

static DB: Mutex<Option<rusqlite::Connection>> = Mutex::new(None);

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
        ",
    )
    .map_err(|e| InvoiceError::DatabaseError(format!("Failed to create tables: {e}")))?;

    // ترقية آمنة: إضافة العمود إذا كان الجدول موجوداً مسبقاً
    let _ = conn.execute("ALTER TABLE parts ADD COLUMN image_path TEXT;", []);

    let mut db = DB
        .lock()
        .map_err(|_| InvoiceError::IoError("DB lock poisoned".into()))?;
    *db = Some(conn);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn save_invoice(payload: InvoicePayload, date: String) -> Result<i64, InvoiceError> {
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

    tx.commit()
        .map_err(|e| InvoiceError::DatabaseError(format!("Failed to commit transaction: {e}")))?;

    Ok(invoice_id)
}