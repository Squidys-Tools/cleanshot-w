use std::{
    fmt,
    fs,
    io::Cursor,
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
    time::{SystemTime, UNIX_EPOCH},
};

use image::{ImageFormat, ImageReader};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tauri::{AppHandle, Manager, State};
use url::Url;
use uuid::Uuid;

const SCHEMA_VERSION: i64 = 1;
const MAX_FILE_BYTES: usize = 50 * 1024 * 1024;
const MAX_IMAGE_DIMENSION: u32 = 20_000;
const MAX_IMAGE_ALLOC_BYTES: u64 = 256 * 1024 * 1024;
const THUMBNAIL_EDGE: u32 = 512;

const ITEMS_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS items (
    id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL,
    title TEXT,
    description TEXT,
    source_url TEXT,
    source_label TEXT,
    local_asset_path TEXT,
    thumbnail_path TEXT,
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    archived INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0, 1)),
    favorite INTEGER NOT NULL DEFAULT 0 CHECK (favorite IN (0, 1))
);

CREATE INDEX IF NOT EXISTS idx_items_active_updated
    ON items (archived, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_items_kind
    ON items (kind);
"#;

#[derive(Debug)]
pub enum StorageError {
    Io(std::io::Error),
    Image(image::ImageError),
    Sql(rusqlite::Error),
    Json(serde_json::Error),
    NotInitialized,
    NotFound(String),
    InvalidInput(String),
}

impl fmt::Display for StorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "storage filesystem error: {error}"),
            Self::Image(error) => write!(formatter, "image processing error: {error}"),
            Self::Sql(error) => write!(formatter, "storage database error: {error}"),
            Self::Json(error) => write!(formatter, "storage metadata error: {error}"),
            Self::NotInitialized => write!(formatter, "storage has not been initialized"),
            Self::NotFound(id) => write!(formatter, "item not found: {id}"),
            Self::InvalidInput(message) => write!(formatter, "invalid storage input: {message}"),
        }
    }
}

impl std::error::Error for StorageError {}

impl From<std::io::Error> for StorageError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<image::ImageError> for StorageError {
    fn from(error: image::ImageError) -> Self {
        Self::Image(error)
    }
}

impl From<rusqlite::Error> for StorageError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sql(error)
    }
}

impl From<serde_json::Error> for StorageError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<StorageError> for String {
    fn from(error: StorageError) -> Self {
        error.to_string()
    }
}

#[derive(Default)]
pub struct StorageState {
    database: Mutex<Option<LibraryStorage>>,
}

