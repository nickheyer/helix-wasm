use std::path::{Path, PathBuf};
use std::{env, fs};

fn main() {
    if env::var_os("DISABLED_TS_BUILD").is_some() {
        return;
    }

    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let vendor_src = manifest.join("vendor/src");
    let vendor_inc = manifest.join("vendor/include");
    let sysroot = manifest.join("../../patches/wasm-sysroot/include");

    for entry in fs::read_dir(&vendor_src).unwrap().flatten() {
        println!(
            "cargo:rerun-if-changed={}",
            vendor_src.join(entry.file_name()).display()
        );
    }

    let target = env::var("TARGET").unwrap_or_default();
    let is_wasm = target.starts_with("wasm32");

    let mut build = cc::Build::new();
    build
        .file(vendor_src.join("lib.c"))
        .include(&vendor_src)
        .include(&vendor_inc);

    // Provide C stdlib stubs (fputs, fputc) so tree-sitter's error/logging
    // paths resolve at link time without needing external wasm imports.
    if is_wasm {
        build.file(vendor_src.join("wasm_stubs.c"));
    }

    // The wasm sysroot provides libc stubs for wasm32 only.
    // On native the host compiler (set via HOST_CC in the Makefile)
    // knows its own include paths.
    if is_wasm {
        build.include(&sysroot);
    }

    build
        .std("c11")
        .flag_if_supported("-fvisibility=hidden")
        .flag_if_supported("-fno-builtin")
        .flag_if_supported("-Wshadow")
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-incompatible-pointer-types")
        .define("_POSIX_C_SOURCE", "200112L")
        .define("_DEFAULT_SOURCE", None)
        .define("NDEBUG", None)
        .warnings(false)
        .compile("tree-sitter");

    generate_empty_registry();
}

fn generate_empty_registry() {
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let code = "pub fn get_grammar(_name: &str) -> Option<super::Grammar> { None }\n";
    fs::write(out.join("grammar_registry.rs"), code).unwrap();
}
