//! Capture library, ported from `src-tauri/src/library.rs`.
//!
//! Same on-disk format (JSON index + per-capture PNG files, atomic writes) so
//! both the Tauri app and this build share one library. Unlike the Tauri wire
//! API, the list call returns metadata only and images are read on demand —
//! no base64 dumps.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageSize {
    pub width: u32,
    pub height: u32,
}

/// Library metadata for one capture, without image bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureMetadata {
    pub id: String,
    pub title: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub image: ImageSize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IndexEntry {
    id: String,
    title: String,
    created_at: i64,
    updated_at: i64,
    image: ImageSize,
}

const INDEX_FILE: &str = "index.json";

pub fn app_data_root() -> Result<PathBuf, String> {
    let base = dirs::data_local_dir()
        .ok_or_else(|| "Windows did not provide a local application-data directory.".to_string())?;
    resolve_data_root(&base)
}

fn resolve_data_root(base: &Path) -> Result<PathBuf, String> {
    let root = base.join("Snipsy");
    let legacy = base.join("CleanShotW");
    if !root.exists() && legacy.is_dir() {
        fs::rename(&legacy, &root)
            .map_err(|error| format!("Could not migrate the legacy capture library: {error}"))?;
    }
    Ok(root)
}

fn library_root() -> Result<PathBuf, String> {
    Ok(app_data_root()?.join("library"))
}

pub(crate) fn validate_id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return Err("The capture id is invalid.".to_string());
    }
    Ok(())
}

fn validate_size(size: &ImageSize) -> Result<(), String> {
    if size.width == 0 || size.height == 0 || size.width > 100_000 || size.height > 100_000 {
        return Err("The capture dimensions are invalid.".to_string());
    }
    Ok(())
}

fn validate_title(title: &str) -> Result<(), String> {
    if title.trim().is_empty() {
        return Err("The capture title cannot be empty.".to_string());
    }
    if title.chars().count() > 500 {
        return Err("The capture title is too long.".to_string());
    }
    Ok(())
}

fn validate_index_entry(entry: &IndexEntry) -> Result<(), String> {
    validate_id(&entry.id)?;
    validate_size(&entry.image)?;
    validate_title(&entry.title)
}

/// Normalize records written by older versions before strict validation.
///
/// Older builds allowed whitespace-only titles. Keep those captures usable by
/// assigning a stable fallback title instead of rejecting the whole index.
fn normalize_index_entry(entry: &mut IndexEntry) -> Result<(), String> {
    let title = entry.title.trim();
    entry.title = if title.is_empty() {
        format!("Capture {}", entry.id)
    } else {
        title.to_string()
    };
    validate_index_entry(entry)
}

fn read_index(root: &Path) -> Result<Vec<IndexEntry>, String> {
    let path = root.join(INDEX_FILE);
    let mut entries = match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|error| format!("Could not read the library index: {error}"))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("Could not read the library index: {error}")),
    };
    for entry in &mut entries {
        normalize_index_entry(entry)?;
    }
    Ok(entries)
}

pub(crate) fn write_json<T: Serialize + ?Sized>(path: &Path, value: &T) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("Could not encode {}: {error}", path.display()))?;
    write_atomic(path, &bytes)
}

pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temp = path.with_extension("tmp");
    fs::write(&temp, bytes)
        .map_err(|error| format!("Could not write {}: {error}", path.display()))?;
    fs::rename(&temp, path)
        .map_err(|error| format!("Could not finish writing {}: {error}", path.display()))
}

fn write_index(root: &Path, entries: &[IndexEntry]) -> Result<(), String> {
    write_json(&root.join(INDEX_FILE), entries)
}

fn capture_dir(root: &Path, id: &str) -> PathBuf {
    root.join(id)
}

