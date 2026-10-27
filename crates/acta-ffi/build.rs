//! Regenerates `include/acta_ffi.h` from this crate's C ABI surface.
//! The header is committed; cbindgen only rewrites it when it changes.

fn main() {
    let crate_dir = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set"),
    );
    let config =
        cbindgen::Config::from_file(crate_dir.join("cbindgen.toml")).expect("valid cbindgen.toml");
    cbindgen::Builder::new()
        .with_crate(crate_dir.to_string_lossy().as_ref())
        .with_config(config)
        .generate()
        .expect("cbindgen generation succeeds")
        .write_to_file(crate_dir.join("include/acta_ffi.h"));
}
