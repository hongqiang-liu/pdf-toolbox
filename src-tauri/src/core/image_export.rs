use std::path::PathBuf;

use image::ImageFormat as EncoderFormat;
use pdfium_render::prelude::*;

use crate::core::progress::{emit_progress, ProgressSink};
use crate::core::text::create_pdfium;
use crate::core::{PdfToolboxError, Result};
use crate::utils::fs::{ensure_dir, require_pdf};

#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    Png,
    Jpg,
}

impl ImageFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpg => "jpg",
        }
    }

    fn encoder_format(self) -> EncoderFormat {
        match self {
            Self::Png => EncoderFormat::Png,
            Self::Jpg => EncoderFormat::Jpeg,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageExportOptions {
    pub input: PathBuf,
    pub output_dir: PathBuf,
    pub dpi: u16,
    pub format: ImageFormat,
    pub pages: Option<String>,
}

/// Renders PDF pages to PNG or JPG images using PDFium.
///
/// Example:
/// ```ignore
/// export_images(ImageExportOptions {
///     input: "source.pdf".into(),
///     output_dir: "images".into(),
///     dpi: 300,
///     format: ImageFormat::Png,
///     pages: Some("1-3,9".into()),
/// }, None)?;
/// ```
pub fn export_images(
    options: ImageExportOptions,
    mut progress: Option<ProgressSink<'_>>,
) -> Result<Vec<PathBuf>> {
    require_pdf(&options.input)?;
    ensure_dir(&options.output_dir)?;
    if options.dpi == 0 {
        return Err(PdfToolboxError::InvalidArgument(
            "DPI must be greater than zero".to_string(),
        ));
    }

    let pdfium = create_pdfium()?;
    let document = pdfium
        .load_pdf_from_file(&options.input, None)
        .map_err(|err| PdfToolboxError::Pdfium(err.to_string()))?;
    let total_pages = document.pages().len() as u32;
    if total_pages == 0 {
        return Err(PdfToolboxError::DamagedPdf(options.input));
    }

    let pages = match options.pages.as_deref() {
        Some(value) if !value.trim().is_empty() => parse_pages(value, total_pages)?,
        _ => (1..=total_pages).collect(),
    };

    let stem = options
        .input
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("page");
    let mut outputs = Vec::with_capacity(pages.len());

    for (index, page_number) in pages.iter().enumerate() {
        let page = document
            .pages()
            .get((*page_number - 1) as u16)
            .map_err(|err| PdfToolboxError::Pdfium(err.to_string()))?;
        let width = points_to_pixels(page.width().value, options.dpi);
        let height = points_to_pixels(page.height().value, options.dpi);
        let bitmap = page
            .render_with_config(
                &PdfRenderConfig::new()
                    .set_target_width(width)
                    .set_target_height(height)
                    .render_form_data(true),
            )
            .map_err(|err| PdfToolboxError::Pdfium(err.to_string()))?;
        let image = bitmap.as_image();
        let output = options
            .output_dir
            .join(format!("{stem}_page_{:04}.{}", page_number, options.format.extension()));
        image
            .save_with_format(&output, options.format.encoder_format())
            .map_err(|err| PdfToolboxError::Task(err.to_string()))?;

        emit_progress(
            &mut progress,
            "img",
            index + 1,
            pages.len(),
            format!("rendered {}", output.display()),
        );
        outputs.push(output);
    }

    Ok(outputs)
}

fn points_to_pixels(points: f32, dpi: u16) -> i32 {
    ((points / 72.0) * f32::from(dpi)).round().max(1.0) as i32
}

fn parse_pages(value: &str, total_pages: u32) -> Result<Vec<u32>> {
    let mut pages = Vec::new();
    for raw_part in value.split(',') {
        let part = raw_part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((start, end)) = part.split_once('-') {
            let start = parse_page(start, total_pages)?;
            let end = parse_page(end, total_pages)?;
            if start > end {
                return Err(PdfToolboxError::InvalidArgument(format!(
                    "range start is greater than range end: {part}"
                )));
            }
            pages.extend(start..=end);
        } else {
            pages.push(parse_page(part, total_pages)?);
        }
    }
    if pages.is_empty() {
        return Err(PdfToolboxError::InvalidArgument(
            "empty page list".to_string(),
        ));
    }
    Ok(pages)
}

fn parse_page(value: &str, total_pages: u32) -> Result<u32> {
    let page = value.trim().parse::<u32>().map_err(|_| {
        PdfToolboxError::InvalidArgument(format!("invalid page number: {}", value.trim()))
    })?;
    if page == 0 || page > total_pages {
        return Err(PdfToolboxError::InvalidArgument(format!(
            "page {page} is out of bounds; document has {total_pages} pages"
        )));
    }
    Ok(page)
}
