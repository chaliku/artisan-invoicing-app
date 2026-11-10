use serde::{Deserialize, Serialize};
use specta::Type;
use thiserror::Error;

#[derive(Debug, Error, Clone, Serialize, Deserialize, Type)]
pub enum InvoiceError {
    #[error("Missing justification for price exceeding high bound")]
    MissingJustification,

    #[error("Invalid total calculation: negative values are not allowed")]
    InvalidTotal,

    #[error("PDF generation failed: {0}")]
    PdfGenerationFailed(String),

    #[error("Share action failed: {0}")]
    ShareFailed(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Client name contains Arabic characters. Only Latin script is allowed.")]
    InvalidClientName,
}