pub struct LibraryStorage {
    connection: Connection,
    database_path: PathBuf,
    fts5_enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageStatus {
    pub database_path: String,
    pub fts5_enabled: bool,
    pub schema_version: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemDto {
    pub id: String,
    pub kind: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub source_url: Option<String>,
    pub source_label: Option<String>,
    pub local_asset_path: Option<String>,
    pub thumbnail_path: Option<String>,
    pub metadata: Value,
    pub created_at: i64,
    pub updated_at: i64,
    pub archived: bool,
    pub favorite: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateNoteInput {
    pub title: Option<String>,
    pub body: String,
    pub metadata: Option<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateUrlInput {
    pub source_url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub body: String,
    pub metadata: Option<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveFileInput {
    #[serde(default, alias = "itemId")]
    pub id: Option<String>,
    pub file_name: String,
    pub mime_type: Option<String>,
    pub kind: Option<String>,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateItemInput {
    pub id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub source_url: Option<String>,
    pub source_label: Option<String>,
    pub local_asset_path: Option<String>,
    pub thumbnail_path: Option<String>,
    pub metadata: Option<Value>,
    pub favorite: Option<bool>,
}

impl StorageState {
    fn lock(&self) -> Result<MutexGuard<'_, Option<LibraryStorage>>, StorageError> {
        self.database
            .lock()
            .map_err(|_| StorageError::InvalidInput("storage lock was poisoned".into()))
    }

    fn require_storage(&self) -> Result<MutexGuard<'_, Option<LibraryStorage>>, StorageError> {
        let guard = self.lock()?;
        if guard.is_none() {
            return Err(StorageError::NotInitialized);
        }
        Ok(guard)
    }
}

impl LibraryStorage {
    fn open(database_path: PathBuf) -> Result<Self, StorageError> {
        let connection = Connection::open(&database_path)?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        connection.execute_batch(ITEMS_SCHEMA)?;

        let fts5_enabled = setup_fts5(&connection);
        connection.pragma_update(None, "user_version", SCHEMA_VERSION)?;

        Ok(Self {
            connection,
            database_path,
            fts5_enabled,
        })
    }

    fn status(&self) -> StorageStatus {
        StorageStatus {
            database_path: self.database_path.to_string_lossy().into_owned(),
            fts5_enabled: self.fts5_enabled,
            schema_version: SCHEMA_VERSION,
        }
    }

    fn list_active_items(&self) -> Result<Vec<ItemDto>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT id, kind, title, description, source_url, source_label,
                    local_asset_path, thumbnail_path, metadata, created_at,
                    updated_at, archived, favorite
             FROM items
             WHERE archived = 0
             ORDER BY updated_at DESC, created_at DESC",
        )?;

        let items = statement
            .query_map([], item_from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(items)
    }

    fn create_note(&self, input: CreateNoteInput) -> Result<ItemDto, StorageError> {
        let body = input.body.trim().to_owned();
        if body.is_empty() {
            return Err(StorageError::InvalidInput(
                "note body cannot be empty".into(),
            ));
        }

        let id = Uuid::new_v4().to_string();
        let timestamp = now_millis()?;
        let title = input.title.and_then(non_empty_string);
        let metadata = input.metadata.unwrap_or_else(|| Value::Object(Map::new()));
        let metadata_json = serde_json::to_string(&metadata)?;

        self.connection.execute(
            "INSERT INTO items (
                id, kind, title, description, metadata, created_at, updated_at
             ) VALUES (?1, 'note', ?2, ?3, ?4, ?5, ?5)",
            params![id, title, body, metadata_json, timestamp],
        )?;

        self.get_item(&id)?.ok_or(StorageError::NotFound(id))
    }

    fn create_url(&self, input: CreateUrlInput) -> Result<ItemDto, StorageError> {
        let source_url = normalize_http_url(&input.source_url)?;
        let source_label = Url::parse(&source_url)
            .ok()
            .and_then(|url| url.host_str().map(str::to_owned));
        let title = input
            .title
            .and_then(non_empty_string)
            .or_else(|| source_label.clone());
        let description = input.description.and_then(non_empty_string);
        let body = input.body.trim().to_owned();
        let metadata = article_metadata(input.metadata, &source_url, &body)?;
        let metadata_json = serde_json::to_string(&metadata)?;
        let id = Uuid::new_v4().to_string();
        let timestamp = now_millis()?;

        self.connection.execute(
            "INSERT INTO items (
                id, kind, title, description, source_url, source_label,
                metadata, created_at, updated_at
             ) VALUES (?1, 'url', ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            params![
                id,
                title,
                description,
                source_url,
                source_label,
                metadata_json,
                timestamp,
            ],
        )?;

        self.get_item(&id)?.ok_or(StorageError::NotFound(id))
    }

    fn save_file(&self, input: SaveFileInput) -> Result<ItemDto, StorageError> {
        if input.bytes.len() > MAX_FILE_BYTES {
            return Err(StorageError::InvalidInput(format!(
                "file exceeds the {} MB input limit",
                MAX_FILE_BYTES / (1024 * 1024)
            )));
        }
        if input.bytes.is_empty() {
            return Err(StorageError::InvalidInput("file bytes cannot be empty".into()));
        }

        let file_name = sanitize_file_name(&input.file_name)?;
        let id = match input.id {
            Some(id) => validate_item_id(id)?,
            None => Uuid::new_v4().to_string(),
        };
        let mime_type = input
            .mime_type
            .as_deref()
            .map(normalize_mime_type)
            .filter(|value| !value.is_empty());
        let kind = normalize_file_kind(input.kind.as_deref(), mime_type.as_deref(), &file_name)?;
        let thumbnail = if kind == "image" {
            Some(make_image_thumbnail(&input.bytes)?)
        } else {
            None
        };

        let item_directory = self.assets_directory().join(&id);
        fs::create_dir_all(&item_directory)?;
        let original_path = item_directory.join(&file_name);
        fs::write(&original_path, &input.bytes)?;

        let thumbnail_path = if let Some(thumbnail_bytes) = thumbnail {
            let path = item_directory.join("thumbnail.webp");
            fs::write(&path, thumbnail_bytes)?;
            Some(path)
        } else {
            None
        };

        let local_asset_path = relative_asset_path(&original_path, &self.assets_root())?;
        let thumbnail_path = thumbnail_path
            .as_ref()
            .map(|path| relative_asset_path(path, &self.assets_root()))
            .transpose()?;
        let metadata = file_metadata(&file_name, mime_type.as_deref(), input.bytes.len());
        let metadata_json = serde_json::to_string(&metadata)?;
        let timestamp = now_millis()?;
        let title = Some(file_name.clone());
        let source_label = mime_type.clone();

        let existing = self.get_item(&id)?;
        if existing.is_some() {
            self.connection.execute(
                "UPDATE items
                 SET kind = ?2, title = ?3, source_label = ?4,
                     local_asset_path = ?5, thumbnail_path = ?6,
                     metadata = ?7, archived = 0, updated_at = ?8
                 WHERE id = ?1",
                params![
                    id,
                    kind,
                    title,
                    source_label,
                    local_asset_path,
                    thumbnail_path,
                    metadata_json,
                    timestamp,
                ],
            )?;
        } else {
            self.connection.execute(
                "INSERT INTO items (
                    id, kind, title, source_label, local_asset_path,
                    thumbnail_path, metadata, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
                params![
                    id,
                    kind,
                    title,
                    source_label,
                    local_asset_path,
                    thumbnail_path,
                    metadata_json,
                    timestamp,
                ],
            )?;
        }

        self.get_item(&id)?.ok_or(StorageError::NotFound(id))
    }

    fn assets_root(&self) -> PathBuf {
        self.database_path
            .parent()
            .map(PathBuf::from)
            .unwrap_or_default()
            .join("assets")
    }

    fn assets_directory(&self) -> PathBuf {
        self.assets_root().join("items")
    }

    fn resolve_asset_path(&self, relative_path: &str) -> Result<String, StorageError> {
        let normalized = relative_path.replace('\\', "/");
        let relative = normalized.strip_prefix("assets/").ok_or_else(|| {
            StorageError::InvalidInput("asset path must be relative to the managed assets directory".into())
        })?;
        let relative_path = Path::new(relative);
        if relative_path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        }) {
            return Err(StorageError::InvalidInput("asset path contains an unsafe segment".into()));
        }

        let assets_root = fs::canonicalize(self.assets_root())?;
        let candidate = fs::canonicalize(assets_root.join(relative_path))?;
        if !candidate.starts_with(&assets_root) {
            return Err(StorageError::InvalidInput("asset path escaped the managed assets directory".into()));
        }
        Ok(candidate.to_string_lossy().into_owned())
    }

