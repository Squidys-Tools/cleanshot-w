use std::{
    fmt,
    fs,
    path::PathBuf,
    sync::{Mutex, MutexGuard},
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tauri::{AppHandle, Manager, State};
use uuid::Uuid;

const SCHEMA_VERSION: i64 = 1;

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
