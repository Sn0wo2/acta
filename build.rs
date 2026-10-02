#![allow(clippy::expect_used)]

fn main() {
    let width = acta_build::walk_src_max_width("src", "src/");
    let out_dir = std::env::var_os("OUT_DIR").expect("Cargo should set OUT_DIR");
    let path = std::path::Path::new(&out_dir).join("path_width");
    if let Err(e) = std::fs::write(&path, width.to_string()) {
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
    let wasm_console = wasm && feature("wasm-console");
    let async_any = feature("custom-async") || feature("native-async");
    let wasm_blocking = wasm && (feature("file") || feature("compress") || async_any);

    println!("cargo::rustc-check-cfg=cfg(acta_wasm_console,acta_async,acta_wasm_blocking)");
    if wasm_console {
        println!("cargo::rustc-cfg=acta_wasm_console");
    }
    if async_any {
        println!("cargo::rustc-cfg=acta_async");
    }
    if wasm_blocking {
        println!("cargo::rustc-cfg=acta_wasm_blocking");
    }
}
