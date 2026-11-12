use crate::errors::InvoiceError;
use crate::models::Invoice;
use printpdf::path::{PaintMode, WindingOrder};
use printpdf::*;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

fn format_price(price: f64) -> Result<String, InvoiceError> {
    // Decision #032 / Rule 22: negatives are invalid (no credit memo model
    // in v1.0); NaN/Inf are never valid prices. Both return Err(InvalidTotal)
    // rather than panic or produce "0.00 DZD" for garbage input.
    if !price.is_finite() || price < 0.0 {
        return Err(InvoiceError::InvalidTotal);
    }
    // Cents-first, half-up. Rounding before integer division avoids the
    // E-05 fraction overflow: the old `(price - whole) * 100` path yielded
    // frac==100 for 1234.999, formatting as "1,234.100 DZD" instead of
    // "1,235.00 DZD".
    let cents = (price * 100.0).round() as i64;
    let whole = cents / 100;
    let frac = cents % 100;
    let whole_str = whole.to_string();
    let mut formatted = String::new();
    for (i, c) in whole_str.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            formatted.push(',');
        }
        formatted.push(c);
    }
    let formatted = formatted.chars().rev().collect::<String>();
    Ok(format!("{}.{:02} DZD", formatted, frac))
}

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

// TODO Commit 6: refactor to InvoiceLayoutParams struct (12 args after photo bytes)
#[allow(clippy::too_many_arguments)]
pub fn draw_invoice_layout(
    _doc: &PdfDocumentReference,
    layer: &PdfLayerReference,
    font: &IndirectFontRef,
    invoice: &Invoice,
    date: &str,
    plumber_name: &str,
    plumber_phone: &str,
    plumber_address: &str,
    signature_path: Option<String>,
    developer_qr_bytes: Option<Vec<u8>>,
) -> Result<(), InvoiceError> {
    let red = Color::Rgb(Rgb::new(0.94, 0.26, 0.27, None));
    let black = Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None));
    let white = Color::Rgb(Rgb::new(1.0, 1.0, 1.0, None));
    let dark_red = Color::Rgb(Rgb::new(0.43, 0.23, 0.22, None));
    let dark_grey = Color::Rgb(Rgb::new(0.1, 0.1, 0.1, None));

    // 1. Header (Plumber Info)
    layer.set_fill_color(red.clone());
    layer.use_text("FACTURE", 24.0, Mm(150.0), Mm(270.0), font);

    layer.set_fill_color(black.clone());
    layer.use_text(plumber_name, 16.0, Mm(20.0), Mm(270.0), font);
    layer.use_text(plumber_phone, 10.0, Mm(20.0), Mm(262.0), font);
    layer.use_text(plumber_address, 10.0, Mm(20.0), Mm(256.0), font);

    // 2. Client Info
    layer.use_text(
        format!("Facturé à: {}", invoice.client_name),
        12.0,
        Mm(20.0),
        Mm(240.0),
        font,
    );
    layer.use_text(format!("Date: {}", date), 12.0, Mm(20.0), Mm(230.0), font);

    // 3. Table Header
    draw_rect(layer, 20.0, 200.0, 190.0, 208.0, red.clone());
    layer.set_fill_color(white.clone());
    layer.use_text("Description", 12.0, Mm(25.0), Mm(202.0), font);
    layer.use_text("Prix", 12.0, Mm(160.0), Mm(202.0), font);

    // 4. Items Loop (Text only, as per recent decision)
    let mut current_y: f32 = 195.0;
    for item in &invoice.items {
        if current_y < 80.0 {
            break;
        }
        layer.set_fill_color(black.clone());
        layer.use_text(&item.name, 12.0, Mm(25.0), Mm(current_y), font);
        layer.use_text(
            format_price(item.price)?,
            12.0,
            Mm(160.0),
            Mm(current_y),
            font,
        );
        current_y -= 10.0;
    }

    // 5. Totals Breakdown (Task 1)
    let items_total = invoice.total - invoice.labor_fee - invoice.transport_fee;
    let mut total_y = current_y - 10.0;

    layer.set_fill_color(black.clone());
    layer.use_text("Sous-total:", 10.0, Mm(100.0), Mm(total_y), font);
    layer.use_text(
        format_price(items_total)?,
        10.0,
        Mm(160.0),
        Mm(total_y),
        font,
    );

    total_y -= 8.0;
    layer.use_text("Main d'œuvre:", 10.0, Mm(100.0), Mm(total_y), font);
    layer.use_text(
        format_price(invoice.labor_fee)?,
        10.0,
        Mm(160.0),
        Mm(total_y),
        font,
    );

    total_y -= 8.0;
    layer.use_text("Transport:", 10.0, Mm(100.0), Mm(total_y), font);
    layer.use_text(
        format_price(invoice.transport_fee)?,
        10.0,
        Mm(160.0),
        Mm(total_y),
        font,
    );

    total_y -= 12.0;
    draw_rect(
        layer,
        120.0,
        total_y - 15.0,
        190.0,
        total_y,
        dark_red.clone(),
    );
    layer.set_fill_color(white.clone());
    layer.use_text("TOTAL:", 12.0, Mm(125.0), Mm(total_y - 10.0), font);
    layer.use_text(
        format_price(invoice.total)?,
        12.0,
        Mm(155.0),
        Mm(total_y - 10.0),
        font,
    );

    // 6. Signature Image (at y=55)
    if let Some(path) = signature_path {
        if let Ok(bytes) = std::fs::read(&path) {
            if let Ok(dynamic_image) = ::image::load_from_memory(&bytes) {
                use ::image::GenericImageView;
                let (_, height_px) = dynamic_image.dimensions();
                let height_mm = (height_px as f32 / 300.0) * 25.4;
                if height_mm > 0.0 {
                    let max_height_mm = 30.0;
                    let scale_factor = if height_mm > max_height_mm {
                        max_height_mm / height_mm
                    } else {
                        1.0
                    };
                    let pdf_image = Image::from_dynamic_image(&dynamic_image);
                    let transform = ImageTransform {
                        translate_x: Some(Mm(20.0)),
                        translate_y: Some(Mm(55.0)),
                        scale_x: Some(scale_factor),
                        scale_y: Some(scale_factor),
                        rotate: None,
                        dpi: None,
                    };
                    pdf_image.add_to_layer(layer.clone(), transform);
                }
            }
        }
    } else {
        layer.set_fill_color(black.clone());
        layer.use_text("Signature", 12.0, Mm(20.0), Mm(55.0), font);
    }

    // 7. Dark Footer Bar & QR Code (Task 3)
    // Draw a dark grey rectangle from y=0 to y=25
    draw_rect(layer, 0.0, 0.0, 210.0, 25.0, dark_grey);

    layer.set_fill_color(white.clone());
    // Left Side: Developer Branding
    layer.use_text("Développé par Yassu", 10.0, Mm(20.0), Mm(15.0), font);
    layer.use_text("Merci de votre confiance", 8.0, Mm(20.0), Mm(7.0), font);

    // Center: Contact Email
    layer.use_text(
        "Contact: An-Yassu-kei@proton.me",
        8.0,
        Mm(85.0),
        Mm(11.0),
        font,
    );

    // Right Side: QR Code
    if let Some(bytes) = developer_qr_bytes {
        if let Ok(dynamic_image) = ::image::load_from_memory(&bytes) {
            use ::image::GenericImageView;
            let (_, height_px) = dynamic_image.dimensions();
            let height_mm = (height_px as f32 / 300.0) * 25.4;
            if height_mm > 0.0 {
                // Scale QR to max 18mm height
                let max_height_mm = 18.0;
                let scale_factor = if height_mm > max_height_mm {
                    max_height_mm / height_mm
                } else {
                    1.0
                };
                let pdf_image = Image::from_dynamic_image(&dynamic_image);
                let transform = ImageTransform {
                    translate_x: Some(Mm(170.0)),
                    translate_y: Some(Mm(3.5)),
                    scale_x: Some(scale_factor),
                    scale_y: Some(scale_factor),
                    rotate: None,
                    dpi: None,
                };
                pdf_image.add_to_layer(layer.clone(), transform);
            }
        }
    }

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
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|_| InvoiceError::PdfGenerationFailed("Path resolution error".into()))?;

    // Fetch plumber info
    let plumber_name = crate::db::get_setting("plumber_name".to_string())?
        .unwrap_or_else(|| "Plomberie".to_string());
    let plumber_phone =
        crate::db::get_setting("plumber_phone".to_string())?.unwrap_or_else(|| "".to_string());
    let plumber_address =
        crate::db::get_setting("plumber_address".to_string())?.unwrap_or_else(|| "".to_string());
    let signature_path = crate::db::get_setting("signature_path".to_string())?
        .map(|p| app_data_dir.join(p).to_string_lossy().to_string());

    // E-03 (Commit 3): bundle QR at compile time. Missing file = compile error,
    // not a silent `.ok()` failure. Path from src-tauri/src/pdf/mod.rs →
    // ../../assets/qr_code.png = src-tauri/assets/qr_code.png.
    let developer_qr_bytes = Some(include_bytes!("../../assets/qr_code.png").to_vec());

    let (doc, page1, layer1) = PdfDocument::new("Invoice", Mm(210.0), Mm(297.0), "Layer 1");
    let current_layer = doc.get_page(page1).get_layer(layer1);
    let font = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| InvoiceError::PdfGenerationFailed(format!("Font error: {}", e)))?;

    draw_invoice_layout(
        &doc,
        &current_layer,
        &font,
        &invoice,
        &date,
        &plumber_name,
        &plumber_phone,
        &plumber_address,
        signature_path,
        developer_qr_bytes,
    )?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_price_gate_4_values() {
        // §10.2 GATE 4 — canonical money formatting cases.
        assert_eq!(format_price(0.005).ok().as_deref(), Some("0.01 DZD"));
        assert_eq!(format_price(0.001).ok().as_deref(), Some("0.00 DZD"));
        assert_eq!(format_price(0.004).ok().as_deref(), Some("0.00 DZD"));
        assert_eq!(format_price(0.999).ok().as_deref(), Some("1.00 DZD"));
        assert_eq!(format_price(99.99).ok().as_deref(), Some("99.99 DZD"));
        assert_eq!(format_price(1234.999).ok().as_deref(), Some("1,235.00 DZD"));
        assert_eq!(
            format_price(1234567.89).ok().as_deref(),
            Some("1,234,567.89 DZD")
        );
        assert_eq!(format_price(0.00).ok().as_deref(), Some("0.00 DZD"));
    }

    #[test]
    fn format_price_rejects_negatives() {
        assert!(matches!(
            format_price(-1.00),
            Err(InvoiceError::InvalidTotal)
        ));
        assert!(matches!(
            format_price(-0.01),
            Err(InvoiceError::InvalidTotal)
        ));
    }

    #[test]
    fn format_price_rejects_non_finite() {
        assert!(matches!(
            format_price(f64::NAN),
            Err(InvoiceError::InvalidTotal)
        ));
        assert!(matches!(
            format_price(f64::INFINITY),
            Err(InvoiceError::InvalidTotal)
        ));
        assert!(matches!(
            format_price(f64::NEG_INFINITY),
            Err(InvoiceError::InvalidTotal)
        ));
    }
}
