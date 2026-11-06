use printpdf::*;
use tauri::{AppHandle, Manager};
use crate::errors::InvoiceError;

#[tauri::command]
#[specta::specta]
pub async fn generate_test_pdf(app_handle: AppHandle) -> Result<String, InvoiceError> {
    let (doc, page_idx, layer_idx) = PdfDocument::new(
        "Test Invoice", 
        Mm(210.0), 
        Mm(297.0), 
        "Layer 1"
    );
    
    let page = doc.get_page(page_idx);
    let layer = page.get_layer(layer_idx);

    // ملاحظة: تأكد أن ملف الخط موجود في src-tauri/arabic-font.ttf
    let font_bytes = include_bytes!("../../arabic-font.ttf");
    let font = doc.add_external_font(std::io::Cursor::new(font_bytes))
        .map_err(|e| InvoiceError::PdfGenerationFailed(format!("Failed to load font: {}", e)))?;

    // التصحيح: استخدام use_text بدلاً من use_font و write_text
    layer.use_text("فاتورة السباك - Facture Plombier", 14.0, Mm(40.0), Mm(250.0), &font);
    layer.use_text("العميل: أحمد - 1,500,000.00 DZD", 14.0, Mm(40.0), Mm(230.0), &font);

    let app_data_dir = app_handle.path().app_data_dir()
        .map_err(|e| InvoiceError::PdfGenerationFailed(format!("Failed to resolve app data dir: {}", e)))?;
    
    std::fs::create_dir_all(&app_data_dir)
        .map_err(|e| InvoiceError::PdfGenerationFailed(format!("Failed to create app data dir: {}", e)))?;

    let file_path = app_data_dir.join("test_invoice.pdf");
    let file = std::fs::File::create(&file_path)
        .map_err(|e| InvoiceError::PdfGenerationFailed(format!("Failed to create PDF file: {}", e)))?;

    let mut writer = std::io::BufWriter::new(file);
    doc.save(&mut writer)
        .map_err(|e| InvoiceError::PdfGenerationFailed(format!("Failed to save PDF document: {}", e)))?;

    Ok(file_path.to_string_lossy().into_owned())
}