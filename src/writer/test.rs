use crate::Rotation;
use crate::writer::file::{build_file_layer, resolve_log_path, rotate_log_file};

#[cfg(feature = "file")]
#[test]
fn build_file_layer_creates_dirs() {
    let dir = std::env::temp_dir().join("acta-test-filelayer");
    drop(std::fs::remove_dir_all(&dir));
    let nested = dir.join("a").join("b");
    let log_path = nested.join("app.log");

    let result = build_file_layer(&log_path, Rotation::None);
    assert!(result.is_ok());

    let (_writer, _guard, path) = result.unwrap();
    assert!(path.parent().unwrap().exists());

    drop(std::fs::remove_dir_all(&dir));
}

#[cfg(feature = "file")]
#[test]
fn resolve_log_path_new_file() {
    let dir = std::env::temp_dir().join("acta-test-resolve");
    drop(std::fs::remove_dir_all(&dir));
    drop(std::fs::create_dir_all(&dir));
    let path = dir.join("new.log");

    let resolved = resolve_log_path(&path);
    assert_eq!(resolved, path);

    drop(std::fs::remove_dir_all(&dir));
}

#[cfg(feature = "file")]
#[test]
fn resolve_log_path_fallback_when_parent_is_file() {
    let dir = std::env::temp_dir().join("acta-test-resolve-fallback");
    drop(std::fs::remove_dir_all(&dir));
    drop(std::fs::create_dir_all(&dir));

    let bad_file = dir.join("existing_file");
    std::fs::write(&bad_file, b"contents").unwrap();

    let nested = bad_file.join("should_not_exist.log");
    let resolved = resolve_log_path(&nested);

    assert!(!resolved.exists());
    let pid = std::process::id();
    assert!(
        resolved
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .contains(&pid.to_string())
    );

    drop(std::fs::remove_dir_all(&dir));
}

#[cfg(feature = "file")]
#[test]
fn rotate_nonexistent_file_is_noop() {
    let path = std::env::temp_dir().join("acta-test-nonexistent.log");
    drop(std::fs::remove_file(&path));
    assert!(rotate_log_file(&path, Rotation::Rename).is_ok());
    #[cfg(feature = "compress")]
    assert!(rotate_log_file(&path, Rotation::Compress).is_ok());
    assert!(rotate_log_file(&path, Rotation::None).is_ok());
}

#[cfg(feature = "file")]
#[test]
fn rotate_none_keeps_file() {
    let dir = std::env::temp_dir().join("acta-test-rotate-none");
    drop(std::fs::create_dir_all(&dir));
    let path = dir.join("app.log");
    std::fs::write(&path, b"hello\n").unwrap();

    rotate_log_file(&path, Rotation::None).unwrap();
    assert!(path.exists());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello\n");
    drop(std::fs::remove_dir_all(&dir));
}

#[cfg(feature = "file")]
#[test]
fn rotate_rename() {
    let dir = std::env::temp_dir().join("acta-test-rotate-rename");
    drop(std::fs::remove_dir_all(&dir));
    drop(std::fs::create_dir_all(&dir));
    let path = dir.join("app.log");
    std::fs::write(&path, b"old content\n").unwrap();

    rotate_log_file(&path, Rotation::Rename).unwrap();
    assert!(!path.exists());

    let entries: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().collect();
    assert_eq!(entries.len(), 1);
    let content = std::fs::read_to_string(entries[0].path()).unwrap();
    assert_eq!(content, "old content\n");
    drop(std::fs::remove_dir_all(&dir));
}

#[test]
#[cfg(feature = "compress")]
fn rotate_compress() {
    let dir = std::env::temp_dir().join("acta-test-rotate-compress");
    drop(std::fs::remove_dir_all(&dir));
    drop(std::fs::create_dir_all(&dir));
    let path = dir.join("app.log");
    std::fs::write(&path, b"compress me\n").unwrap();

    rotate_log_file(&path, Rotation::Compress).unwrap();
    assert!(!path.exists());

    let mut entries = Vec::new();
    for _ in 0..100 {
        std::thread::sleep(std::time::Duration::from_millis(10));
        entries = std::fs::read_dir(&dir).unwrap().flatten().collect();
        if entries.len() == 1
            && entries[0]
                .path()
                .extension()
                .is_some_and(|ext| ext == "gz")
        {
            break;
        }
    }

    assert_eq!(entries.len(), 1);
    let gz_data = std::fs::read(entries[0].path()).unwrap();
    assert!(gz_data.len() > 2);
    assert_eq!(gz_data[0], 0x1f);
    assert_eq!(gz_data[1], 0x8b);
    drop(std::fs::remove_dir_all(&dir));
}