    fn update_item(&self, input: UpdateItemInput) -> Result<ItemDto, StorageError> {
        let metadata_json = input
            .metadata
            .map(|metadata| serde_json::to_string(&metadata))
            .transpose()?;
        let title = input.title.as_deref().map(str::trim);
        let description = input.description.as_deref().map(str::trim);
        let now = now_millis()?;

        let updated = self.connection.execute(
            "UPDATE items
             SET title = COALESCE(?2, title),
                 description = COALESCE(?3, description),
                 source_url = COALESCE(?4, source_url),
                 source_label = COALESCE(?5, source_label),
                 local_asset_path = COALESCE(?6, local_asset_path),
                 thumbnail_path = COALESCE(?7, thumbnail_path),
                 metadata = COALESCE(?8, metadata),
                 favorite = COALESCE(?9, favorite),
                 updated_at = ?10
             WHERE id = ?1",
            params![
                input.id,
                title,
                description,
                input.source_url,
                input.source_label,
                input.local_asset_path,
                input.thumbnail_path,
                metadata_json,
                input.favorite.map(bool_to_int),
                now,
            ],
        )?;

        if updated == 0 {
            return Err(StorageError::NotFound(input.id));
        }

        self.get_item(&input.id)?.ok_or(StorageError::NotFound(input.id))
    }

