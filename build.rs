#![allow(clippy::expect_used)]

fn main() {
    let width = acta_build::walk_src_max_width("src", "src/");
    let out_dir = std::env::var_os("OUT_DIR").expect("Cargo should set OUT_DIR");
    let path = std::path::Path::new(&out_dir).join("path_width");
    if let Err(e) = std::fs::write(&path, width.to_string()) {
        println!("cargo::warning=failed to write {}: {e}", path.display());
    }
    println!("cargo::rerun-if-changed=src");
}
