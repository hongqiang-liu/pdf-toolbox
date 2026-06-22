use serde::Serialize;
use tauri::{Emitter, Manager};
use tauri_plugin_dialog::DialogExt;

use crate::core::{
    export_images, extract_text, merge_pdfs, split_pdf, ImageExportOptions, MergeOptions,
    PdfToolboxError, ProgressEvent, SplitOptions, TextExtractOptions,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TaskResult {
    ok: bool,
    paths: Vec<String>,
    message: String,
}

#[tauri::command]
async fn split_pdf_task(
    app: tauri::AppHandle,
    options: SplitOptions,
) -> Result<TaskResult, PdfToolboxError> {
    run_background(app, "split", move |progress| {
        split_pdf(options, Some(progress)).map(|paths| {
            paths
                .into_iter()
                .map(|path| path.to_string_lossy().to_string())
                .collect::<Vec<_>>()
        })
    })
    .await
}

#[tauri::command]
async fn merge_pdf_task(
    app: tauri::AppHandle,
    options: MergeOptions,
) -> Result<TaskResult, PdfToolboxError> {
    run_background(app, "merge", move |progress| {
        merge_pdfs(options, Some(progress)).map(|path| vec![path.to_string_lossy().to_string()])
    })
    .await
}

#[tauri::command]
async fn text_pdf_task(
    app: tauri::AppHandle,
    options: TextExtractOptions,
) -> Result<TaskResult, PdfToolboxError> {
    run_background(app, "text", move |progress| {
        extract_text(options, Some(progress)).map(|path| vec![path.to_string_lossy().to_string()])
    })
    .await
}

#[tauri::command]
async fn image_pdf_task(
    app: tauri::AppHandle,
    options: ImageExportOptions,
) -> Result<TaskResult, PdfToolboxError> {
    run_background(app, "img", move |progress| {
        export_images(options, Some(progress)).map(|paths| {
            paths
                .into_iter()
                .map(|path| path.to_string_lossy().to_string())
                .collect::<Vec<_>>()
        })
    })
    .await
}

#[tauri::command]
async fn open_path(path: String) -> Result<(), PdfToolboxError> {
    tauri_plugin_opener::open_path(path, None::<&str>)
        .map_err(|err| PdfToolboxError::Task(err.to_string()))
}

#[tauri::command]
async fn pick_pdf_files(app: tauri::AppHandle) -> Result<Vec<String>, PdfToolboxError> {
    let files = app
        .dialog()
        .file()
        .add_filter("PDF", &["pdf"])
        .blocking_pick_files()
        .unwrap_or_default();
    Ok(files
        .into_iter()
        .filter_map(|path| path.into_path().ok())
        .map(|path| path.to_string_lossy().to_string())
        .collect::<Vec<_>>())
}

#[tauri::command]
async fn pick_output_file(
    app: tauri::AppHandle,
    default_name: String,
) -> Result<Option<String>, PdfToolboxError> {
    let file = app
        .dialog()
        .file()
        .set_file_name(&default_name)
        .blocking_save_file();
    Ok(file
        .and_then(|path| path.into_path().ok())
        .map(|path| path.to_string_lossy().to_string()))
}

#[tauri::command]
async fn pick_output_dir(app: tauri::AppHandle) -> Result<Option<String>, PdfToolboxError> {
    let dir = app.dialog().file().blocking_pick_folder();
    Ok(dir
        .and_then(|path| path.into_path().ok())
        .map(|path| path.to_string_lossy().to_string()))
}

async fn run_background<F>(
    app: tauri::AppHandle,
    task: &'static str,
    work: F,
) -> Result<TaskResult, PdfToolboxError>
where
    F: FnOnce(&mut dyn FnMut(ProgressEvent)) -> Result<Vec<String>, PdfToolboxError>
        + Send
        + 'static,
{
    let task_name = task.to_string();
    let app_for_task = app.clone();
    tokio::task::spawn_blocking(move || {
        let mut progress = |event: ProgressEvent| {
            let _ = app_for_task.emit("task-progress", &event);
        };

        match work(&mut progress) {
            Ok(paths) => {
                let result = TaskResult {
                    ok: true,
                    paths,
                    message: format!("{task_name} completed"),
                };
                let _ = app_for_task.emit("task-complete", &result);
                Ok(result)
            }
            Err(err) => {
                let result = TaskResult {
                    ok: false,
                    paths: Vec::new(),
                    message: err.to_string(),
                };
                let _ = app_for_task.emit("task-complete", &result);
                Err(err)
            }
        }
    })
    .await
    .map_err(|err| PdfToolboxError::Task(err.to_string()))?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            split_pdf_task,
            merge_pdf_task,
            text_pdf_task,
            image_pdf_task,
            open_path,
            pick_pdf_files,
            pick_output_file,
            pick_output_dir
        ])
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_title("PDF Toolbox");
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}
