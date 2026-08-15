use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    OcrImage,
    OcrPdfPage,
    GenerateEmbedding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobDto {
    pub id: String,
    pub item_id: String,
    pub kind: JobKind,
    pub status: JobStatus,
    pub retry_count: i64,
    pub max_retries: i64,
    pub error_message: Option<String>,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
}

pub const JOBS_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS jobs (
    id TEXT PRIMARY KEY NOT NULL,
    item_id TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    retry_count INTEGER NOT NULL DEFAULT 0,
    max_retries INTEGER NOT NULL DEFAULT 3,
    error_message TEXT,
    created_at INTEGER NOT NULL,
    started_at INTEGER,
    completed_at INTEGER
);

CREATE INDEX IF NOT EXISTS idx_jobs_status_created
    ON jobs (status, created_at);
CREATE INDEX IF NOT EXISTS idx_jobs_item
    ON jobs (item_id, created_at DESC);
"#;

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn kind_to_str(kind: JobKind) -> &'static str {
    match kind {
        JobKind::OcrImage => "ocr_image",
        JobKind::OcrPdfPage => "ocr_pdf_page",
        JobKind::GenerateEmbedding => "generate_embedding",
    }
}

fn str_to_kind(s: &str) -> JobKind {
    match s {
        "ocr_image" => JobKind::OcrImage,
        "ocr_pdf_page" => JobKind::OcrPdfPage,
        "generate_embedding" => JobKind::GenerateEmbedding,
        _ => JobKind::OcrImage,
    }
}

fn str_to_status(s: &str) -> JobStatus {
    match s {
        "pending" => JobStatus::Pending,
        "processing" => JobStatus::Processing,
        "completed" => JobStatus::Completed,
        "failed" => JobStatus::Failed,
        _ => JobStatus::Pending,
    }
}

fn job_from_row(row: &Row<'_>) -> rusqlite::Result<JobDto> {
    Ok(JobDto {
        id: row.get(0)?,
        item_id: row.get(1)?,
        kind: str_to_kind(&row.get::<_, String>(2)?),
        status: str_to_status(&row.get::<_, String>(3)?),
        retry_count: row.get(4)?,
        max_retries: row.get(5)?,
        error_message: row.get(6)?,
        created_at: row.get(7)?,
        started_at: row.get(8)?,
        completed_at: row.get(9)?,
    })
}

pub struct JobQueue;

