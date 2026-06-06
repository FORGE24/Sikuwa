use sikuwa_core::{Codename, VERSION};
use sikuwa_link::{detect_compiler, find_runtime_sources, find_sikuwa_include};

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
        } else {
            println!("[..] c/include — not found (run from repo root for link)");
        }
    }

    match detect_compiler() {
        Some(cc) => println!("[ok] C compiler: {cc}"),
        None => println!("[!!] C compiler not found — set CC or install gcc/clang"),
    }

    println!();
    println!("[..] Nuitka backend — optional (Plan 5+)");
    println!();
    println!("Try:");
    println!("  sikuwa codegen c tests/fixtures/add.py --out-dir .sikuwa/build/add");
    println!("  sikuwa link shared .sikuwa/build/add -o dist/libadd.so");
    0
}
