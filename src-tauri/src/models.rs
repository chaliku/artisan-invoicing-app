use crate::errors::InvoiceError;
use serde::{Deserialize, Serialize};
use specta::Type;

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Client {
    pub id: Option<i64>,
    pub name: String,
    pub phone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct InvoiceItem {
    pub name: String,
    pub price: f64,
    pub receipt_image_path: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct InvoicePayload {
    pub client_name: String,
    pub items: Vec<InvoiceItem>,
    pub labor_fee: f64,
    pub transport_fee: f64,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Invoice {
    pub client_name: String,
    pub items: Vec<InvoiceItem>,
    pub labor_fee: f64,
    pub transport_fee: f64,
    pub total: f64,
    pub before_photo_path: Option<String>,
    pub after_photo_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Part {
    pub id: i64,
    pub name: String,
    pub low_bound: Option<f64>,
    pub high_bound: Option<f64>,
    pub image_path: Option<String>,
}

pub fn calculate_total(
    items: &[InvoiceItem],
    labor: f64,
    transport: f64,
) -> Result<f64, InvoiceError> {
    if labor < 0.0 || transport < 0.0 {
        return Err(InvoiceError::InvalidTotal);
    }
    let mut total = labor + transport;
    for item in items {
        if item.price < 0.0 {
            return Err(InvoiceError::InvalidTotal);
        }
        total += item.price;
    }
    Ok(total)
}
