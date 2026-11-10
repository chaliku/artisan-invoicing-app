# Artisan Invoicing App

**Offline-first invoicing for artisans (plumbers, electricians, carpenters, mechanics).**

Replace verbal quotes with a transparent, 2-minute invoicing process. Generate professional A4 PDF invoices with labor/transport breakdown, signature, and QR code. No cloud, no accounts, 100% local.

## Features (Current)
- ✅ Parts catalog with images.
- ✅ Client name validation (Latin-only to prevent PDF corruption).
- ✅ Invoice PDF with breakdown: Sub-total, Labor, Transport, TOTAL.
- ✅ Signature image embedded in PDF.
- ✅ Dark footer with developer contact and QR code.
- ✅ Settings page for plumber info (name, phone, address, signature).

## Tech Stack
| Layer | Technology |
|---|---|
| UI | Svelte 5 (Runes) |
| Backend | Rust + Tauri 2 |
| PDF | printpdf 0.7 |
| Database | SQLite (rusqlite) |

## Target Platform
- **Primary:** Android (APK)
- **Dev Environment:** Fedora KDE + Podman

## Roadmap
- [ ] Before/After photos (Page 2 of PDF).
- [ ] Native Share Sheet (WhatsApp/Telegram).
- [ ] Resilience (PDF retry on failure).
- [ ] Generalization (Profession setting).
- [ ] F-Droid release.

## License
GPLv3 (to be added before first release).

## Status
🚧 **MVP in active development.** Not ready for production use yet.
