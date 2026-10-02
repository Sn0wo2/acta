#![allow(clippy::expect_used)]

fn main() {
    let path =
        std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo should set OUT_DIR"))
            .join("path_width");
    if let Err(e) = std::fs::write(
        &path,
        acta_build::walk_src_max_width("src", "src/").to_string(),
    ) {
        println!("cargo::warning=failed to write {}: {e}", path.display());
    }
    println!("cargo::rerun-if-changed=src");

    let feature = |name: &str| {
        std::env::var_os(format!(
            "CARGO_FEATURE_{}",
            name.to_uppercase().replace('-', "_")
        ))
        .is_some()
    };
    let wasm = std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32");
    let async_any = feature("custom-async") || feature("native-async");

    println!("cargo::rustc-check-cfg=cfg(acta_wasm_console,acta_async,acta_wasm_blocking)");
    if wasm && feature("wasm-console") {
        println!("cargo::rustc-cfg=acta_wasm_console");
    }
    if async_any {
        println!("cargo::rustc-cfg=acta_async");
    }
    if wasm && (feature("file") || feature("compress") || async_any) {
        println!("cargo::rustc-cfg=acta_wasm_blocking");
    }
}