/// Save a captured PNG into the library and return its metadata.
pub fn save_capture(
    png: &[u8],
    width: u32,
    height: u32,
    title: &str,
) -> Result<CaptureMetadata, String> {
    let id = generate_id();
    let metadata = CaptureMetadata {
        id: id.clone(),
        title: title.trim().to_string(),
        created_at: now_millis(),
        updated_at: now_millis(),
        image: ImageSize { width, height },
    };
    validate_id(&metadata.id)?;
    validate_size(&metadata.image)?;
    validate_title(&metadata.title)?;

    let root = library_root()?;
    fs::create_dir_all(&root).map_err(|error| format!("Could not create the library: {error}"))?;
    let dir = capture_dir(&root, &id);
    fs::create_dir_all(&dir)
        .map_err(|error| format!("Could not create capture folder: {error}"))?;
    write_atomic(&dir.join("image.png"), png)?;

    let mut entries = read_index(&root)?;
    entries.push(IndexEntry {
        id: metadata.id.clone(),
        title: metadata.title.clone(),
        created_at: metadata.created_at,
        updated_at: metadata.updated_at,
        image: metadata.image.clone(),
    });
    write_index(&root, &entries)?;
    Ok(metadata)
}

/// Metadata for every capture, newest first.
#[cfg_attr(not(windows), allow(dead_code))] // consumed by the tray "Open" path
pub fn list_captures() -> Result<Vec<CaptureMetadata>, String> {
    let root = library_root()?;
    let mut entries = read_index(&root)?;
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.updated_at));
    Ok(entries
        .into_iter()
        .map(|entry| CaptureMetadata {
            id: entry.id,
            title: entry.title,
            created_at: entry.created_at,
            updated_at: entry.updated_at,
            image: entry.image,
        })
        .collect())
}

/// Read one capture's PNG bytes from disk, on demand.
#[cfg_attr(not(windows), allow(dead_code))] // consumed by the tray "Open" path
pub fn read_image_png(id: &str) -> Result<Vec<u8>, String> {
    validate_id(id)?;
    let root = library_root()?;
    let entries = read_index(&root)?;
    if !entries.iter().any(|entry| entry.id == id) {
        return Err("No such capture in the library.".to_string());
    }
    fs::read(capture_dir(&root, id).join("image.png"))
        .map_err(|error| format!("Could not read capture {id}: {error}"))
}

#[allow(dead_code)] // library management API for the editor phase
pub fn update_title(id: &str, title: &str) -> Result<bool, String> {
    validate_id(id)?;
    validate_title(title)?;
    let root = library_root()?;
    let mut entries = read_index(&root)?;
    let Some(entry) = entries.iter_mut().find(|entry| entry.id == id) else {
        return Ok(false);
    };
    entry.title = title.trim().to_string();
    entry.updated_at = now_millis();
    write_index(&root, &entries)?;
    Ok(true)
}

pub fn delete_capture(id: &str) -> Result<(), String> {
    validate_id(id)?;
    let root = library_root()?;
    delete_capture_from_root(&root, id)
}

fn delete_capture_from_root(root: &Path, id: &str) -> Result<(), String> {
    validate_id(id)?;
    let mut entries = read_index(root)?;
    let before = entries.len();
    entries.retain(|entry| entry.id != id);
    if entries.len() == before {
        return Ok(());
    }
    write_index(root, &entries)?;
    let dir = capture_dir(root, id);
    if dir.exists() {
        fs::remove_dir_all(&dir).map_err(|error| {
            format!(
                "Capture {id} was removed from the library, but its files could not be deleted: {error}"
            )
        })?;
    }
    Ok(())
}

/// Path of the editor's annotation document for one capture. The document
/// is created lazily by the editor on first save.
pub fn annotations_path(id: &str) -> Result<PathBuf, String> {
    validate_id(id)?;
    Ok(capture_dir(&library_root()?, id).join("annotations.json"))
}

#[derive(Debug)]
pub enum AnnotationSource {
    Transient,
    Missing {
        path: PathBuf,
    },
    Loaded {
        document: crate::editor::model::Document,
        path: PathBuf,
    },
    Rejected {
        error: String,
    },
}

pub fn read_annotations(id: &str) -> AnnotationSource {
    let path = match annotations_path(id) {
        Ok(path) => path,
        Err(error) => return AnnotationSource::Rejected { error },
    };
    annotation_source_from_path(path)
}

fn annotation_source_from_path(path: PathBuf) -> AnnotationSource {
    match read_annotations_from_path(&path) {
        Ok(Some(document)) => AnnotationSource::Loaded { document, path },
        Ok(None) => AnnotationSource::Missing { path },
        Err(error) => AnnotationSource::Rejected { error },
    }
}

