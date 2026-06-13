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
    /// Link `c/src/hotpath/*.c` + `asm/x86_64/` on x86_64 hosts
    pub link_hotpath: bool,
    /// Additional directories whose `.c` files are compiled into the same DSO
    pub extra_source_dirs: Vec<PathBuf>,
    /// `-L` directories for `-l` when linking against dependency DSOs
    pub library_dirs: Vec<PathBuf>,
    /// Library names without `lib` prefix (e.g. `add` → `-ladd`)
    pub libraries: Vec<String>,
    /// Extra `.c` files (e.g. generated `main`) linked into the same binary
    pub extra_sources: Vec<PathBuf>,
}

impl Default for LinkOptions {
    fn default() -> Self {
        Self {
            input: PathBuf::new(),
            output: PathBuf::new(),
            include_dirs: Vec::new(),
            compiler: None,
            link_runtime: true,
            link_hotpath: true,
            extra_source_dirs: Vec::new(),
            library_dirs: Vec::new(),
            libraries: Vec::new(),
            extra_sources: Vec::new(),
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

pub fn find_hotpath_sources(start: &Path) -> Vec<PathBuf> {
    let Some(root) = find_sikuwa_root(start) else {
        return Vec::new();
    };
    let hp = root.join("c").join("src").join("hotpath");
    if !hp.is_dir() {
        return Vec::new();
    }
    let mut sources = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&hp) {
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

/// ASM directory name for the current host (`linux`, `win-gnu`, or `win` with `SKW_USE_MSVC=1`).
pub fn asm_subdir() -> &'static str {
    if cfg!(windows) {
        if std::env::var("SKW_USE_MSVC").ok().as_deref() == Some("1") {
            "win"
        } else {
            "win-gnu"
        }
    } else {
        "linux"
    }
}

/// True when `compiler` is MinGW / mingw-w64 gcc.
pub fn is_mingw_compiler(compiler: &str) -> bool {
    let lower = compiler.to_ascii_lowercase();
    if lower.contains("mingw") || lower.contains("w64-mingw32") {
        return true;
    }
    let output = Command::new(compiler).args(["-dumpmachine"]).output();
    match output {
        Ok(out) if out.status.success() => {
            let triple = String::from_utf8_lossy(&out.stdout).to_ascii_lowercase();
            triple.contains("mingw") || triple.contains("w64")
        }
        _ => false,
    }
}

fn mingw_gcc_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(msys) = std::env::var("MSYS2") {
        paths.push(PathBuf::from(&msys).join("mingw64").join("bin").join("gcc.exe"));
        paths.push(PathBuf::from(&msys).join("ucrt64").join("bin").join("gcc.exe"));
    }
    for prefix in [
        r"C:\msys64",
        r"C:\tools\msys64",
        r"C:\Program Files\msys64",
    ] {
        let base = PathBuf::from(prefix);
        paths.push(base.join("mingw64").join("bin").join("gcc.exe"));
        paths.push(base.join("ucrt64").join("bin").join("gcc.exe"));
    }
    paths
}

/// Prefer MinGW gcc on Windows (MSYS2 / mingw-w64).
pub fn detect_mingw_gcc() -> Option<String> {
    if let Ok(cc) = std::env::var("CC") {
        if !cc.is_empty() && is_mingw_compiler(&cc) {
            return Some(cc);
        }
    }
    for name in ["gcc", "x86_64-w64-mingw32-gcc"] {
        if command_exists(name) && is_mingw_compiler(name) {
            return Some(name.to_string());
        }
    }
    for path in mingw_gcc_candidates() {
        if path.is_file() {
            let display = path.to_string_lossy().into_owned();
            if is_mingw_compiler(&display) {
                return Some(display);
            }
        }
    }
    None
}

/// x86_64 assembly sources for the current host OS.
pub fn find_asm_sources(start: &Path) -> Vec<PathBuf> {
    if std::env::consts::ARCH != "x86_64" {
        return Vec::new();
    }
    let Some(root) = find_sikuwa_root(start) else {
        return Vec::new();
    };
    let subdir = asm_subdir();
    let asm_dir = root.join("asm").join("x86_64").join(subdir);
    if !asm_dir.is_dir() {
        return Vec::new();
    }
    let ext = if subdir == "win" { "asm" } else { "S" };
    let mut sources = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&asm_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some(ext) {
                sources.push(path);
            }
        }
    }
    sources.sort();
    sources
}

