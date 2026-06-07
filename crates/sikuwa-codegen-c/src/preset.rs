//! CI / tooling presets for manifest verification.

use std::path::{Path, PathBuf};

pub const CI_GOLDEN_MANIFESTS: &str = "tests/golden/manifests";
pub const CI_PRESET_LIST: &str = "tests/golden/manifests/preset.txt";

/// Resolve repository root from CLI crate layout or cwd.
pub fn repo_root() -> PathBuf {
    let from_crate = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    if from_crate.join(CI_PRESET_LIST).is_file() {
        return from_crate;
    }
    PathBuf::from(".")
}

pub fn ci_golden_manifest(repo_root: &Path, py_stem: &str) -> PathBuf {
    repo_root
        .join(CI_GOLDEN_MANIFESTS)
        .join(format!("{py_stem}.skw.json"))
}

pub fn load_ci_preset_cases(repo_root: &Path) -> Result<Vec<PathBuf>, String> {
    let list = repo_root.join(CI_PRESET_LIST);
    let text = std::fs::read_to_string(&list)
        .map_err(|e| format!("cannot read {}: {e}", list.display()))?;
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let path = repo_root.join(line);
        if !path.is_file() {
            return Err(format!("preset fixture not found: {}", path.display()));
        }
        out.push(path);
    }
    if out.is_empty() {
        return Err(format!("no cases in {}", list.display()));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_list_exists() {
        let root = repo_root();
        let cases = load_ci_preset_cases(&root).expect("preset.txt");
        assert!(!cases.is_empty());
    }
}