    fn archive_item(&self, id: &str, archived: bool) -> Result<ItemDto, StorageError> {
        let updated = self.connection.execute(
            "UPDATE items SET archived = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, bool_to_int(archived), now_millis()?],
        )?;

        if updated == 0 {
            return Err(StorageError::NotFound(id.to_owned()));
        }

        self.get_item(id)?.ok_or_else(|| StorageError::NotFound(id.to_owned()))
    }

    fn search_items(&self, query: &str, limit: u32) -> Result<Vec<ItemDto>, StorageError> {
        let limit = i64::from(limit.clamp(1, 200));
        let query = query.trim();

        if query.is_empty() {
            return self.list_active_items_limited(limit);
        }

        if self.fts5_enabled {
            let fts_query = escape_fts_query(query);
            let mut statement = self.connection.prepare(
                "SELECT i.id, i.kind, i.title, i.description, i.source_url,
                        i.source_label, i.local_asset_path, i.thumbnail_path,
                        i.metadata, i.created_at, i.updated_at, i.archived,
                        i.favorite
                 FROM items_fts f
                 JOIN items i ON i.id = f.item_id
                 WHERE i.archived = 0 AND f MATCH ?1
                 ORDER BY i.updated_at DESC, i.created_at DESC
                 LIMIT ?2",
            )?;

            let items = statement
                .query_map(params![fts_query, limit], item_from_row)?
                .collect::<Result<Vec<_>, _>>()?;
            return Ok(items);
        }

        let pattern = format!("%{query}%");
        let mut statement = self.connection.prepare(
            "SELECT id, kind, title, description, source_url, source_label,
                    local_asset_path, thumbnail_path, metadata, created_at,
                    updated_at, archived, favorite
             FROM items
             WHERE archived = 0
               AND (title LIKE ?1 COLLATE NOCASE
                    OR description LIKE ?1 COLLATE NOCASE
                    OR source_label LIKE ?1 COLLATE NOCASE
                    OR metadata LIKE ?1 COLLATE NOCASE)
             ORDER BY updated_at DESC, created_at DESC
             LIMIT ?2",
        )?;

        let items = statement
            .query_map(params![pattern, limit], item_from_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(items)
    }

    fn list_active_items_limited(&self, limit: i64) -> Result<Vec<ItemDto>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT id, kind, title, description, source_url, source_label,
                    local_asset_path, thumbnail_path, metadata, created_at,
                    updated_at, archived, favorite
             FROM items
             WHERE archived = 0
             ORDER BY updated_at DESC, created_at DESC
             LIMIT ?1",
        )?;

        let items = statement
            .query_map(params![limit], item_from_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(items)
    }

    fn get_item(&self, id: &str) -> Result<Option<ItemDto>, StorageError> {
        self.connection
            .query_row(
                "SELECT id, kind, title, description, source_url, source_label,
                        local_asset_path, thumbnail_path, metadata, created_at,
                        updated_at, archived, favorite
                 FROM items WHERE id = ?1",
                params![id],
                item_from_row,
            )
            .optional()
            .map_err(StorageError::from)
    }
}

#[tauri::command]
pub fn initialize_storage(
    app: AppHandle,
    state: State<'_, StorageState>,
) -> Result<StorageStatus, String> {
    let database_directory = app
        .path()
        .app_data_dir()
        .map_err(|error| StorageError::InvalidInput(error.to_string()))?;
    fs::create_dir_all(&database_directory).map_err(StorageError::from)?;

    let database_path = database_directory.join("library.sqlite3");
    let mut database = state.lock().map_err(String::from)?;

    if let Some(existing) = database.as_ref() {
        if existing.database_path == database_path {
            return Ok(existing.status());
        }
    }

    let storage = LibraryStorage::open(database_path).map_err(String::from)?;
    let status = storage.status();
    *database = Some(storage);
    Ok(status)
}

