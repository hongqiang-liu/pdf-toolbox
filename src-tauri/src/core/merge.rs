use std::collections::BTreeMap;
use std::path::PathBuf;

use lopdf::{dictionary, Dictionary, Document, Object, ObjectId};

use crate::core::progress::{emit_progress, ProgressSink};
use crate::core::{PdfToolboxError, Result};
use crate::utils::fs::{ensure_parent_dir, require_pdf};

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum MergeItem {
    Pdf { path: PathBuf },
    Blank { width: f32, height: f32 },
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeOptions {
    pub items: Vec<MergeItem>,
    pub output: PathBuf,
}

/// Merges PDFs and optional blank pages into one output document.
///
/// Example:
/// ```ignore
/// merge_pdfs(MergeOptions {
///     items: vec![MergeItem::Pdf { path: "a.pdf".into() }],
///     output: "all.pdf".into(),
/// }, None)?;
/// ```
pub fn merge_pdfs(options: MergeOptions, mut progress: Option<ProgressSink<'_>>) -> Result<PathBuf> {
    if options.items.is_empty() {
        return Err(PdfToolboxError::InvalidArgument(
            "at least one PDF or blank page is required".to_string(),
        ));
    }
    ensure_parent_dir(&options.output)?;

    let mut all_objects = BTreeMap::<ObjectId, Object>::new();
    let mut page_ids = Vec::<ObjectId>::new();
    let mut output = Document::with_version("1.5");

    let catalog_id = output.new_object_id();
    let pages_id = output.new_object_id();
    let mut max_id = output.max_id + 1;

    for (index, item) in options.items.iter().enumerate() {
        match item {
            MergeItem::Pdf { path } => {
                require_pdf(path)?;
                let mut doc = Document::load(path)?;
                if doc.is_encrypted() {
                    return Err(PdfToolboxError::EncryptedPdf(path.clone()));
                }
                doc.renumber_objects_with(max_id);
                max_id = doc.max_id + 1;

                let pages = doc.get_pages();
                for (_, page_id) in pages {
                    page_ids.push(page_id);
                }

                for (object_id, object) in doc.objects {
                    match object.type_name().unwrap_or("") {
                        "Catalog" | "Pages" => {}
                        "Page" => {
                            let mut dict = object.as_dict()?.clone();
                            dict.set("Parent", pages_id);
                            all_objects.insert(object_id, Object::Dictionary(dict));
                        }
                        _ => {
                            all_objects.insert(object_id, object);
                        }
                    }
                }
            }
            MergeItem::Blank { width, height } => {
                if *width <= 0.0 || *height <= 0.0 {
                    return Err(PdfToolboxError::InvalidArgument(
                        "blank page width and height must be positive".to_string(),
                    ));
                }
                let page_id = (max_id, 0);
                max_id += 1;
                let page = dictionary! {
                    "Type" => "Page",
                    "Parent" => pages_id,
                    "MediaBox" => vec![0.into(), 0.into(), (*width as f64).into(), (*height as f64).into()],
                    "Resources" => Dictionary::new()
                };
                page_ids.push(page_id);
                all_objects.insert(page_id, Object::Dictionary(page));
            }
        }

        emit_progress(
            &mut progress,
            "merge",
            index + 1,
            options.items.len(),
            "processed merge item",
        );
    }

    if page_ids.is_empty() {
        return Err(PdfToolboxError::InvalidArgument(
            "no pages were found to merge".to_string(),
        ));
    }

    for (object_id, object) in all_objects {
        output.objects.insert(object_id, object);
    }

    let kids = page_ids
        .iter()
        .map(|id| Object::Reference(*id))
        .collect::<Vec<_>>();
    output.objects.insert(
        pages_id,
        dictionary! {
            "Type" => "Pages",
            "Kids" => kids,
            "Count" => page_ids.len() as i64
        }
        .into(),
    );
    output.objects.insert(
        catalog_id,
        dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id
        }
        .into(),
    );
    output.trailer.set("Root", catalog_id);
    output.max_id = max_id;
    output.prune_objects();
    output.renumber_objects();
    output.compress();
    output.save(&options.output)?;

    emit_progress(
        &mut progress,
        "merge",
        options.items.len(),
        options.items.len(),
        format!("created {}", options.output.display()),
    );
    Ok(options.output)
}
