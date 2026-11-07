use crate::errors::InvoiceError;
use crate::models::Invoice;
use printpdf::*;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

// دالة تنسيق السعر
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

// دالة رسم التصميم الاحترافي
pub fn draw_invoice_layout(
    layer: &PdfLayerReference,
    font: &IndirectFontRef,
    invoice: &Invoice,
    date: &str, // نمرر التاريخ هنا لأن نموذج Invoice لا يحتويه
) -> Result<(), InvoiceError> {
    // تعريف الألوان
    let red = Color::Rgb(Rgb::new(0.94, 0.26, 0.27, None)); // #f14345
    let black = Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None));
    let white = Color::Rgb(Rgb::new(1.0, 1.0, 1.0, None));
    let dark_red = Color::Rgb(Rgb::new(0.43, 0.23, 0.22, None)); // #6f3c39

    // دالة مساعدة لرسم مستطيل ملون
    let draw_rect =
        |layer: &PdfLayerReference, x1: f64, y1: f64, x2: f64, y2: f64, color: Color| {
            layer.set_fill_color(color);
            let points = vec![
                (Point::new(Mm(x1), Mm(y1)), false),
                (Point::new(Mm(x2), Mm(y1)), false),
                (Point::new(Mm(x2), Mm(y2)), false),
                (Point::new(Mm(x1), Mm(y2)), false),
            ];
            layer.add_polygon(Polygon {
                rings: vec![PolygonRing { points }],
                mode: PaintMode::Fill,
            });
        };

    // 1. الترويسة (Header)
    layer.set_fill_color(red);
    layer.use_text("FACTURE", 24.0, Mm(150.0), Mm(270.0), font);

    layer.set_fill_color(black);
    layer.use_text("Plombier", 16.0, Mm(20.0), Mm(270.0), font); // يمكن تغييرها لاحقاً باسم السباك

    // 2. معلومات العميل (Client Info)
    layer.use_text(
        &format!("Facturé à: {}", invoice.client_name),
        12.0,
        Mm(20.0),
        Mm(240.0),
        font,
    );
    layer.use_text(&format!("Date: {}", date), 12.0, Mm(20.0), Mm(230.0), font);

    // 3. ترويسة الجدول (Table Header)
    draw_rect(layer, 20.0, 200.0, 190.0, 208.0, red);
    layer.set_fill_color(white);
    layer.use_text("Description", 12.0, Mm(25.0), Mm(202.0), font);
    layer.use_text("Prix", 12.0, Mm(160.0), Mm(202.0), font);

    // 4. حلقة القطع (Items Loop)
    let mut current_y = 195.0;
    for item in &invoice.items {
        if current_y < 80.0 {
            break;
        } // منع تجاوز الصفحة
        layer.set_fill_color(black);
        layer.use_text(&item.name, 12.0, Mm(25.0), Mm(current_y), font);
        layer.use_text(
            &format_price(item.price),
            12.0,
            Mm(160.0),
            Mm(current_y),
            font,
        );
        current_y -= 10.0;
    }

    // 5. صندوق المجموع (Totals Box)
    let total_y = current_y - 10.0;
    draw_rect(layer, 120.0, total_y, 190.0, total_y + 15.0, dark_red);
    layer.set_fill_color(white);
    layer.use_text(
        &format!("TOTAL: {}", format_price(invoice.total)),
        12.0,
        Mm(125.0),
        Mm(total_y + 5.0),
        font,
    );

    // 6. التذييل (Footer)
    layer.set_fill_color(black);
    layer.use_text("Développé par Yassu", 10.0, Mm(20.0), Mm(20.0), font);

    Ok(())
}

// الأمر الذي سيستدعيه التطبيق
#[tauri::command]
#[specta::specta]
pub fn generate_invoice_pdf(
    app_handle: AppHandle,
    invoice: Invoice,
    invoice_type: String,
    date: String,
) -> Result<String, InvoiceError> {
    let (doc, page1, layer1) = PdfDocument::new("Invoice", Mm(210.0), Mm(297.0), "Layer 1");
    let current_layer = doc.get_page(page1).get_layer(layer1);

    let font = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| InvoiceError::PdfGenerationFailed(format!("Font error: {}", e)))?;

    // استدعاء دالة الرسم
    draw_invoice_layout(&current_layer, &font, &invoice, &date)?;

    // حفظ الملف مؤقتاً في مجلد النظام
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