fn read_annotations_from_path(
    path: &Path,
) -> Result<Option<crate::editor::model::Document>, String> {
    match fs::read(path) {
        Ok(bytes) => match crate::editor::model::Document::from_bytes(&bytes) {
            Ok(document) => Ok(Some(document)),
            Err(error) => Err(format!(
                "Could not read annotation document at {}: {error}",
                path.display()
            )),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("Could not read the annotation document: {error}")),
    }
}

/// Save an annotation document at the path the editor was opened with.
pub fn write_annotations_idempotent(
    document: &crate::editor::model::Document,
    path: &Path,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Could not create the capture folder: {error}"))?;
    }
    let bytes = document.to_bytes()?;
    write_atomic(path, &bytes)
}

/// Replace a capture's image (after an in-editor crop) and update the index.
pub fn replace_image(id: &str, png: &[u8], width: u32, height: u32) -> Result<(), String> {
    validate_id(id)?;
    validate_size(&ImageSize { width, height })?;
    let root = library_root()?;
    let mut entries = read_index(&root)?;
    let Some(entry) = entries.iter_mut().find(|entry| entry.id == id) else {
        return Err("No such capture in the library.".to_string());
    };
    write_atomic(&capture_dir(&root, id).join("image.png"), png)?;
    entry.image = ImageSize { width, height };
    entry.updated_at = now_millis();
    write_index(&root, &entries)
}

fn generate_id() -> String {
    static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    format!(
        "{}-{}",
        now_millis(),
        NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    )
}