#[tauri::command]
pub fn list_active_items(state: State<'_, StorageState>) -> Result<Vec<ItemDto>, String> {
    let database = state.require_storage().map_err(String::from)?;
    database
        .as_ref()
        .expect("require_storage guarantees initialization")
        .list_active_items()
        .map_err(String::from)
}

#[tauri::command]
pub fn create_note(
    input: CreateNoteInput,
    state: State<'_, StorageState>,
) -> Result<ItemDto, String> {
    let database = state.require_storage().map_err(String::from)?;
    database
        .as_ref()
        .expect("require_storage guarantees initialization")
        .create_note(input)
        .map_err(String::from)
}

#[tauri::command]
pub fn create_url(
    input: CreateUrlInput,
    state: State<'_, StorageState>,
) -> Result<ItemDto, String> {
    let database = state.require_storage().map_err(String::from)?;
    database
        .as_ref()
        .expect("require_storage guarantees initialization")
        .create_url(input)
        .map_err(String::from)
}

#[tauri::command]
pub fn save_file(
    input: SaveFileInput,
    state: State<'_, StorageState>,
) -> Result<ItemDto, String> {
    let database = state.require_storage().map_err(String::from)?;
    database
        .as_ref()
        .expect("require_storage guarantees initialization")
        .save_file(input)
        .map_err(String::from)
}

#[tauri::command]
pub fn resolve_asset_path(
    path: String,
    state: State<'_, StorageState>,
) -> Result<String, String> {
    let database = state.require_storage().map_err(String::from)?;
    database
        .as_ref()
        .expect("require_storage guarantees initialization")
        .resolve_asset_path(&path)
        .map_err(String::from)
}

#[tauri::command]
pub fn update_item(
    input: UpdateItemInput,
    state: State<'_, StorageState>,
) -> Result<ItemDto, String> {
    let database = state.require_storage().map_err(String::from)?;
    database
        .as_ref()
        .expect("require_storage guarantees initialization")
        .update_item(input)
        .map_err(String::from)
}

#[tauri::command]
pub fn archive_item(
    id: String,
    archived: Option<bool>,
    state: State<'_, StorageState>,
) -> Result<ItemDto, String> {
    let database = state.require_storage().map_err(String::from)?;
    database
        .as_ref()
        .expect("require_storage guarantees initialization")
        .archive_item(&id, archived.unwrap_or(true))
        .map_err(String::from)
}

#[tauri::command]
pub fn search_items(
    query: String,
    limit: Option<u32>,
    state: State<'_, StorageState>,
) -> Result<Vec<ItemDto>, String> {
    let database = state.require_storage().map_err(String::from)?;
    database
        .as_ref()
        .expect("require_storage guarantees initialization")
        .search_items(&query, limit.unwrap_or(50))
        .map_err(String::from)
}

fn setup_fts5(connection: &Connection) -> bool {
    let result = connection.execute_batch(
        r#"
        CREATE VIRTUAL TABLE IF NOT EXISTS items_fts USING fts5(
            item_id UNINDEXED,
            title,
            description,
            source_label,
            metadata
        );

        CREATE TRIGGER IF NOT EXISTS items_fts_after_insert
        AFTER INSERT ON items BEGIN
            INSERT INTO items_fts(item_id, title, description, source_label, metadata)
            VALUES (new.id, new.title, new.description, new.source_label, new.metadata);
        END;

        CREATE TRIGGER IF NOT EXISTS items_fts_after_delete
        AFTER DELETE ON items BEGIN
            DELETE FROM items_fts WHERE item_id = old.id;
        END;

        CREATE TRIGGER IF NOT EXISTS items_fts_after_update
        AFTER UPDATE ON items BEGIN
            DELETE FROM items_fts WHERE item_id = old.id;
            INSERT INTO items_fts(item_id, title, description, source_label, metadata)
            VALUES (new.id, new.title, new.description, new.source_label, new.metadata);
        END;

        DELETE FROM items_fts;
        INSERT INTO items_fts(item_id, title, description, source_label, metadata)
        SELECT id, title, description, source_label, metadata FROM items;
        "#,
    );

    if let Err(error) = result {
        eprintln!("FTS5 unavailable; using LIKE search fallback: {error}");
        false
    } else {
        true
    }
}

