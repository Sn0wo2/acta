//! Build-time helper for [acta](https://crates.io/crates/acta).
//!
//! Call from your `build.rs` to compute a sensible default `path_width` for
//! [`acta::Formatter`] based on the longest source-file path in your project.
//!
//! # Example
//!
//! Add to your `Cargo.toml`:
//!
//! ```toml
//! [build-dependencies]
//! acta-build = "0.1"
//! ```
//!
//! In `build.rs`:
//!
//! ```no_run
//! # fn example() {
//! let width = acta_build::walk_src_max_width("src", "src/");
//! println!("cargo:rustc-env=ACTA_PATH_WIDTH={width}");
//! println!("cargo:rerun-if-changed=src");
//! # }
//! ```
//!
//! Then in your code:
//!
//! ```ignore
//! let width: usize = env!("ACTA_PATH_WIDTH").parse().unwrap_or(40);
//! let formatter = acta::Formatter::new().with_path_width(width);
//! ```

use std::path::Path;
use walkdir::WalkDir;

const FALLBACK_WIDTH: usize = 40;
const PADDING: usize = 4;

/// Walk `dir` recursively, find every `.rs` file, and return
/// `max(path_len_after_stripping(strip_prefix)) + 4`.
///
/// Returns [`FALLBACK_WIDTH`] (40) if `dir` does not exist or contains no
/// `.rs` files — safe to call unconditionally from a `build.rs`.
#[must_use]
pub fn walk_src_max_width(dir: impl AsRef<Path>, strip_prefix: &str) -> usize {
    let entries: Vec<_> = WalkDir::new(dir.as_ref())
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "rs"))
        .collect();

    if entries.is_empty() {
        return FALLBACK_WIDTH;
    }

    let max = entries
        .iter()
        .map(|e| {
            let display = e.path().to_string_lossy().replace('\\', "/");
            display
                .find(strip_prefix)
                .map_or(display.len(), |i| display[i + strip_prefix.len()..].len())
        })
        .max()
        .unwrap_or(FALLBACK_WIDTH);

    max + PADDING
}
