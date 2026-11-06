use printpdf::*;
use tauri::{AppHandle, Manager};
use crate::errors::InvoiceError; // تصحيح: errors وليس error
use crate::models::Invoice; // نحتاج استيراد نموذج الفاتورة
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

#[tauri::command]
#[specta::specta] // تصحيح: إضافة هذا السطر ضروري لـ Specta
pub fn generate_invoice_pdf(
    app_handle: AppHandle,
    invoice: Invoice,
    invoice_type: String,
) -> Result<String, InvoiceError> {
    let (doc, page1, layer1) = PdfDocument::new("Invoice", Mm(210.0), Mm(297.0), "Layer 1");
    let current_layer = doc.get_page(page1).get_layer(layer1);

    let font = doc
    .add_builtin_font(BuiltinFont::Helvetica)
    .map_err(|e| InvoiceError::PdfGenerationFailed(format!("Font error: {}", e)))?;

    let mut y = 280.0;
    let left_margin = 20.0;
    let right_margin = 190.0;

    // Header: Invoice Type and Date
    let header_text = if invoice_type == "Facture" { "Facture Plombier" } else { "Devis" };

    // تصحيح: استخدام use_text بدلاً من set_text_cursor
    current_layer.use_text(header_text, 20.0, Mm(right_margin - 70.0), Mm(y), &font);

    y -= 10.0;
    current_layer.use_text(&format!("Date: {}", format_date()), 12.0, Mm(right_margin - 70.0), Mm(y), &font);

    // Client Name
    y -= 30.0;
    current_layer.use_text(&format!("Client: {}", invoice.client_name), 14.0, Mm(left_margin), Mm(y), &font);

    // Items Table Header
    y -= 20.0;
    current_layer.use_text("Description", 12.0, Mm(left_margin), Mm(y), &font);
    current_layer.use_text("Prix", 12.0, Mm(right_margin - 40.0), Mm(y), &font);

    // Items Loop
    for item in &invoice.items {
        y -= 10.0;
        if y < 60.0 { break; } // Prevent text overflow

        current_layer.use_text(&item.name, 12.0, Mm(left_margin), Mm(y), &font);
        let price_str = format_price(item.price);
        current_layer.use_text(&price_str, 12.0, Mm(right_margin - 40.0), Mm(y), &font);
    }

    // Totals Section
    y -= 20.0;
    current_layer.use_text(&format!("Main d'oeuvre: {}", format_price(invoice.labor_fee)), 12.0, Mm(right_margin - 80.0), Mm(y), &font);

    y -= 10.0;
    current_layer.use_text(&format!("Transport: {}", format_price(invoice.transport_fee)), 12.0, Mm(right_margin - 80.0), Mm(y), &font);

    y -= 15.0;
    current_layer.use_text(&format!("TOTAL: {}", format_price(invoice.total)), 14.0, Mm(right_margin - 80.0), Mm(y), &font);

    // Save PDF
    let app_data_dir = app_handle.path().app_data_dir()
    .map_err(|_| InvoiceError::PdfGenerationFailed("Path resolution error".into()))?;

    let invoices_dir = app_data_dir.join("invoices");
    fs::create_dir_all(&invoices_dir)
    .map_err(|_| InvoiceError::PdfGenerationFailed("Directory creation error".into()))?;

    let timestamp = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map_err(|_| InvoiceError::PdfGenerationFailed("System time error".into()))?
    .as_secs();

    let file_path = invoices_dir.join(format!("invoice_{}.pdf", timestamp));

    let file = fs::File::create(&file_path)
    .map_err(|_| InvoiceError::PdfGenerationFailed("File creation error".into()))?;

    let mut writer = std::io::BufWriter::new(file);
    doc.save(&mut writer)
    .map_err(|_| InvoiceError::PdfGenerationFailed("PDF save error".into()))?;

    Ok(file_path.to_string_lossy().to_string())
}

// --- Helper Functions ---

fn format_price(price: f64) -> String {
    let whole = price as i64;
    let frac = ((price - whole as f64) * 100.0).round() as i64;
    let whole_str = whole.to_string();
    let mut formatted = String::new();

    for (i, c) in whole_str.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            formatted.push(',');
        }
        formatted.push(c);
    }
    let formatted = formatted.chars().rev().collect::<String>();
    format!("{}.{:02} DZD", formatted, frac)
}

fn format_date() -> String {
    // Simple date for now
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let days = (now.as_secs() / 86400) as i64;
    let (y, m, d) = days_to_ymd(days);
    format!("{:02}/{:02}/{}", d, m, y)
}

fn days_to_ymd(mut days: i64) -> (i64, i64, i64) {
    let mut y = 1970;
    loop {
        let days_in_year = if (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0) { 366 } else { 365 };
        if days < days_in_year { break; }
        days -= days_in_year;
        y += 1;
    }
    let mut m = 1;
    let days_in_months = if (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    for &dim in &days_in_months {
        if days < dim { break; }
        days -= dim;
        m += 1;
    }
    (y, m, days + 1)
}