pub fn uses_msvc_asm(start: &Path) -> bool {
    cfg!(windows)
        && std::env::var("SKW_USE_MSVC").ok().as_deref() == Some("1")
        && command_exists("ml64")
        && find_sikuwa_root(start)
            .map(|root| root.join("asm").join("x86_64").join("win").is_dir())
            .unwrap_or(false)
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
    link_artifact(opts, true)
}

/// Link generated sources into a standalone executable (no `-shared`).
pub fn link_executable(opts: &LinkOptions) -> Result<()> {
    link_artifact(opts, false)
}

fn link_artifact(opts: &LinkOptions, shared: bool) -> Result<()> {
    let mut sources = collect_c_sources(&opts.input)?;
    for dir in &opts.extra_source_dirs {
        sources.extend(collect_c_sources(dir)?);
    }
    sources.sort();
    sources.dedup();
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

    let mut extra_c = Vec::new();
    let mut asm = Vec::new();
    let mut use_asm = false;
    if opts.link_hotpath {
        extra_c.extend(find_hotpath_sources(&opts.input));
        asm = find_asm_sources(&opts.input);
        use_asm = !asm.is_empty();
    }
    if opts.link_runtime {
        extra_c.extend(find_runtime_sources(&opts.input));
    }

    let base = compiler_base_name(&compiler);
    let msvc_asm = base == "cl" && uses_msvc_asm(&opts.input) && use_asm;
    if use_asm && base == "cl" && !msvc_asm {
        use_asm = false;
    }

    if msvc_asm {
        let win_asm = find_msvc_asm_sources(&opts.input);
        link_shared_msvc(
            &compiler,
            &include_base,
            opts,
            &sources,
            &extra_c,
            &win_asm,
            use_asm,
            shared,
        )
    } else {
        link_gcc(
            &compiler,
            &include_base,
            opts,
            &sources,
            &extra_c,
            &asm,
            use_asm,
            shared,
        )
    }
}

fn find_msvc_asm_sources(start: &Path) -> Vec<PathBuf> {
    let Some(root) = find_sikuwa_root(start) else {
        return Vec::new();
    };
    let asm_dir = root.join("asm").join("x86_64").join("win");
    if !asm_dir.is_dir() {
        return Vec::new();
    }
    let mut sources = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&asm_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("asm") {
                sources.push(path);
            }
        }
    }
    sources.sort();
    sources
}

fn link_gcc(
    compiler: &str,
    include_base: &Path,
    opts: &LinkOptions,
    sources: &[PathBuf],
    extra_c: &[PathBuf],
    asm: &[PathBuf],
    use_asm: bool,
    shared: bool,
) -> Result<()> {
    let mut args = build_compile_args(compiler, shared);
    args.push("-DSKW_BUILDING_MODULE".to_string());
    if use_asm {
        args.push("-DSKW_HOTPATH_ASM".to_string());
    }
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
    for src in sources
        .iter()
        .chain(extra_c.iter())
        .chain(opts.extra_sources.iter())
    {
        args.push(src.to_string_lossy().into_owned());
    }
    for src in asm {
        args.push(src.to_string_lossy().into_owned());
    }
    for dir in &opts.library_dirs {
        args.push(format!("-L{}", dir.display()));
    }
    for lib in &opts.libraries {
        args.push(format!("-l{lib}"));
    }
    run_compiler(compiler, &args)
}

