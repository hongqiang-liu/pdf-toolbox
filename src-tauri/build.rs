use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn main() {
    if let Err(err) = prepare_pdfium() {
        println!("cargo:warning=failed to prepare bundled PDFium: {err}");
        println!("cargo:warning=PDF text/image commands will require a system PDFium library.");
    }
    tauri_build::build();
}

fn prepare_pdfium() -> Result<(), Box<dyn std::error::Error>> {
    let target = env::var("TARGET")?;
    let profile = env::var("PROFILE")?;
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let cache_dir = manifest_dir.join("pdfium").join(&target);
    let lib_name = platform_library_name(&target)?;
    let cached_lib = find_file(&cache_dir, lib_name);

    let source_lib = match cached_lib {
        Some(path) => path,
        None => {
            fs::create_dir_all(&cache_dir)?;
            download_and_extract_pdfium(&target, &cache_dir)?;
            find_file(&cache_dir, lib_name).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("downloaded archive did not contain {lib_name}"),
                )
            })?
        }
    };

    let target_dir = cargo_target_profile_dir()?;
    fs::create_dir_all(&target_dir)?;
    fs::copy(&source_lib, target_dir.join(lib_name))?;

    // Tauri release builds use PROFILE=release; plain cargo run uses PROFILE=debug.
    println!("cargo:rerun-if-env-changed=PDF_TOOLBOX_SKIP_PDFIUM_DOWNLOAD");
    println!("cargo:rerun-if-changed={}", source_lib.display());
    println!("cargo:warning=PDFium {lib_name} prepared for {target} {profile}");
    Ok(())
}

fn download_and_extract_pdfium(
    target: &str,
    cache_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if env::var("PDF_TOOLBOX_SKIP_PDFIUM_DOWNLOAD").ok().as_deref() == Some("1") {
        return Err("PDF_TOOLBOX_SKIP_PDFIUM_DOWNLOAD=1".into());
    }

    let artifact = pdfium_artifact_name(target)?;
    let url = format!(
        "https://github.com/bblanchon/pdfium-binaries/releases/latest/download/{artifact}.tgz"
    );
    let archive_path = cache_dir.join(format!("{artifact}.tgz"));

    let response = ureq::get(&url).call()?;
    let mut reader = response.into_reader();
    let mut archive_file = fs::File::create(&archive_path)?;
    io::copy(&mut reader, &mut archive_file)?;

    let archive_file = fs::File::open(&archive_path)?;
    let decoder = flate2::read::GzDecoder::new(archive_file);
    let mut archive = tar::Archive::new(decoder);
    archive.unpack(cache_dir)?;
    Ok(())
}

fn pdfium_artifact_name(target: &str) -> Result<&'static str, Box<dyn std::error::Error>> {
    match target {
        "x86_64-pc-windows-msvc" | "x86_64-pc-windows-gnu" => Ok("pdfium-win-x64"),
        "aarch64-pc-windows-msvc" => Ok("pdfium-win-arm64"),
        "x86_64-apple-darwin" => Ok("pdfium-mac-x64"),
        "aarch64-apple-darwin" => Ok("pdfium-mac-arm64"),
        other => Err(format!("unsupported PDFium target: {other}").into()),
    }
}

fn platform_library_name(target: &str) -> Result<&'static str, Box<dyn std::error::Error>> {
    if target.contains("windows") {
        Ok("pdfium.dll")
    } else if target.contains("apple-darwin") {
        Ok("libpdfium.dylib")
    } else {
        Err(format!("unsupported PDFium library target: {target}").into())
    }
}

fn cargo_target_profile_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let profile = env::var("PROFILE")?;
    let mut current = out_dir.as_path();

    while let Some(parent) = current.parent() {
        if current.file_name().and_then(|name| name.to_str()) == Some(profile.as_str()) {
            return Ok(current.to_path_buf());
        }
        current = parent;
    }

    Err(format!("failed to find target/{profile} from OUT_DIR").into())
}

fn find_file(dir: &Path, name: &str) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_file() && path.file_name().and_then(|value| value.to_str()) == Some(name) {
            return Some(path);
        }
        if path.is_dir() {
            if let Some(found) = find_file(&path, name) {
                return Some(found);
            }
        }
    }
    None
}