fn item_from_row(row: &Row<'_>) -> rusqlite::Result<ItemDto> {
    let metadata_json: String = row.get(8)?;
    let metadata = serde_json::from_str(&metadata_json).unwrap_or_else(|_| Value::Object(Map::new()));

    Ok(ItemDto {
        id: row.get(0)?,
        kind: row.get(1)?,
        title: row.get(2)?,
        description: row.get(3)?,
        source_url: row.get(4)?,
        source_label: row.get(5)?,
        local_asset_path: row.get(6)?,
        thumbnail_path: row.get(7)?,
        metadata,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
        archived: row.get::<_, i64>(11)? != 0,
        favorite: row.get::<_, i64>(12)? != 0,
    })
}

fn escape_fts_query(query: &str) -> String {
    query
        .split_whitespace()
        .filter(|token| !token.is_empty())
        .map(|token| format!("\"{}\"", token.replace('"', "")))
        .filter(|token| token != "\"\"")
        .collect::<Vec<_>>()
        .join(" ")
}

fn non_empty_string(value: String) -> Option<String> {
    let value = value.trim().to_owned();
    (!value.is_empty()).then_some(value)
}

fn normalize_http_url(value: &str) -> Result<String, StorageError> {
    let mut url = Url::parse(value.trim()).map_err(|_| {
        StorageError::InvalidInput("sourceUrl must be a valid HTTP or HTTPS URL".into())
    })?;

    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(StorageError::InvalidInput(
            "sourceUrl must use HTTP or HTTPS and include a host".into(),
        ));
    }

    url.set_fragment(None);
    Ok(url.to_string())
}

fn article_metadata(
    input: Option<Value>,
    source_url: &str,
    body: &str,
) -> Result<Value, StorageError> {
    let mut metadata = match input.unwrap_or_else(|| Value::Object(Map::new())) {
        Value::Object(metadata) => metadata,
        _ => {
            return Err(StorageError::InvalidInput(
                "metadata must be a JSON object".into(),
            ))
        }
    };

    metadata
        .entry("sourceUrl")
        .or_insert_with(|| Value::String(source_url.to_owned()));
    metadata
        .entry("text")
        .or_insert_with(|| Value::String(body.to_owned()));
    ensure_array_metadata(&mut metadata, "imageUrls")?;
    ensure_array_metadata(&mut metadata, "safeEmbeds")?;
    metadata
        .entry("html")
        .or_insert_with(|| Value::String(String::new()));

    Ok(Value::Object(metadata))
}

fn ensure_array_metadata(metadata: &mut Map<String, Value>, key: &str) -> Result<(), StorageError> {
    match metadata.entry(key.to_owned()) {
        serde_json::map::Entry::Vacant(entry) => {
            entry.insert(Value::Array(Vec::new()));
        }
        serde_json::map::Entry::Occupied(entry) if !entry.get().is_array() => {
            return Err(StorageError::InvalidInput(format!(
                "metadata.{key} must be a JSON array"
            )));
        }
        serde_json::map::Entry::Occupied(_) => {}
    }
    Ok(())
}

fn validate_item_id(id: String) -> Result<String, StorageError> {
    let id = id.trim().to_owned();
    if id.is_empty() || id.len() > 128 {
        return Err(StorageError::InvalidInput(
            "item id must contain 1 to 128 characters".into(),
        ));
    }
    if !id
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err(StorageError::InvalidInput(
            "item id contains unsupported characters".into(),
        ));
    }
    Ok(id)
}

