use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureRecordWire {
    pub id: String,
    pub title: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub image: ImageSize,
    pub image_base64: String,
    pub thumb_base64: String,
    pub annotations: Option<Value>,
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
    dirs::data_local_dir()
        .map(|path| path.join("CleanShotW"))
        .ok_or_else(|| "Windows did not provide a local application-data directory.".to_string())
}

fn library_root() -> Result<PathBuf, String> {
    Ok(app_data_root()?.join("library"))
}

fn validate_id(id: &str) -> Result<(), String> {
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
    if title.len() > 500 {
        return Err("The capture title is too long.".to_string());
    }
    Ok(())
}

fn validate_index_entry(entry: &IndexEntry) -> Result<(), String> {
    validate_id(&entry.id)?;
    validate_size(&entry.image)?;
    validate_title(&entry.title)
}

fn decode_payload(name: &str, value: &str) -> Result<Vec<u8>, String> {
    let bytes = BASE64
        .decode(value)
        .map_err(|error| format!("Could not decode {name}: {error}"))?;
    if bytes.is_empty() {
        return Err(format!("The {name} is empty."));
    }
    Ok(bytes)
}

fn read_index(root: &Path) -> Result<Vec<IndexEntry>, String> {
    let path = root.join(INDEX_FILE);
    let entries = match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|error| format!("Could not read the library index: {error}"))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("Could not read the library index: {error}")),
    };
    for entry in &entries {
        validate_index_entry(entry)?;
    }
    Ok(entries)
}

fn write_json<T: Serialize + ?Sized>(path: &Path, value: &T) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("Could not encode {}: {error}", path.display()))?;
    write_atomic(path, &bytes)
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temp = path.with_extension("tmp");
    fs::write(&temp, bytes)
        .map_err(|error| format!("Could not write {}: {error}", path.display()))?;
    if path.exists() {
        fs::remove_file(path)
            .map_err(|error| format!("Could not replace {}: {error}", path.display()))?;
    }
    fs::rename(&temp, path)
        .map_err(|error| format!("Could not finish writing {}: {error}", path.display()))
}

fn write_index(root: &Path, entries: &[IndexEntry]) -> Result<(), String> {
    write_json(&root.join(INDEX_FILE), entries)
}

fn capture_dir(root: &Path, id: &str) -> PathBuf {
    root.join(id)
}

fn load_record(root: &Path, entry: &IndexEntry) -> Result<CaptureRecordWire, String> {
    let dir = capture_dir(root, &entry.id);
    let image = fs::read(dir.join("image.png"))
        .map_err(|error| format!("Could not read capture {}: {error}", entry.id))?;
    let thumb = fs::read(dir.join("thumbnail.png")).unwrap_or_else(|_| image.clone());
    let annotations = match fs::read(dir.join("annotations.json")) {
        Ok(bytes) => {
            let value = serde_json::from_slice::<Value>(&bytes)
                .map_err(|error| format!("Could not read annotations for {}: {error}", entry.id))?;
            if value.is_null() {
                None
            } else {
                Some(value)
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(format!(
                "Could not read annotations for {}: {error}",
                entry.id
            ))
        }
    };
    Ok(CaptureRecordWire {
        id: entry.id.clone(),
        title: entry.title.clone(),
        created_at: entry.created_at,
        updated_at: entry.updated_at,
        image: entry.image.clone(),
        image_base64: BASE64.encode(image),
        thumb_base64: BASE64.encode(thumb),
        annotations,
    })
}

#[tauri::command]
pub fn library_save_capture(record: CaptureRecordWire) -> Result<(), String> {
    validate_id(&record.id)?;
    validate_size(&record.image)?;
    validate_title(&record.title)?;
    let image = decode_payload("capture image", &record.image_base64)?;
    let thumb = decode_payload("capture thumbnail", &record.thumb_base64)?;
    let root = library_root()?;
    fs::create_dir_all(&root).map_err(|error| format!("Could not create the library: {error}"))?;
    let dir = capture_dir(&root, &record.id);
    fs::create_dir_all(&dir)
        .map_err(|error| format!("Could not create capture folder: {error}"))?;
    write_atomic(&dir.join("image.png"), &image)?;
    write_atomic(&dir.join("thumbnail.png"), &thumb)?;
    write_json(&dir.join("annotations.json"), &record.annotations)?;

    let mut entries = read_index(&root)?;
    let entry = IndexEntry {
        id: record.id,
        title: record.title,
        created_at: record.created_at,
        updated_at: record.updated_at,
        image: record.image,
    };
    if let Some(existing) = entries.iter_mut().find(|item| item.id == entry.id) {
        *existing = entry;
    } else {
        entries.push(entry);
    }
    write_index(&root, &entries)
}

#[tauri::command]
pub fn library_list_captures() -> Result<Vec<CaptureRecordWire>, String> {
    let root = library_root()?;
    let mut entries = read_index(&root)?;
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.updated_at));
    entries
        .iter()
        .map(|entry| load_record(&root, entry))
        .collect()
}

#[tauri::command]
pub fn library_get_capture(id: String) -> Result<Option<CaptureRecordWire>, String> {
    validate_id(&id)?;
    let root = library_root()?;
    let entries = read_index(&root)?;
    entries
        .iter()
        .find(|entry| entry.id == id)
        .map(|entry| load_record(&root, entry))
        .transpose()
}

#[tauri::command]
pub fn library_update_annotations(id: String, annotations: Option<Value>) -> Result<bool, String> {
    validate_id(&id)?;
    let root = library_root()?;
    let mut entries = read_index(&root)?;
    let Some(entry) = entries.iter_mut().find(|entry| entry.id == id) else {
        return Ok(false);
    };
    let dir = capture_dir(&root, &id);
    write_json(&dir.join("annotations.json"), &annotations)?;
    entry.updated_at = now_millis();
    write_index(&root, &entries)?;
    Ok(true)
}

#[tauri::command]
pub fn library_update_title(id: String, title: String) -> Result<bool, String> {
    validate_id(&id)?;
    validate_title(&title)?;
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

#[tauri::command]
pub fn library_delete_capture(id: String) -> Result<(), String> {
    validate_id(&id)?;
    let root = library_root()?;
    let mut entries = read_index(&root)?;
    let before = entries.len();
    entries.retain(|entry| entry.id != id);
    if entries.len() == before {
        return Ok(());
    }
    let dir = capture_dir(&root, &id);
    if dir.exists() {
        fs::remove_dir_all(&dir)
            .map_err(|error| format!("Could not delete capture {id}: {error}"))?;
    }
    write_index(&root, &entries)
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
        decode_payload, validate_id, validate_index_entry, validate_size, validate_title, ImageSize,
        IndexEntry,
    };

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
        assert!(validate_size(&ImageSize {
            width: 100_000,
            height: 100_000,
        })
        .is_ok());
        assert!(validate_size(&ImageSize {
            width: 0,
            height: 100,
        })
        .is_err());
        assert!(validate_size(&ImageSize {
            width: 100_001,
            height: 100,
        })
        .is_err());
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
    fn base64_payloads_must_decode_to_non_empty_bytes() {
        assert_eq!(decode_payload("image", "aGVsbG8=").unwrap(), b"hello");
        assert!(decode_payload("image", "not base64").is_err());
        assert!(decode_payload("image", "").is_err());
    }

    #[test]
    fn titles_are_trimmed_by_the_command_and_cannot_be_blank() {
        assert!(validate_title(" Capture ").is_ok());
        assert!(validate_title("   ").is_err());
        assert!(validate_title(&"x".repeat(501)).is_err());
    }
}
