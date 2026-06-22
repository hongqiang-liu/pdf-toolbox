use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::core::{
    export_images, extract_text, merge_pdfs, split_pdf, ImageExportOptions, ImageFormat,
    MergeItem, MergeOptions, Result, SplitMode, SplitOptions, TextExtractOptions,
};
use crate::utils::log::cli_progress;

#[derive(Debug, Parser)]
#[command(name = "pdf_toolbox")]
#[command(version, about = "Cross-platform PDF split, merge, text and image toolbox")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Split a PDF by page ranges, fixed-size chunks, or a single page.
    Split(SplitArgs),
    /// Merge multiple PDFs into one PDF.
    Merge(MergeArgs),
    /// Extract standard embedded PDF text into a .txt file.
    Text(TextArgs),
    /// Render PDF pages to PNG or JPG images.
    Img(ImageArgs),
}

#[derive(Debug, Args)]
struct SplitArgs {
    input: PathBuf,

    /// Page range expression, for example: 1-5,7-12
    #[arg(long, conflicts_with_all = ["every", "page"])]
    range: Option<String>,

    /// Split into one PDF per N pages.
    #[arg(long, conflicts_with_all = ["range", "page"])]
    every: Option<u32>,

    /// Extract one page as a standalone PDF.
    #[arg(long, conflicts_with_all = ["range", "every"])]
    page: Option<u32>,

    /// Output directory.
    #[arg(short, long)]
    output: PathBuf,
}

#[derive(Debug, Args)]
struct MergeArgs {
    /// Input PDF files. Their CLI order is the output order.
    inputs: Vec<PathBuf>,

    /// Output PDF path.
    #[arg(short, long)]
    output: PathBuf,

    /// Insert this many A4 blank pages at the end.
    #[arg(long, default_value_t = 0)]
    blank_pages: u32,
}

#[derive(Debug, Args)]
struct TextArgs {
    input: PathBuf,

    /// Output .txt path.
    #[arg(short, long)]
    output: PathBuf,

    /// Add "----- Page N -----" markers before each page.
    #[arg(long)]
    page_markers: bool,
}

#[derive(Debug, Args)]
struct ImageArgs {
    input: PathBuf,

    /// Output directory. Defaults to ./images next to the current shell.
    #[arg(short, long, default_value = "./images")]
    output: PathBuf,

    /// Render DPI.
    #[arg(long, default_value_t = 300)]
    dpi: u16,

    /// Image format.
    #[arg(long, value_enum, default_value_t = CliImageFormat::Png)]
    format: CliImageFormat,

    /// Optional page list, for example: 1-3,9
    #[arg(long)]
    pages: Option<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliImageFormat {
    Png,
    Jpg,
}

impl From<CliImageFormat> for ImageFormat {
    fn from(value: CliImageFormat) -> Self {
        match value {
            CliImageFormat::Png => Self::Png,
            CliImageFormat::Jpg => Self::Jpg,
        }
    }
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let mut progress = cli_progress;

    match cli.command {
        Command::Split(args) => {
            let mode = if let Some(ranges) = args.range {
                SplitMode::Ranges { ranges }
            } else if let Some(pages_per_file) = args.every {
                SplitMode::Every { pages_per_file }
            } else if let Some(page) = args.page {
                SplitMode::Single { page }
            } else {
                return Err(crate::core::PdfToolboxError::InvalidArgument(
                    "split requires --range, --every, or --page".to_string(),
                ));
            };

            let outputs = split_pdf(
                SplitOptions {
                    input: args.input,
                    output_dir: args.output,
                    mode,
                },
                Some(&mut progress),
            )?;
            println!("created {} PDF file(s)", outputs.len());
        }
        Command::Merge(args) => {
            let mut items = args
                .inputs
                .into_iter()
                .map(|path| MergeItem::Pdf { path })
                .collect::<Vec<_>>();
            for _ in 0..args.blank_pages {
                items.push(MergeItem::Blank {
                    width: 595.0,
                    height: 842.0,
                });
            }
            let output = merge_pdfs(
                MergeOptions {
                    items,
                    output: args.output,
                },
                Some(&mut progress),
            )?;
            println!("created {}", output.display());
        }
        Command::Text(args) => {
            let output = extract_text(
                TextExtractOptions {
                    input: args.input,
                    output: args.output,
                    with_page_markers: args.page_markers,
                },
                Some(&mut progress),
            )?;
            println!("created {}", output.display());
        }
        Command::Img(args) => {
            let outputs = export_images(
                ImageExportOptions {
                    input: args.input,
                    output_dir: args.output,
                    dpi: args.dpi,
                    format: args.format.into(),
                    pages: args.pages,
                },
                Some(&mut progress),
            )?;
            println!("created {} image file(s)", outputs.len());
        }
    }

    Ok(())
}

