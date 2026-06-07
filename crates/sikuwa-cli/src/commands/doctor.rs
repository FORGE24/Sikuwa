use sikuwa_core::{Codename, VERSION};
use sikuwa_link::{
    asm_subdir, detect_compiler, detect_mingw_gcc, find_asm_sources, find_hotpath_sources,
    find_runtime_sources, find_sikuwa_include, is_mingw_compiler,
};

pub fn run() -> i32 {
    println!("Sikuwa Doctor — Ver.A2 Plan 4");
    println!("────────────────────────────────");
    println!("sikuwa:   {} ({})", VERSION, Codename::NAME);
    println!("engine:   {}", Codename::ENGINE);
    println!("rustc:    {}", option_env!("SIKUWA_RUSTC").unwrap_or("(build-time unknown)"));
    println!();

    println!("[ok] Rust CLI + sikuwa-pir / pystat / codegen-c / link");
    println!("[ok] PythonIR lowering + PyStat PGTE/ITR");
    println!("[ok] Sikuwa-C codegen + .skw.json manifest");

    if let Ok(cwd) = std::env::current_dir() {
        if let Some(inc) = find_sikuwa_include(&cwd) {
            println!("[ok] c/include at {}", inc.display());
            let rt = find_runtime_sources(&cwd);
            if rt.is_empty() {
                println!("[..] c/src/runtime — not found from cwd");
            } else {
                println!("[ok] runtime sources: {}", rt.len());
            }
            let hp = find_hotpath_sources(&cwd);
            if !hp.is_empty() {
                println!("[ok] hotpath sources: {}", hp.len());
            }
            let asm = find_asm_sources(&cwd);
            if asm.is_empty() {
                println!("[..] asm/x86_64/{} — none (non-x86_64 host?)", asm_subdir());
            } else {
                println!(
                    "[ok] asm/x86_64/{}: {} file(s) (MinGW GAS on Windows)",
                    asm_subdir(),
                    asm.len()
                );
            }
        } else {
            println!("[..] c/include — not found (run from repo root for link)");
        }
    }

    if cfg!(windows) {
        match detect_mingw_gcc() {
            Some(gcc) => println!("[ok] MinGW gcc: {gcc}"),
            None => println!(
                "[!!] MinGW gcc not found — install MSYS2 (mingw-w64) or set CC=.../gcc.exe"
            ),
        }
        if std::env::var("SKW_USE_MSVC").ok().as_deref() == Some("1") {
            println!("[..] SKW_USE_MSVC=1 — MSVC/ml64 asm path enabled");
        } else {
            println!("[ok] Windows default: MinGW + asm/x86_64/win-gnu (set SKW_USE_MSVC=1 for ml64)");
        }
    }

    match detect_compiler() {
        Some(cc) => {
            let kind = if is_mingw_compiler(&cc) {
                "MinGW"
            } else if cc.eq_ignore_ascii_case("cl") {
                "MSVC"
            } else {
                "C"
            };
            println!("[ok] link compiler ({kind}): {cc}");
        }
        None => println!("[!!] C compiler not found — set CC or install MinGW gcc"),
    }

    println!();
    println!("[..] Nuitka backend — optional (Plan 5+)");
    println!();
    println!("Try:");
    println!("  sikuwa codegen c tests/fixtures/add.py --out-dir .sikuwa/build/add");
    if cfg!(windows) {
        println!("  sikuwa link shared .sikuwa/build/add -o dist/libadd.dll");
        println!("  powershell -File scripts/asm-smoke.ps1");
    } else {
        println!("  sikuwa link shared .sikuwa/build/add -o dist/libadd.so");
        println!("  bash scripts/asm-smoke.sh");
    }
    0
}
