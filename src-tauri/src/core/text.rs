use std::fs;
use std::path::{Path, PathBuf};

use pdfium_render::prelude::*;

use crate::core::progress::{emit_progress, ProgressSink};
use crate::core::{PdfToolboxError, Result};
use crate::utils::fs::{ensure_parent_dir, require_pdf};

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextExtractOptions {
    pub input: PathBuf,
    pub output: PathBuf,
    pub with_page_markers: bool,
}

/// Extracts text from a standard text PDF into a UTF-8 `.txt` file.
///
/// Example:
/// ```ignore
/// extract_text(TextExtractOptions {
///     input: "book.pdf".into(),
///     output: "book.txt".into(),
///     with_page_markers: true,
/// }, None)?;
/// ```
pub fn extract_text(
    options: TextExtractOptions,
    mut progress: Option<ProgressSink<'_>>,
) -> Result<PathBuf> {
    require_pdf(&options.input)?;
    ensure_parent_dir(&options.output)?;

    let pdfium = create_pdfium()?;
    let document = pdfium
        .load_pdf_from_file(&options.input, None)
        .map_err(|err| PdfToolboxError::Pdfium(err.to_string()))?;

    let page_count = document.pages().len() as usize;
    if page_count == 0 {
        return Err(PdfToolboxError::DamagedPdf(options.input));
    }

    let mut output = String::new();
    for (index, page) in document.pages().iter().enumerate() {
        let text_page = page
            .text()
            .map_err(|err| PdfToolboxError::Pdfium(err.to_string()))?;
        let page_text = text_page.all();

        if options.with_page_markers {
            output.push_str(&format!("----- Page {} -----\n", index + 1));
        }
        output.push_str(page_text.trim());
        output.push_str("\n\n");

        emit_progress(
            &mut progress,
            "text",
            index + 1,
            page_count,
            "extracted page text",
        );
    }

    if output.trim().is_empty() {
        return Err(PdfToolboxError::NoExtractableText);
    }

    fs::write(&options.output, output)?;
    Ok(options.output)
}

pub(crate) fn create_pdfium() -> Result<Pdfium> {
    let executable_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.to_path_buf()));
    let bundled = executable_dir
        .as_ref()
        .map(Pdfium::pdfium_platform_library_name_at_path)
        .ok_or_else(|| {
            PdfToolboxError::Pdfium("failed to locate executable directory".to_string())
        });

    let bindings = bundled
        .and_then(bind_pdfium_library)
        .or_else(|_| extract_embedded_pdfium().and_then(bind_pdfium_library))
        .or_else(|_| {
            Pdfium::bind_to_system_library().map_err(|err| PdfToolboxError::Pdfium(err.to_string()))
        })
        .map_err(|err| {
            PdfToolboxError::Pdfium(format!(
                "failed to initialize PDFium from executable directory, embedded bytes, or system library: {err}"
            ))
        })?;
    Ok(Pdfium::new(bindings))
}

fn bind_pdfium_library(path: PathBuf) -> Result<Box<dyn PdfiumLibraryBindings>> {
    Pdfium::bind_to_library(path).map_err(|err| PdfToolboxError::Pdfium(err.to_string()))
}

fn extract_embedded_pdfium() -> Result<PathBuf> {
    let bytes = embedded_pdfium_bytes();
    let output = std::env::temp_dir()
        .join("pdf_toolbox")
        .join(env!("CARGO_PKG_VERSION"))
        .join(pdfium_library_name());

    if !same_file_len(&output, bytes.len() as u64) {
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&output, bytes)?;
    }

    Ok(output)
}

fn same_file_len(path: &Path, expected_len: u64) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.len() == expected_len)
        .unwrap_or(false)
}

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
fn embedded_pdfium_bytes() -> &'static [u8] {
    include_bytes!("../../pdfium/x86_64-pc-windows-msvc/bin/pdfium.dll")
}

#[cfg(all(target_os = "windows", target_arch = "aarch64"))]
fn embedded_pdfium_bytes() -> &'static [u8] {
    include_bytes!("../../pdfium/aarch64-pc-windows-msvc/bin/pdfium.dll")
}

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
fn embedded_pdfium_bytes() -> &'static [u8] {
    include_bytes!("../../pdfium/x86_64-apple-darwin/lib/libpdfium.dylib")
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn embedded_pdfium_bytes() -> &'static [u8] {
    include_bytes!("../../pdfium/aarch64-apple-darwin/lib/libpdfium.dylib")
}

#[cfg(target_os = "windows")]
fn pdfium_library_name() -> &'static str {
    "pdfium.dll"
}

#[cfg(target_os = "macos")]
fn pdfium_library_name() -> &'static str {
    "libpdfium.dylib"
}