impl JobQueue {
    pub fn enqueue_job(
        conn: &Connection,
        item_id: &str,
        kind: JobKind,
    ) -> rusqlite::Result<String> {
        let kind_name = kind_to_str(kind);
        let existing = conn
            .query_row(
                "SELECT id FROM jobs
                 WHERE item_id = ?1 AND kind = ?2 AND status IN ('pending', 'processing')
                 ORDER BY created_at DESC LIMIT 1",
                params![item_id, kind_name],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if let Some(id) = existing {
            return Ok(id);
        }

        let id = Uuid::new_v4().to_string();
        let now = now_millis();
        conn.execute(
            "INSERT INTO jobs (id, item_id, kind, status, created_at)
             VALUES (?1, ?2, ?3, 'pending', ?4)",
            params![id, item_id, kind_name, now],
        )?;
        Ok(id)
    }

    pub fn claim_next_job(conn: &Connection) -> rusqlite::Result<Option<JobDto>> {
        let tx = conn.unchecked_transaction()?;
        let job = tx
            .query_row(
                "SELECT id, item_id, kind, status, retry_count, max_retries,
                        error_message, created_at, started_at, completed_at
                 FROM jobs
                 WHERE status = 'pending'
                 ORDER BY created_at ASC
                 LIMIT 1",
                [],
                job_from_row,
            )
            .optional()?;

        if let Some(ref job) = job {
            let now = now_millis();
            tx.execute(
                "UPDATE jobs SET status = 'processing', started_at = ?1
                 WHERE id = ?2",
                params![now, job.id],
            )?;
        }
        tx.commit()?;
        Ok(job)
    }

    pub fn complete_job(conn: &Connection, job_id: &str) -> rusqlite::Result<()> {
        let now = now_millis();
        conn.execute(
            "UPDATE jobs SET status = 'completed', completed_at = ?1, started_at = COALESCE(started_at, ?1)
             WHERE id = ?2",
            params![now, job_id],
        )?;
        Ok(())
    }

    pub fn fail_job(conn: &Connection, job_id: &str, error: &str) -> rusqlite::Result<bool> {
        let now = now_millis();
        let updated = conn.execute(
            "UPDATE jobs SET error_message = ?1, started_at = COALESCE(started_at, ?2)
             WHERE id = ?3 AND retry_count < max_retries",
            params![error, now, job_id],
        )?;

        if updated == 0 {
            conn.execute(
                "UPDATE jobs SET status = 'failed', completed_at = ?1, error_message = ?2
                 WHERE id = ?3",
                params![now, error, job_id],
            )?;
            return Ok(false);
        }

        conn.execute(
            "UPDATE jobs SET status = 'pending', retry_count = retry_count + 1
             WHERE id = ?1",
            params![job_id],
        )?;
        Ok(true)
    }

    pub fn get_jobs_for_item(
        conn: &Connection,
        item_id: &str,
    ) -> rusqlite::Result<Vec<JobDto>> {
        let mut stmt = conn.prepare(
            "SELECT id, item_id, kind, status, retry_count, max_retries,
                    error_message, created_at, started_at, completed_at
             FROM jobs WHERE item_id = ?1 ORDER BY created_at DESC",
        )?;
        let jobs = stmt
            .query_map(params![item_id], job_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(jobs)
    }

    /// Requeue jobs left in `processing` after an interrupted worker run.
    ///
    /// This is safe to call before each worker pass because the worker claims
    /// jobs only after recovery has completed, so an active job cannot be
    /// mistaken for an interrupted one by the same worker.
    pub fn recover_processing_jobs(conn: &Connection) -> rusqlite::Result<usize> {
        conn.execute(
            "UPDATE jobs
             SET status = 'pending', started_at = NULL, error_message = NULL
             WHERE status = 'processing'",
            [],
        )
    }

    pub fn count_active_jobs(conn: &Connection) -> rusqlite::Result<i64> {
        conn.query_row(
            "SELECT COUNT(*) FROM jobs WHERE status IN ('pending', 'processing')",
            [],
            |row| row.get(0),
        )
    }
}

#[derive(Default)]
pub struct ProcessingState {
    pub wake_tx: std::sync::Mutex<Option<Sender<()>>>,
    pub database_path: std::sync::Mutex<Option<PathBuf>>,
    pub worker_handle: std::sync::Mutex<Option<thread::JoinHandle<()>>>,
}

impl ProcessingState {
    pub fn set_database_path(&self, path: PathBuf) {
        let mut guard = self.database_path.lock().unwrap();
        *guard = Some(path);
        let path = guard.clone().unwrap();
        drop(guard);
        self.start_worker_if_needed(&path);
    }

    fn start_worker_if_needed(&self, db_path: &PathBuf) {
        let mut handle_guard = self.worker_handle.lock().unwrap();
        if handle_guard.is_some() {
            return;
        }

        let (tx, rx) = mpsc::channel::<()>();
        *self.wake_tx.lock().unwrap() = Some(tx);

        let db_path = db_path.to_path_buf();
        let handle = thread::Builder::new()
            .name("job-worker".into())
            .spawn(move || worker_loop(&db_path, rx))
            .expect("failed to spawn job worker thread");
        *handle_guard = Some(handle);
    }

    pub fn enqueue_and_wake(&self, _item_id: &str, _kind: JobKind) {
        if let Some(tx) = self.wake_tx.lock().unwrap().as_ref() {
            let _ = tx.send(());
        }
    }

}

fn worker_loop(db_path: &PathBuf, rx: Receiver<()>) {
    loop {
        match rx.recv_timeout(Duration::from_secs(5)) {
            Ok(()) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }

        process_pending_jobs(db_path);
    }
}

fn process_pending_jobs(db_path: &PathBuf) {
    let storage = match crate::storage::LibraryStorage::open(db_path.clone()) {
        Ok(s) => s,
        Err(_) => return,
    };

    let conn = &storage.connection;

    if JobQueue::recover_processing_jobs(conn).is_err() {
        return;
    }

    loop {
        let job = match JobQueue::claim_next_job(conn) {
            Ok(Some(job)) => job,
            _ => break,
        };

        process_job(&storage, job);
    }
}

fn process_job(storage: &crate::storage::LibraryStorage, job: JobDto) {
    if job.kind != JobKind::OcrImage {
        let _ = JobQueue::fail_job(
            &storage.connection,
            &job.id,
            "this processing job is not implemented yet",
        );
        return;
    }

    let item = match storage.get_item(&job.item_id) {
        Ok(Some(item)) => item,
        _ => {
            let _ = JobQueue::fail_job(&storage.connection, &job.id, "item not found");
            return;
        }
    };

    let asset_path = match &item.local_asset_path {
        Some(p) => p,
        None => {
            let _ = JobQueue::fail_job(&storage.connection, &job.id, "item has no local asset path");
            return;
        }
    };

    let resolved = match storage.resolve_asset_path(asset_path) {
        Ok(p) => p,
        Err(e) => {
            let _ = JobQueue::fail_job(&storage.connection, &job.id, &format!("cannot resolve asset: {e}"));
            return;
        }
    };

    if !std::path::Path::new(&resolved).exists() {
        let _ = JobQueue::fail_job(&storage.connection, &job.id, "asset file not found");
        return;
    }

    let bytes = match std::fs::read(&resolved) {
        Ok(b) => b,
        Err(e) => {
            let _ = JobQueue::fail_job(&storage.connection, &job.id, &format!("cannot read asset: {e}"));
            return;
        }
    };

    let backend = crate::ocr::create_ocr_backend();
    match backend.extract_text(&bytes) {
        Ok(Some(text)) => {
            match storage.update_item_ocr_text(&item.id, &text, backend.name()) {
                Ok(()) => {
                    let _ = JobQueue::complete_job(&storage.connection, &job.id);
                }
                Err(error) => {
                    let _ = JobQueue::fail_job(
                        &storage.connection,
                        &job.id,
                        &format!("cannot store OCR text: {error}"),
                    );
                }
            }
        }
        Ok(None) => {
            let _ = JobQueue::complete_job(&storage.connection, &job.id);
        }
        Err(e) => {
            let _ = JobQueue::fail_job(&storage.connection, &job.id, &format!("{e}"));
        }
    }
}

pub(crate) fn enqueue_ocr_for_item(
    conn: &Connection,
    item_id: &str,
    item_kind: &str,
) -> rusqlite::Result<Option<String>> {
    if item_kind != "image" {
        return Ok(None);
    }

    JobQueue::enqueue_job(conn, item_id, JobKind::OcrImage).map(Some)
}

#[tauri::command]
pub fn enqueue_ocr_job(
    item_id: String,
    state: tauri::State<'_, ProcessingState>,
    storage_state: tauri::State<'_, crate::storage::StorageState>,
) -> Result<String, String> {
    let guard = storage_state.lock().map_err(|_| "storage lock poisoned".to_string())?;
    let storage = guard
        .as_ref()
        .ok_or_else(|| "storage not initialized".to_string())?;
    let item = storage
        .get_item(&item_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "item not found".to_string())?;
    let job_id = enqueue_ocr_for_item(&storage.connection, &item_id, &item.kind)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "OCR is only available for image items".to_string())?;

    state.enqueue_and_wake(&item_id, JobKind::OcrImage);
    Ok(job_id)
}

#[tauri::command]
pub fn get_job_status(
    item_id: String,
    storage_state: tauri::State<'_, crate::storage::StorageState>,
) -> Result<Vec<JobDto>, String> {
    let guard = storage_state.lock().map_err(|_| "storage lock poisoned".to_string())?;
    let storage = guard
        .as_ref()
        .ok_or_else(|| "storage not initialized".to_string())?;
    JobQueue::get_jobs_for_item(&storage.connection, &item_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn count_active_jobs(
    storage_state: tauri::State<'_, crate::storage::StorageState>,
) -> Result<i64, String> {
    let guard = storage_state.lock().map_err(|_| "storage lock poisoned".to_string())?;
    let storage = guard
        .as_ref()
        .ok_or_else(|| "storage not initialized".to_string())?;
    JobQueue::count_active_jobs(&storage.connection).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_connection() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE items (id TEXT PRIMARY KEY NOT NULL);\n                 CREATE TABLE jobs (\n                     id TEXT PRIMARY KEY NOT NULL,\n                     item_id TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,\n                     kind TEXT NOT NULL,\n                     status TEXT NOT NULL DEFAULT 'pending',\n                     retry_count INTEGER NOT NULL DEFAULT 0,\n                     max_retries INTEGER NOT NULL DEFAULT 3,\n                     error_message TEXT,\n                     created_at INTEGER NOT NULL,\n                     started_at INTEGER,\n                     completed_at INTEGER\n                 );",
            )
            .unwrap();
        connection
    }

    #[test]
    fn recovers_interrupted_jobs_and_preserves_deduplication() {
        let connection = test_connection();
        connection
            .execute("INSERT INTO items (id) VALUES ('item-1')", [])
            .unwrap();
        connection
            .execute(
                "INSERT INTO jobs (
                    id, item_id, kind, status, error_message, created_at, started_at
                 ) VALUES ('job-1', 'item-1', 'ocr_image', 'processing', 'old error', 1, 2)",
                [],
            )
            .unwrap();

        assert_eq!(JobQueue::recover_processing_jobs(&connection).unwrap(), 1);
        let jobs = JobQueue::get_jobs_for_item(&connection, "item-1").unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].status, JobStatus::Pending);
        assert_eq!(jobs[0].started_at, None);
        assert_eq!(jobs[0].error_message, None);
        assert_eq!(JobQueue::count_active_jobs(&connection).unwrap(), 1);

        assert_eq!(
            JobQueue::enqueue_job(&connection, "item-1", JobKind::OcrImage).unwrap(),
            "job-1"
        );
        assert_eq!(
            JobQueue::claim_next_job(&connection).unwrap().unwrap().status,
            JobStatus::Processing
        );
    }
}