pub fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{
        AnnotationSource, ImageSize, IndexEntry, annotation_source_from_path, generate_id,
        normalize_index_entry, resolve_data_root, validate_id, validate_index_entry, validate_size,
        validate_title, write_annotations_idempotent, write_atomic,
    };
    use std::fs;

    fn valid_entry() -> IndexEntry {
        IndexEntry {
            id: "capture-1".to_string(),
            title: "A capture".to_string(),
            created_at: 1,
            updated_at: 2,
            image: ImageSize {
                width: 320,
                height: 180,
            },
        }
    }

    #[test]
    fn library_ids_are_safe_for_directory_names() {
        assert!(validate_id("capture-1").is_ok());
        assert!(validate_id("../capture").is_err());
        assert!(validate_id("").is_err());
    }

    #[test]
    fn library_sizes_accept_edges_and_reject_invalid_dimensions() {
        assert!(
            validate_size(&ImageSize {
                width: 100_000,
                height: 100_000,
            })
            .is_ok()
        );
        assert!(
            validate_size(&ImageSize {
                width: 0,
                height: 100,
            })
            .is_err()
        );
        assert!(
            validate_size(&ImageSize {
                width: 100_001,
                height: 100,
            })
            .is_err()
        );
    }

    #[test]
    fn malformed_index_entries_are_rejected_before_disk_access() {
        let mut entry = valid_entry();
        entry.id = "../../outside".to_string();
        assert!(validate_index_entry(&entry).is_err());

        let mut oversized = valid_entry();
        oversized.title = "x".repeat(501);
        assert!(validate_index_entry(&oversized).is_err());
    }

    #[test]
    fn titles_are_trimmed_and_cannot_be_blank() {
        assert!(validate_title(" Capture ").is_ok());
        assert!(validate_title("   ").is_err());
        assert!(validate_title(&"界".repeat(500)).is_ok());
        assert!(validate_title(&"界".repeat(501)).is_err());
    }

    #[test]
    fn atomic_write_replaces_existing_file() {
        let directory = std::env::temp_dir().join(format!(
            "snipsy-atomic-write-test-{}-{}",
            std::process::id(),
            generate_id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("settings.json");
        fs::write(&path, b"old contents").unwrap();

        write_atomic(&path, b"new contents").unwrap();

        assert_eq!(fs::read(&path).unwrap(), b"new contents");
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn legacy_blank_titles_are_recovered_before_index_validation() {
        let mut entry = valid_entry();
        entry.title = " \t ".to_string();
        normalize_index_entry(&mut entry).unwrap();
        assert_eq!(entry.title, "Capture capture-1");
    }

    #[test]
    fn generated_ids_are_valid_library_ids() {
        for _ in 0..5 {
            assert!(validate_id(&generate_id()).is_ok());
        }
    }

    #[test]
    fn legacy_data_directory_moves_to_the_canonical_snipsy_path() {
        let base = std::env::temp_dir().join(format!(
            "snipsy-data-migration-test-{}-{}",
            std::process::id(),
            generate_id()
        ));
        let legacy = base.join("CleanShotW");
        fs::create_dir_all(&legacy).unwrap();
        fs::write(legacy.join("settings.json"), b"{}").unwrap();
        let old_capture = legacy.join("library").join("capture-1");
        fs::create_dir_all(&old_capture).unwrap();
        fs::write(old_capture.join("image.png"), b"png bytes").unwrap();

        let root = resolve_data_root(&base).unwrap();
        assert_eq!(root, base.join("Snipsy"));
        assert!(root.join("settings.json").exists());
        assert_eq!(
            fs::read(root.join("library/capture-1/image.png")).unwrap(),
            b"png bytes"
        );
        assert!(!legacy.exists());
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn existing_snipsy_data_wins_over_a_legacy_directory() {
        let base = std::env::temp_dir().join(format!(
            "snipsy-data-precedence-test-{}-{}",
            std::process::id(),
            generate_id()
        ));
        let canonical = base.join("Snipsy");
        let legacy = base.join("CleanShotW");
        fs::create_dir_all(&canonical).unwrap();
        fs::create_dir_all(&legacy).unwrap();
        fs::write(legacy.join("settings.json"), b"legacy").unwrap();

        assert_eq!(resolve_data_root(&base).unwrap(), canonical);
        assert!(legacy.join("settings.json").exists());
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn invalid_annotation_files_are_rejected_without_being_modified() {
        let path = std::env::temp_dir().join(format!(
            "snipsy-phase3-{}-{}.json",
            std::process::id(),
            super::now_millis()
        ));
        let original = b"{\"version\":99}";
        std::fs::write(&path, original).unwrap();

        let source = super::annotation_source_from_path(path.clone());
        let bytes_after_read = std::fs::read(&path).unwrap();
        std::fs::remove_file(path).unwrap();

        assert!(matches!(source, AnnotationSource::Rejected { .. }));
        assert_eq!(bytes_after_read, original);
    }

    #[test]
    fn versionless_annotation_files_load_and_are_saved_with_a_version() {
        let directory = std::env::temp_dir().join(format!(
            "snipsy-versionless-annotations-{}-{}",
            std::process::id(),
            generate_id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("annotations.json");
        fs::write(&path, br#"{"imageWidth":10,"imageHeight":10,"shapes":[]}"#).unwrap();

        let AnnotationSource::Loaded {
            document,
            path: loaded_path,
        } = annotation_source_from_path(path.clone())
        else {
            panic!("versionless annotation document was not loaded");
        };
        assert_eq!(document.version, crate::editor::model::DOCUMENT_VERSION);

        write_annotations_idempotent(&document, &loaded_path).unwrap();
        let saved: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(saved["version"], crate::editor::model::DOCUMENT_VERSION);
        assert!(matches!(
            annotation_source_from_path(path.clone()),
            AnnotationSource::Loaded { .. }
        ));

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn deleting_capture_removes_index_entry_and_capture_files() {
        let root = std::env::temp_dir().join(format!(
            "snipsy-library-delete-{}-{}",
            std::process::id(),
            super::now_millis()
        ));
        let dir = root.join("capture-1");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("image.png"), b"capture").unwrap();
        super::write_index(&root, &[valid_entry()]).unwrap();

        super::delete_capture_from_root(&root, "capture-1").unwrap();

        assert!(super::read_index(&root).unwrap().is_empty());
        assert!(!dir.exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn deleting_invalid_or_missing_library_ids_is_non_destructive() {
        let root = std::env::temp_dir().join(format!(
            "snipsy-library-delete-empty-{}-{}",
            std::process::id(),
            super::now_millis()
        ));
        std::fs::create_dir_all(&root).unwrap();
        assert!(super::delete_capture_from_root(&root, "../outside").is_err());
        assert!(super::delete_capture_from_root(&root, "missing").is_ok());
        assert!(super::read_index(&root).unwrap().is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }
}