fn sanitize_file_name(value: &str) -> Result<String, StorageError> {
    let mut name = value
        .trim()
        .chars()
        .map(|character| match character {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            character if character.is_control() => '_',
            character => character,
        })
        .collect::<String>();

    while name.ends_with('.') || name.ends_with(' ') {
        name.pop();
    }
    if name.is_empty() || name == "." || name == ".." {
        return Err(StorageError::InvalidInput(
            "fileName must contain a usable file name".into(),
        ));
    }

    let mut name = name.chars().take(128).collect::<String>();
    if name.starts_with('.') {
        name.insert(0, '_');
    }
    let stem = name.split('.').next().unwrap_or_default().to_ascii_uppercase();
    if matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "COM1" | "COM2" | "COM3" | "COM4"
            | "COM5" | "COM6" | "COM7" | "COM8" | "COM9" | "LPT1" | "LPT2" | "LPT3"
            | "LPT4" | "LPT5" | "LPT6" | "LPT7" | "LPT8" | "LPT9"
    ) {
        name.insert(0, '_');
    }
    Ok(name)
}

fn normalize_mime_type(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn normalize_file_kind(
    kind: Option<&str>,
    mime_type: Option<&str>,
    file_name: &str,
) -> Result<String, StorageError> {
    if let Some(kind) = kind.map(str::trim).filter(|kind| !kind.is_empty()) {
        return match kind.to_ascii_lowercase().as_str() {
            "image" => Ok("image".into()),
            "pdf" => Ok("pdf".into()),
            "video" => Ok("video".into()),
            "file" | "other" => Ok("file".into()),
            _ => Err(StorageError::InvalidInput(
                "kind must be image, pdf, video, or file".into(),
            )),
        };
    }

    let extension = file_name
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase());
    if mime_type.is_some_and(|mime| mime.starts_with("image/"))
        || extension.as_deref().is_some_and(is_image_extension)
    {
        return Ok("image".into());
    }
    if mime_type == Some("application/pdf") || extension.as_deref() == Some("pdf") {
        return Ok("pdf".into());
    }
    if mime_type.is_some_and(|mime| mime.starts_with("video/"))
        || extension.as_deref().is_some_and(is_video_extension)
    {
        return Ok("video".into());
    }
    Ok("file".into())
}

fn is_image_extension(extension: &str) -> bool {
    matches!(extension, "avif" | "bmp" | "gif" | "ico" | "jpeg" | "jpg" | "png" | "tif" | "tiff" | "webp")
}

fn is_video_extension(extension: &str) -> bool {
    matches!(extension, "avi" | "m4v" | "mkv" | "mov" | "mp4" | "mpeg" | "webm" | "wmv")
}

fn make_image_thumbnail(bytes: &[u8]) -> Result<Vec<u8>, StorageError> {
    let mut reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_DIMENSION);
    limits.max_image_height = Some(MAX_IMAGE_DIMENSION);
    limits.max_alloc = Some(MAX_IMAGE_ALLOC_BYTES);
    reader.limits(limits);
    let image = reader.decode()?;
    let thumbnail = image.thumbnail(THUMBNAIL_EDGE, THUMBNAIL_EDGE);
    let mut output = Cursor::new(Vec::new());
    thumbnail.write_to(&mut output, ImageFormat::WebP)?;
    Ok(output.into_inner())
}

fn file_metadata(file_name: &str, mime_type: Option<&str>, byte_length: usize) -> Value {
    let mut metadata = Map::new();
    metadata.insert("fileName".into(), Value::String(file_name.to_owned()));
    metadata.insert(
        "mimeType".into(),
        mime_type
            .map(|mime| Value::String(mime.to_owned()))
            .unwrap_or(Value::Null),
    );
    metadata.insert(
        "byteLength".into(),
        Value::Number(serde_json::Number::from(byte_length as u64)),
    );
    Value::Object(metadata)
}

fn relative_asset_path(path: &std::path::Path, assets_root: &std::path::Path) -> Result<String, StorageError> {
    let relative = path.strip_prefix(assets_root).map_err(|_| {
        StorageError::InvalidInput("managed asset path escaped the assets directory".into())
    })?;
    Ok(PathBuf::from("assets")
        .join(relative)
        .to_string_lossy()
        .replace('\\', "/"))
}

fn bool_to_int(value: bool) -> i64 {
    i64::from(value)
}

fn now_millis() -> Result<i64, StorageError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| StorageError::InvalidInput(format!("system clock before Unix epoch: {error}")))?;
    i64::try_from(duration.as_millis())
        .map_err(|_| StorageError::InvalidInput("system timestamp exceeds SQLite integer range".into()))
}
