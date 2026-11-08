use printpdf::*;
use printpdf::path::{PaintMode, WindingOrder}; // Correct import path
use tauri::{AppHandle, Manager};
use crate::errors::InvoiceError;
use crate::models::Invoice;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

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

// Draw rectangle using add_polygon
fn draw_rect(layer: &PdfLayerReference, x1: f32, y1: f32, x2: f32, y2: f32, color: Color) {
    layer.set_fill_color(color);
    let points = vec![
        (Point::new(Mm(x1), Mm(y1)), false),
        (Point::new(Mm(x2), Mm(y1)), false),
        (Point::new(Mm(x2), Mm(y2)), false),
        (Point::new(Mm(x1), Mm(y2)), false),
    ];
    layer.add_polygon(Polygon {
        rings: vec![points],
        mode: PaintMode::Fill,
        winding_order: WindingOrder::NonZero,
    });
}

pub fn draw_invoice_layout(
    _doc: &PdfDocumentReference,
    layer: &PdfLayerReference,
    font: &IndirectFontRef,
    invoice: &Invoice,
    date: &str,
) -> Result<(), InvoiceError> {
    let red = Color::Rgb(Rgb::new(0.94, 0.26, 0.27, None));
    let black = Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None));
    let white = Color::Rgb(Rgb::new(1.0, 1.0, 1.0, None));
    let dark_red = Color::Rgb(Rgb::new(0.43, 0.23, 0.22, None));

    // 1. Header
    layer.set_fill_color(red.clone());
    layer.use_text("FACTURE", 24.0, Mm(150.0), Mm(270.0), font);
    layer.set_fill_color(black.clone());
    layer.use_text("Plombier", 16.0, Mm(20.0), Mm(270.0), font);

    // 2. Client Info
    layer.use_text(&format!("Facturé à: {}", invoice.client_name), 12.0, Mm(20.0), Mm(240.0), font);
    layer.use_text(&format!("Date: {}", date), 12.0, Mm(20.0), Mm(230.0), font);

    // 3. Table Header
    draw_rect(layer, 20.0, 200.0, 190.0, 208.0, red.clone());
    layer.set_fill_color(white.clone());
    layer.use_text("Description", 12.0, Mm(25.0), Mm(202.0), font);
    layer.use_text("Prix", 12.0, Mm(160.0), Mm(202.0), font);

    // 4. Items Loop with Images
    let mut current_y: f32 = 195.0;
    for item in &invoice.items {
        if current_y < 80.0 { break; }

        let mut text_x_position: f32 = 25.0;

        if let Some(path) = &item.receipt_image_path {
            if let Ok(bytes) = std::fs::read(path) {
                if let Ok(dynamic_image) = ::image::load_from_memory(&bytes) {
                    use ::image::GenericImageView;
                    let (_, height_px) = dynamic_image.dimensions();
                    let height_mm = (height_px as f32 / 300.0) * 25.4;

                    if height_mm > 0.0 {
                        let scale_factor = 10.0 / height_mm;
                        let pdf_image = Image::from_dynamic_image(&dynamic_image);

                        let transform = ImageTransform {
                            translate_x: Some(Mm(20.0)),
                            translate_y: Some(Mm(current_y - 5.0)),
                            scale_x: Some(scale_factor),
                            scale_y: Some(scale_factor),
                            rotate: None,
                            dpi: None,
                        };

                        pdf_image.add_to_layer(layer.clone(), transform);
                        text_x_position = 35.0;
                    }
                }
            }
        }

        layer.set_fill_color(black.clone());
        layer.use_text(&item.name, 12.0, Mm(text_x_position), Mm(current_y), font);
        layer.use_text(&format_price(item.price), 12.0, Mm(160.0), Mm(current_y), font);
        current_y -= 10.0;
    }

    // 5. Totals Box
    let total_y = current_y - 10.0;
    draw_rect(layer, 120.0, total_y, 190.0, total_y + 15.0, dark_red.clone());
    layer.set_fill_color(white.clone());
    layer.use_text(&format!("TOTAL: {}", format_price(invoice.total)), 12.0, Mm(125.0), Mm(total_y + 5.0), font);

    // 6. Footer
    layer.set_fill_color(black.clone());
    layer.use_text("Développé par Yassu", 10.0, Mm(20.0), Mm(20.0), font);

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn generate_invoice_pdf(
    app_handle: AppHandle,
    invoice: Invoice,
    _invoice_type: String,
    date: String,
) -> Result<String, InvoiceError> {
    let (doc, page1, layer1) = PdfDocument::new("Invoice", Mm(210.0), Mm(297.0), "Layer 1");
    let current_layer = doc.get_page(page1).get_layer(layer1);

    let font = doc
    .add_builtin_font(BuiltinFont::Helvetica)
    .map_err(|e| InvoiceError::PdfGenerationFailed(format!("Font error: {}", e)))?;

    draw_invoice_layout(&doc, &current_layer, &font, &invoice, &date)?;

    let app_data_dir = app_handle
    .path()
    .app_data_dir()
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
