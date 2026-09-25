//! Глобальное состояние приложения: панели-операции, токены отмены, БД.
//! Global app state: running ops, cancel tokens, DB handle.
//! Сюда кладём то, что должно жить между вызовами команд.
//! Anything that must survive between command calls goes here.
//! register_op/unregister_op — пара для учёта долгих операций.
//! register_op/unregister_op — the bookkeeping pair for long operations.

use crate::types::ConflictChoice;
use notify::RecommendedWatcher;
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{mpsc, Arc, Mutex};
use tauri::{AppHandle, Manager};

#[derive(Clone)]
pub struct CancelToken(pub Arc<AtomicBool>);

impl Default for CancelToken {
    fn default() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }
}

impl CancelToken {
    pub fn cancelled(&self) -> bool {
        self.0.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn cancel(&self) {
        self.0.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Registry-Eintrag einer laufenden/verwalteten Operation.
pub struct OpHandle {
    pub cancel: Arc<AtomicBool>,
    pub kind: String,
    pub created_at_ms: i64,
}

/// Eine portable-pty-Terminalsitzung.
pub struct TerminalSession {
    pub master: Mutex<Box<dyn portable_pty::MasterPty + Send>>,
    pub writer: Mutex<Box<dyn std::io::Write + Send>>,
    pub child: Mutex<Box<dyn portable_pty::Child + Send + Sync>>,
    pub quit: Arc<AtomicBool>,
}

pub struct AppState {
    pub app: AppHandle,
    /// App-Datenverzeichnis (SQLite-Index).
    pub data_dir: PathBuf,
    /// App-Cacheverzeichnis (Thumbnails).
    pub cache_dir: PathBuf,
    /// SQLite-Verbindung (Index/Dedup/Suche).
    pub db: Mutex<Option<Connection>>,
    /// Идущие операции (op_id -> Handle) для отмены.
    /// Running operations (op_id -> handle), used for cancel.
    pub ops: Mutex<HashMap<String, OpHandle>>,
    /// Очередь ответов по конфликтам (op_id:key -> Sender).
    /// Queue of conflict answers (op_id:key -> sender).
    pub conflict_waiters: Mutex<HashMap<String, mpsc::Sender<ConflictChoice>>>,
    /// Indizierung läuft bereits (globale Sperre).
    pub indexing: Mutex<()>,
    /// Вотчер каталогов для авто-обновления (в v1.0 не используется, резерв).
    /// Directory watcher for auto-refresh (unused in v1.0, reserved).
    #[allow(dead_code)]
    pub watcher: Mutex<Option<RecommendedWatcher>>,
    /// Terminal-Sitzungen (id -> Session).
    pub terminals: Mutex<HashMap<String, TerminalSession>>,
}

impl AppState {
    pub fn new(app: AppHandle) -> Result<Self, Box<dyn std::error::Error>> {
        let data_dir = app.path().app_data_dir()?;
        let cache_dir = app.path().app_cache_dir()?;
        std::fs::create_dir_all(&data_dir)?;
        std::fs::create_dir_all(cache_dir.join("thumbs"))?;
        Ok(Self {
            app,
            data_dir,
            cache_dir,
            db: Mutex::new(None),
            ops: Mutex::new(HashMap::new()),
            conflict_waiters: Mutex::new(HashMap::new()),
            indexing: Mutex::new(()),
            watcher: Mutex::new(None),
            terminals: Mutex::new(HashMap::new()),
        })
    }

    /// SQLite-Verbindung holen (legt sie bei Bedarf an).
    pub fn conn(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, Option<Connection>>, crate::ui_error::Error> {
        let mut guard = self
            .db
            .lock()
            .map_err(|_| crate::ui_error::lock())?;
        if guard.is_none() {
            let conn = open_db(&self.data_dir)?;
            *guard = Some(conn);
        }
        Ok(guard)
    }
}

fn open_db(data_dir: &std::path::Path) -> Result<Connection, crate::ui_error::Error> {
    let conn = Connection::open(data_dir.join("index.db")).map_err(crate::ui_error::db)?;
    conn.busy_timeout(std::time::Duration::from_secs(10)).ok();
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA synchronous=NORMAL;
         PRAGMA cache_size=-16000;
         CREATE TABLE IF NOT EXISTS files(
             path TEXT PRIMARY KEY,
             name TEXT NOT NULL,
             parent TEXT NOT NULL,
             is_file INTEGER NOT NULL,
             size INTEGER NOT NULL DEFAULT 0,
             mtime INTEGER NOT NULL DEFAULT 0,
             kind TEXT NOT NULL DEFAULT 'other'
         );
         CREATE INDEX IF NOT EXISTS idx_files_parent ON files(parent);
         CREATE VIRTUAL TABLE IF NOT EXISTS filetext USING fts5(path UNINDEXED, content, tokenize='unicode61');
         CREATE TABLE IF NOT EXISTS roots(
             root TEXT PRIMARY KEY,
             indexed_at INTEGER NOT NULL,
             file_count INTEGER NOT NULL DEFAULT 0
         );",
    )
    .map_err(crate::ui_error::db)?;
    Ok(conn)
}

// Регистрируем новую операцию: отдаём id + токен отмены.
// Register a new op: hand back its id + cancel token.
pub fn register_op(state: &AppState, kind: &str) -> (String, CancelToken) {
    let id = uuid::Uuid::new_v4().to_string();
    let token = CancelToken::default();
    if let Ok(mut ops) = state.ops.lock() {
        ops.insert(
            id.clone(),
            OpHandle {
                cancel: token.0.clone(),
                kind: kind.into(),
                created_at_ms: now_ms(),
            },
        );
    }
    (id, token)
}

pub fn unregister_op(state: &AppState, id: &str) {
    if let Ok(mut ops) = state.ops.lock() {
        ops.remove(id);
    }
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}