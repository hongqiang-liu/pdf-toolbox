use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::core::{PdfToolboxError, Result};

pub fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

pub fn ensure_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)?;
    Ok(())
}

pub fn require_pdf(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(PdfToolboxError::InvalidArgument(format!(
            "input file does not exist: {}",
            path.display()
        )));
    }
    if path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| !ext.eq_ignore_ascii_case("pdf"))
        .unwrap_or(true)
    {
        return Err(PdfToolboxError::InvalidArgument(format!(
            "input is not a PDF: {}",
            path.display()
        )));
    }
    Ok(())
}

pub fn collect_pdf_files(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for path in paths {
        if path.is_dir() {
            for entry in WalkDir::new(path)
                .into_iter()
                .filter_map(std::result::Result::ok)
            {
                let entry_path = entry.path();
                if entry_path.is_file()
                    && entry_path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| ext.eq_ignore_ascii_case("pdf"))
                        .unwrap_or(false)
                {
                    files.push(entry_path.to_path_buf());
                }
            }
        } else {
            require_pdf(path)?;
            files.push(path.clone());
        }
    }
    if files.is_empty() {
        return Err(PdfToolboxError::InvalidArgument(
            "no PDF files were provided".to_string(),
        ));
    }
    Ok(files)
}
