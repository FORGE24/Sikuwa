//! Link generated Sikuwa-C into `.so` / `.dll`.

use std::path::PathBuf;

use clap::Subcommand;
use sikuwa_core::Result;
use sikuwa_link::{default_shared_extension, find_sikuwa_include, link_shared, LinkOptions};

#[derive(Subcommand)]
pub enum LinkCommands {
    /// Compile `.c` (or directory) into a shared library
    Shared {
        /// `.c` file or directory with generated sources
        input: PathBuf,
        /// Output library path (default: `lib<name>.so|.dll`)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Extra include directory (repeatable)
        #[arg(short = 'I', long = "include")]
        include: Vec<PathBuf>,
        /// C compiler executable (default: MinGW gcc on Windows, else `$CC` / auto)
        #[arg(long)]
        cc: Option<String>,
        /// Do not link libsikuwa runtime sources
        #[arg(long)]
        no_runtime: bool,
        /// Do not link hotpath asm (c/src/hotpath only)
        #[arg(long)]
        no_hotpath: bool,
    },
}

pub fn run(cmd: LinkCommands) -> i32 {
    match run_inner(cmd) {
        Ok(path) => {
            println!("linked {}", path.display());
            0
        }
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}

fn run_inner(cmd: LinkCommands) -> Result<PathBuf> {
    match cmd {
        LinkCommands::Shared {
            input,
            output,
            include,
            cc,
            no_runtime,
            no_hotpath,
        } => {
            let output = output.unwrap_or_else(|| default_output_path(&input));
            if let Some(parent) = output.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)?;
                }
            }
            let mut include_dirs = include;
            if let Some(inc) = find_sikuwa_include(&input) {
                if !include_dirs.iter().any(|p| p == &inc) {
                    include_dirs.push(inc);
                }
            }
            link_shared(&LinkOptions {
                input: input.clone(),
                output: output.clone(),
                include_dirs,
                compiler: cc,
                link_runtime: !no_runtime,
                link_hotpath: !no_hotpath,
                extra_source_dirs: Vec::new(),
                library_dirs: Vec::new(),
                libraries: Vec::new(),
                extra_sources: Vec::new(),
            })?;
            Ok(output)
        }
    }
}

fn default_output_path(input: &PathBuf) -> PathBuf {
    let stem = if input.is_dir() {
        input
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("module")
    } else {
        input.file_stem().and_then(|s| s.to_str()).unwrap_or("module")
    };
    let ext = default_shared_extension();
    PathBuf::from(format!("lib{stem}.{ext}"))
}
