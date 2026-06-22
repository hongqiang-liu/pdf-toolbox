use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PdfToolboxError {
    #[error("file IO failed: {0}")]
    Io(#[from] std::io::Error),

    #[error("PDF parsing failed: {0}")]
    PdfParse(String),

    #[error("PDFium operation failed: {0}")]
    Pdfium(String),

    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    #[error("permission denied for path: {0}")]
    PermissionDenied(PathBuf),

    #[error("encrypted PDFs are not supported: {0}")]
    EncryptedPdf(PathBuf),

    #[error("damaged or unsupported PDF: {0}")]
    DamagedPdf(PathBuf),

    #[error("no extractable text was found; this may be a scanned image PDF")]
    NoExtractableText,

    #[error("task failed: {0}")]
    Task(String),
}

impl serde::Serialize for PdfToolboxError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, PdfToolboxError>;

impl From<lopdf::Error> for PdfToolboxError {
    fn from(value: lopdf::Error) -> Self {
        let message = value.to_string();
        let lower = message.to_ascii_lowercase();
        if lower.contains("encrypted") || lower.contains("password") {
            Self::PdfParse("encrypted PDF requires a password".to_string())
        } else {
            Self::PdfParse(message)
        }
    }
}
