use std::path::PathBuf;

use lopdf::Document;

use crate::core::progress::{emit_progress, ProgressSink};
use crate::core::{PdfToolboxError, Result};
use crate::utils::fs::{ensure_dir, require_pdf};

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SplitMode {
    Ranges { ranges: String },
    Every { pages_per_file: u32 },
    Single { page: u32 },
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SplitOptions {
    pub input: PathBuf,
    pub output_dir: PathBuf,
    pub mode: SplitMode,
}

/// Splits a PDF by range, fixed chunk size, or single page.
///
/// Example:
/// ```ignore
/// split_pdf(SplitOptions {
///     input: "book.pdf".into(),
///     output_dir: "out".into(),
///     mode: SplitMode::Ranges { ranges: "1-5,8".into() },
/// }, None)?;
/// ```
pub fn split_pdf(options: SplitOptions, mut progress: Option<ProgressSink<'_>>) -> Result<Vec<PathBuf>> {
    require_pdf(&options.input)?;
    ensure_dir(&options.output_dir)?;

    let document = Document::load(&options.input).map_err(|err| classify_lopdf_error(err, &options.input))?;
    if document.is_encrypted() {
        return Err(PdfToolboxError::EncryptedPdf(options.input));
    }

    let total_pages = document.get_pages().len() as u32;
    if total_pages == 0 {
        return Err(PdfToolboxError::DamagedPdf(options.input));
    }

    let groups = match &options.mode {
        SplitMode::Ranges { ranges } => parse_page_ranges(ranges, total_pages)?,
        SplitMode::Every { pages_per_file } => build_equal_groups(*pages_per_file, total_pages)?,
        SplitMode::Single { page } => {
            validate_page(*page, total_pages)?;
            vec![vec![*page]]
        }
    };

    let stem = options
        .input
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("split");
    let mut outputs = Vec::with_capacity(groups.len());

    for (index, pages) in groups.iter().enumerate() {
        let first = pages.first().copied().unwrap_or(1);
        let last = pages.last().copied().unwrap_or(first);
        let output = options
            .output_dir
            .join(format!("{stem}_p{first}-{last}_{:03}.pdf", index + 1));

        let mut split_doc = document.clone();
        let keep: std::collections::BTreeSet<u32> = pages.iter().copied().collect();
        let delete_pages: Vec<u32> = (1..=total_pages)
            .filter(|page| !keep.contains(page))
            .collect();
        split_doc.delete_pages(&delete_pages);
        split_doc.prune_objects();
        split_doc.renumber_objects();
        split_doc.compress();
        split_doc.save(&output)?;

        emit_progress(
            &mut progress,
            "split",
            index + 1,
            groups.len(),
            format!("created {}", output.display()),
        );
        outputs.push(output);
    }

    Ok(outputs)
}

fn parse_page_ranges(ranges: &str, total_pages: u32) -> Result<Vec<Vec<u32>>> {
    let mut groups = Vec::new();
    for raw_part in ranges.split(',') {
        let part = raw_part.trim();
        if part.is_empty() {
            continue;
        }

        if let Some((start, end)) = part.split_once('-') {
            let start = parse_page_number(start)?;
            let end = parse_page_number(end)?;
            if start > end {
                return Err(PdfToolboxError::InvalidArgument(format!(
                    "range start is greater than range end: {part}"
                )));
            }
            validate_page(start, total_pages)?;
            validate_page(end, total_pages)?;
            groups.push((start..=end).collect());
        } else {
            let page = parse_page_number(part)?;
            validate_page(page, total_pages)?;
            groups.push(vec![page]);
        }
    }

    if groups.is_empty() {
        return Err(PdfToolboxError::InvalidArgument(
            "empty page range; expected values like 1-5,7".to_string(),
        ));
    }
    Ok(groups)
}

fn build_equal_groups(pages_per_file: u32, total_pages: u32) -> Result<Vec<Vec<u32>>> {
    if pages_per_file == 0 {
        return Err(PdfToolboxError::InvalidArgument(
            "pages per file must be greater than zero".to_string(),
        ));
    }
    let mut groups = Vec::new();
    let mut current = 1;
    while current <= total_pages {
        let end = (current + pages_per_file - 1).min(total_pages);
        groups.push((current..=end).collect());
        current = end + 1;
    }
    Ok(groups)
}

fn parse_page_number(value: &str) -> Result<u32> {
    value.trim().parse::<u32>().map_err(|_| {
        PdfToolboxError::InvalidArgument(format!("invalid page number: {}", value.trim()))
    })
}

fn validate_page(page: u32, total_pages: u32) -> Result<()> {
    if page == 0 || page > total_pages {
        return Err(PdfToolboxError::InvalidArgument(format!(
            "page {page} is out of bounds; document has {total_pages} pages"
        )));
    }
    Ok(())
}

fn classify_lopdf_error(err: lopdf::Error, path: &std::path::Path) -> PdfToolboxError {
    let message = err.to_string().to_ascii_lowercase();
    if message.contains("encrypted") || message.contains("password") {
        PdfToolboxError::EncryptedPdf(path.to_path_buf())
    } else {
        PdfToolboxError::PdfParse(err.to_string())
    }
}
