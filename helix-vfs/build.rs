use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

fn main() {
    if env::var("CARGO_FEATURE_EMBED_RUNTIME").is_err() {
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let runtime_dir = manifest_dir.join("../runtime");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let out_file = out_dir.join("embedded_runtime.rs");

    let mut entries: Vec<(String, PathBuf)> = Vec::new();
    collect_files(&runtime_dir, &runtime_dir, &mut entries);

    let mut f = fs::File::create(&out_file).unwrap();
    writeln!(f, "static EMBEDDED_RUNTIME: &[(&str, &[u8])] = &[").unwrap();
    for (rel, abs) in &entries {
        // Use include_bytes! so the content is baked in at compile time
        writeln!(f, "    (\"{rel}\", include_bytes!(\"{}\")),", abs.display()).unwrap();
    }
    writeln!(f, "];").unwrap();

    println!("cargo:rerun-if-changed=../runtime");
}

fn collect_files(base: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();

        // Skip the entire grammars directory — grammar .wasm modules are
        // loaded individually at runtime from URLs, NOT embedded in the
        // main binary (embedding produced a ~300MB blob that browsers
        // couldn't handle).
        if path.ends_with("grammars") && path.parent().is_some_and(|p| p.ends_with("runtime")) {
            continue;
        }

        if path.is_dir() {
            collect_files(base, &path, out);
        } else if path.is_file() {
            let rel = path.strip_prefix(base).unwrap();
            out.push((rel.to_string_lossy().replace('\\', "/"), path.clone()));
        }
    }
}
