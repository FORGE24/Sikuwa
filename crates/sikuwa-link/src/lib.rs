//! Compile generated Sikuwa-C into a shared library.

use std::path::{Path, PathBuf};
use std::process::Command;

use sikuwa_core::{Result, SikuwaError};

#[derive(Debug, Clone)]
pub struct LinkOptions {
    /// `.c` file or directory containing `.c` sources
    pub input: PathBuf,
    pub output: PathBuf,
    /// Extra `-I` directories (in addition to auto-detected `c/include`)
    pub include_dirs: Vec<PathBuf>,
    pub compiler: Option<String>,
    /// Link `c/src/runtime/*.c` (S3 helpers)
    pub link_runtime: bool,
}

impl Default for LinkOptions {
    fn default() -> Self {
        Self {
            input: PathBuf::new(),
            output: PathBuf::new(),
            include_dirs: Vec::new(),
            compiler: None,
            link_runtime: true,
        }
    }
}

/// Walk upward from `start` to find `c/include`.
pub fn find_sikuwa_include(start: &Path) -> Option<PathBuf> {
    let mut dir = if start.is_dir() {
        start.to_path_buf()
    } else {
        start.parent()?.to_path_buf()
    };
    loop {
        let inc = dir.join("c").join("include");
        if inc.is_dir() {
            return Some(inc);
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

/// Walk upward from `start` to find repo root (directory containing `c/include`).
pub fn find_sikuwa_root(start: &Path) -> Option<PathBuf> {
    find_sikuwa_include(start).and_then(|inc| inc.parent()?.parent().map(|p| p.to_path_buf()))
}

pub fn find_runtime_sources(start: &Path) -> Vec<PathBuf> {
    let Some(root) = find_sikuwa_root(start) else {
        return Vec::new();
    };
    let rt = root.join("c").join("src").join("runtime");
    if !rt.is_dir() {
        return Vec::new();
    }
    let mut sources = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&rt) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("c") {
                sources.push(path);
            }
        }
    }
    sources.sort();
    sources
}

pub fn collect_c_sources(input: &Path) -> Result<Vec<PathBuf>> {
    if input.is_file() {
        if input.extension().and_then(|e| e.to_str()) != Some("c") {
            return Err(SikuwaError::pir(format!(
                "not a .c file: {}",
                input.display()
            )));
        }
        return Ok(vec![input.to_path_buf()]);
    }
    if !input.is_dir() {
        return Err(SikuwaError::pir(format!("not found: {}", input.display())));
    }
    let mut sources = Vec::new();
    for entry in std::fs::read_dir(input).map_err(SikuwaError::from)? {
        let entry = entry.map_err(SikuwaError::from)?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("c") {
            sources.push(path);
        }
    }
    sources.sort();
    if sources.is_empty() {
        return Err(SikuwaError::pir(format!(
            "no .c files in {}",
            input.display()
        )));
    }
    Ok(sources)
}

pub fn default_shared_extension() -> &'static str {
    if cfg!(windows) {
        "dll"
    } else if cfg!(target_os = "macos") {
        "dylib"
    } else {
        "so"
    }
}

pub fn link_shared(opts: &LinkOptions) -> Result<()> {
    let sources = collect_c_sources(&opts.input)?;
    let include_base = find_sikuwa_include(&opts.input).ok_or_else(|| {
        SikuwaError::pir(
            "could not find c/include — run from Sikuwa repo or set -I manually",
        )
    })?;

    let compiler = opts
        .compiler
        .clone()
        .or_else(detect_compiler)
        .ok_or_else(|| SikuwaError::pir("no C compiler found (set CC or install gcc/clang)"))?;

    let mut args = build_compile_args(&compiler);
    args.push("-DSKW_BUILDING_MODULE".to_string());
    args.push(format!("-I{}", include_base.display()));
    for inc in &opts.include_dirs {
        args.push(format!("-I{}", inc.display()));
    }
    if opts.input.is_dir() {
        args.push(format!("-I{}", opts.input.display()));
    } else if let Some(parent) = opts.input.parent() {
        args.push(format!("-I{}", parent.display()));
    }
    args.push("-o".to_string());
    args.push(opts.output.to_string_lossy().into_owned());
    for src in &sources {
        args.push(src.to_string_lossy().into_owned());
    }
    if opts.link_runtime {
        for rt in find_runtime_sources(&opts.input) {
            args.push(rt.to_string_lossy().into_owned());
        }
    }

    run_compiler(&compiler, &args)
}

pub fn detect_compiler() -> Option<String> {
    if let Ok(cc) = std::env::var("CC") {
        if !cc.is_empty() {
            return Some(cc);
        }
    }
    for c in &["gcc", "clang", "cc", "cl"] {
        if command_exists(c) {
            return Some(c.to_string());
        }
    }
    None
}

fn command_exists(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn build_compile_args(compiler: &str) -> Vec<String> {
    let base = compiler_base_name(compiler);
    if base == "cl" {
        vec![
            "/LD".to_string(),
            "/nologo".to_string(),
            "/W3".to_string(),
        ]
    } else {
        let mut args = vec!["-shared".to_string(), "-fPIC".to_string(), "-O2".to_string()];
        if cfg!(windows) {
            // MinGW gcc on Windows
        }
        if cfg!(not(windows)) {
            args.push("-fvisibility=hidden".to_string());
        }
        args
    }
}

fn compiler_base_name(compiler: &str) -> String {
    Path::new(compiler)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(compiler)
        .to_ascii_lowercase()
}

fn run_compiler(compiler: &str, args: &[String]) -> Result<()> {
    let output = Command::new(compiler)
        .args(args)
        .output()
        .map_err(|e| SikuwaError::pir(format!("failed to run {compiler}: {e}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(SikuwaError::pir(format!(
            "{compiler} failed:\n{stdout}{stderr}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_include_from_repo() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..");
        assert!(find_sikuwa_include(&root).is_some());
    }

    #[test]
    fn find_runtime_from_repo() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..");
        assert!(!find_runtime_sources(&root).is_empty());
    }
}