fn link_shared_msvc(
    compiler: &str,
    include_base: &Path,
    opts: &LinkOptions,
    sources: &[PathBuf],
    extra_c: &[PathBuf],
    asm: &[PathBuf],
    use_asm: bool,
    shared: bool,
) -> Result<()> {
    let out_dir = opts
        .output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let obj_dir = out_dir.join(".sikuwa_link_obj");
    std::fs::create_dir_all(&obj_dir).map_err(SikuwaError::from)?;

    let mut objects = Vec::new();
    let mut defines = vec!["/DSKW_BUILDING_MODULE".to_string()];
    if use_asm {
        defines.push("/DSKW_HOTPATH_ASM".to_string());
    }
    let include_arg = format!("/I{}", include_base.display());

    for (i, src) in sources
        .iter()
        .chain(extra_c.iter())
        .chain(opts.extra_sources.iter())
        .enumerate()
    {
        let obj = obj_dir.join(format!("src{i}.obj"));
        let mut args = vec![
            "/nologo".to_string(),
            "/W3".to_string(),
            "/O2".to_string(),
            "/c".to_string(),
            include_arg.clone(),
        ];
        args.extend(defines.iter().cloned());
        for inc in &opts.include_dirs {
            args.push(format!("/I{}", inc.display()));
        }
        args.push(format!("/Fo{}", obj.display()));
        args.push(src.to_string_lossy().into_owned());
        run_compiler(compiler, &args)?;
        objects.push(obj);
    }

    if use_asm {
        for (i, src) in asm.iter().enumerate() {
            let obj = obj_dir.join(format!("asm{i}.obj"));
            let args = vec![
                "/nologo".to_string(),
                "/c".to_string(),
                format!("/Fo{}", obj.display()),
                src.to_string_lossy().into_owned(),
            ];
            run_tool("ml64", &args)?;
            objects.push(obj);
        }
    }

    let mut link_args = vec!["/nologo".to_string()];
    if shared {
        link_args.push("/LD".to_string());
    }
    link_args.push(format!("/Fe:{}", opts.output.display()));
    for obj in &objects {
        link_args.push(obj.to_string_lossy().into_owned());
    }
    run_tool("link", &link_args)
}

fn run_tool(tool: &str, args: &[String]) -> Result<()> {
    let output = Command::new(tool)
        .args(args)
        .output()
        .map_err(|e| SikuwaError::pir(format!("failed to run {tool}: {e}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(SikuwaError::pir(format!(
            "{tool} failed:\n{stdout}{stderr}"
        )));
    }
    Ok(())
}

pub fn detect_compiler() -> Option<String> {
    if let Ok(cc) = std::env::var("CC") {
        if !cc.is_empty() {
            return Some(cc);
        }
    }
    if cfg!(windows) {
        if let Some(gcc) = detect_mingw_gcc() {
            return Some(gcc);
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

fn build_compile_args(compiler: &str, shared: bool) -> Vec<String> {
    let base = compiler_base_name(compiler);
    if base == "cl" {
        let mut args = vec!["/nologo".to_string(), "/W3".to_string()];
        if shared {
            args.push("/LD".to_string());
        }
        args
    } else {
        let mut args = vec!["-O2".to_string()];
        if shared {
            args.push("-shared".to_string());
        }
        if cfg!(not(windows)) {
            args.push("-fPIC".to_string());
            if shared {
                args.push("-fvisibility=hidden".to_string());
            }
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

    #[test]
    fn find_hotpath_from_repo() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..");
        assert!(!find_hotpath_sources(&root).is_empty());
    }

    #[test]
    fn asm_subdir_linux_or_mingw() {
        let sub = asm_subdir();
        if cfg!(windows) {
            assert_eq!(sub, "win-gnu");
        } else {
            assert_eq!(sub, "linux");
        }
    }

    #[test]
    fn find_asm_on_x86_64() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..");
        if std::env::consts::ARCH == "x86_64" {
            assert!(!find_asm_sources(&root).is_empty());
        }
    }
}
