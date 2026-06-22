#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let arg_count = std::env::args_os().count();

    if arg_count > 1 {
        if let Err(err) = pdf_toolbox_lib::cli::run() {
            eprintln!("{err}");
            std::process::exit(1);
        }
    } else {
        pdf_toolbox_lib::tauri_app::run();
    }
}

