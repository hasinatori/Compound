// Операции над файлами: копировать, переместить, создать, переименовать, корзина.
// File operations: copy, move, create, rename, trash.
// Долгое = с register_op + CancelToken, прогресс через Channel.
// Long ops use register_op + CancelToken, progress over a Channel.
// Порядок функций = пользовательский поток в UI.
// Function order mirrors the user flow in the UI.

use crate::fsutil;
use crate::state::{register_op, unregister_op, AppState, CancelToken};
use crate::types::*;
use crate::ui_error::{self, Error};
use serde::Serialize;
use std::fs;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, Manager, State};

const CHUNK: usize = 1024 * 1024;

fn conflict_waiter_insert(
    app: &AppHandle,
    op_id: &str,
    key: &str,
    tx: mpsc::Sender<ConflictChoice>,
) {
    if let Ok(mut w) = app.state::<AppState>().conflict_waiters.lock() {
        w.insert(format!("{}:{}", op_id, key), tx);
    }
}

fn conflict_waiter_remove(app: &AppHandle, op_id: &str, key: &str) {
    if let Ok(mut w) = app.state::<AppState>().conflict_waiters.lock() {
        w.remove(&format!("{}:{}", op_id, key));
    }
}

/// Kleines Fortschritts-Limit: Emits gebündelt (max. ~12/s).
fn throttle_ok(last: &mut Instant) -> bool {
    if last.elapsed() >= Duration::from_millis(80) {
        *last = Instant::now();
        true
    } else {
        false
    }
}

/// Führend bei Kopier-/Verschiebe-Operationen.
struct OpRunner {
    op_id: String,
    op_name: &'static str,
    app: Option<AppHandle>,
    cancel: CancelToken,
    policy: String,
    progress: Channel<ProgressEvent>,
    conflicts: Channel<ConflictRequest>,
    total_bytes: u64,
    bytes_done: Arc<AtomicU64>,
    files_done: Arc<AtomicU64>,
    total_items: u64,
    conflict_index: u64,
    last_emit: Instant,
}

impl OpRunner {
    fn new(
        app: AppHandle,
        op_id: String,
        cancel: CancelToken,
        policy: String,
        progress: Channel<ProgressEvent>,
        conflicts: Channel<ConflictRequest>,
        op_name: &'static str,
        total_bytes: u64,
        total_items: u64,
    ) -> Self {
        Self {
            op_id,
            op_name,
            app: Some(app),
            cancel,
            policy,
            progress,
            conflicts,
            total_bytes,
            bytes_done: Arc::new(AtomicU64::new(0)),
            files_done: Arc::new(AtomicU64::new(0)),
            total_items,
            conflict_index: 0,
            last_emit: Instant::now(),
        }
    }

    /// Fortschritts-Ereignis senden (Maschinen-Token + params, keine Texte).
    fn emit(&self, phase: &str, kind: &str, params: std::collections::BTreeMap<String, String>) {
        let current = self.bytes_done.load(Ordering::Relaxed);
        let total = self.total_bytes.max(current);
        let _ = self.progress.send(ProgressEvent::new(
            self.op_name,
            phase,
            current,
            total,
            kind,
            params,
        ));
    }

    /// Konflikt bei existierendem Ziel entscheiden.
    fn resolve(&mut self, src: &str, dest: &str, is_dir: bool) -> String {
        if self.policy != "ask" {
            return self.policy.clone();
        }
        let app = match &self.app {
            Some(a) => a.clone(),
            None => return self.policy.clone(),
        };
        let key = format!("c{}", self.conflict_index);
        self.conflict_index += 1;
        let (tx, rx) = mpsc::channel::<ConflictChoice>();
        conflict_waiter_insert(&app, &self.op_id, &key, tx);
        let _ = self.conflicts.send(ConflictRequest {
            op_id: self.op_id.clone(),
            key: key.clone(),
            src: src.into(),
            dest: dest.into(),
            is_dir,
            index: self.files_done.load(Ordering::Relaxed),
            total: self.total_items,
        });
        let choice = rx.recv_timeout(Duration::from_secs(300)).ok();
        conflict_waiter_remove(&app, &self.op_id, &key);
        match choice {
            Some(c) => {
                if c.apply_to_all {
                    self.policy = c.action.clone();
                }
                c.action
            }
            None => "skip".into(),
        }
    }

    fn copy_file(&mut self, src: &Path, dest: &Path) -> Result<(), Error> {
        let mut reader = BufReader::new(
            fs::File::open(src).map_err(|e| ui_error::io(src.display(), e))?,
        );
        let mut writer = BufWriter::new(
            fs::File::create(dest).map_err(|e| ui_error::io(dest.display(), e))?,
        );
        let mut buf = vec![0u8; CHUNK];
        loop {
            if self.cancel.cancelled() {
                return Err(Error::new("canceled", "Abgebrochen"));
            }
            let n = reader
                .read(&mut buf)
                .map_err(|e| ui_error::io(src.display(), e))?;
            if n == 0 {
                break;
            }
            writer
                .write_all(&buf[..n])
                .map_err(|e| ui_error::io(dest.display(), e))?;
            self.bytes_done.fetch_add(n as u64, Ordering::Relaxed);
            if throttle_ok(&mut self.last_emit) {
                self.emit("copy", "path", ui_error::par("path", src.display()));
            }
        }
        writer
            .flush()
            .map_err(|e| ui_error::io(dest.display(), e))?;
        // Rechte übernehmen (Linux/Unix).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perm = fs::metadata(src)
                .map_err(|e| ui_error::io(src.display(), e))?
                .permissions();
            let mode = perm.mode();
            fs::set_permissions(dest, fs::Permissions::from_mode(mode))
                .map_err(|e| ui_error::io(dest.display(), e))?;
        }
        Ok(())
    }

    fn copy_link(&mut self, src: &Path, dest: &Path) -> Result<(), Error> {
        let target = fs::read_link(src)
            .map_err(|e| ui_error::io(src.display(), e))?;
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, dest).map_err(|e| ui_error::io(dest.display(), e))?;
        #[cfg(windows)]
        {
            if fs::metadata(src).map(|m| m.is_dir()).unwrap_or(false) {
                std::os::windows::fs::symlink_dir(&target, dest)
            } else {
                std::os::windows::fs::symlink_file(&target, dest)
            }
            .map_err(|e| ui_error::io(dest.display(), e))?;
        }
        Ok(())
    }

    fn copy_one(&mut self, src: &Path, dest: &Path) -> Result<(), Error> {
        let meta = fs::symlink_metadata(src)
            .map_err(|e| ui_error::io(src.display(), e))?;
        if meta.file_type().is_symlink() {
            return self.copy_link(src, dest);
        }
        if meta.is_dir() {
            fs::create_dir_all(dest)
                .map_err(|e| ui_error::io(dest.display(), e))?;
            let rd = fs::read_dir(src).map_err(|e| ui_error::io(src.display(), e))?;
            for entry in rd.flatten() {
                if self.cancel.cancelled() {
                    return Err(Error::new("canceled", "Abgebrochen"));
                }
                let child = entry.path();
                let name = entry.file_name();
                self.copy_one(&child, &dest.join(&name))?;
                let done = self.files_done.fetch_add(1, Ordering::Relaxed);
                if done == 0 || throttle_ok(&mut self.last_emit) {
                    self.emit("copy", "path", ui_error::par("path", name.to_string_lossy()));
                }
            }
            return Ok(());
        }
        self.copy_file(src, dest)
    }

    fn remove_tree(&mut self, path: &Path) -> Result<(), Error> {
        if self.cancel.cancelled() {
            return Err(Error::new("canceled", "Abgebrochen"));
        }
        let meta = fs::symlink_metadata(path)
            .map_err(|e| ui_error::io(path.display(), e))?;
        if meta.is_dir() && !meta.file_type().is_symlink() {
            for entry in fs::read_dir(path)
                .map_err(|e| ui_error::io(path.display(), e))?
                .flatten()
            {
                self.remove_tree(&entry.path())?;
            }
            fs::remove_dir(path).map_err(|e| ui_error::io(path.display(), e))?;
        } else {
            fs::remove_file(path).map_err(|e| ui_error::io(path.display(), e))?;
        }
        self.files_done.fetch_add(1, Ordering::Relaxed);
        if throttle_ok(&mut self.last_emit) {
            self.emit("delete", "path", ui_error::par("path", path.display()));
        }
        Ok(())
    }

    fn precheck(sources: &[String], dest: &str) -> Result<(), Error> {
        let dest = Path::new(dest);
        if !dest.is_dir() {
            return Err(Error::new("destMissing", "Zielordner existiert nicht."));
        }
        for s in sources {
            let p = Path::new(s);
            if !p.exists() {
                continue;
            }
            // Kopieren in sich selbst verhindern.
            if p == dest || dest.starts_with(p) {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::MetadataExt;
                    if let (Ok(a), Ok(b)) = (p.metadata(), dest.metadata()) {
                        if a.dev() == b.dev() && a.ino() == b.ino() {
                            return Err(Error::new(
                                "selfCopy",
                                format!(
                                    "„{}“ kann nicht in sich selbst kopiert werden.",
                                    p.display()
                                ),
                            )
                            .with("path", p.display()));
                        }
                    }
                }
                if p == dest {
                    return Err(Error::new("identicalDest", "Quelle und Ziel sind identisch."));
                }
            }
        }
        Ok(())
    }

    fn run_copy(&mut self, srcs: &[String], dest_dir: &str) -> Result<(), Error> {
        Self::precheck(srcs, dest_dir)?;
        let dest = Path::new(dest_dir);
        for src_s in srcs {
            if self.cancel.cancelled() {
                return Err(Error::new("canceled", "Abgebrochen"));
            }
            let src = Path::new(src_s);
            if !src.exists() {
                self.emit(
                    "info",
                    "skipMissing",
                    ui_error::par("name", src.display()),
                );
                continue;
            }
            let name = src
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| src_s.clone());
            let mut target = dest.join(&name);
            if target.exists() {
                match self.resolve(src_s, &target.display().to_string(), src.is_dir()).as_str() {
                    "overwrite" => {}
                    "keep_both" => {
                        target = fsutil::unique_dest(dest, &name);
                    }
                    "skip" => {
                        self.emit("info", "skip", ui_error::par("name", &name));
                        continue;
                    }
                    _ => return Err(Error::new("canceled", "Abgebrochen")),
                }
            }
            self.emit(
                "copy",
                "target",
                ui_error::par("name", &name).with_chain(dest),
            );
            self.copy_one(src, &target)?;
        }
        Ok(())
    }

    fn run_move(&mut self, srcs: &[String], dest_dir: &str) -> Result<(), Error> {
        let dest = Path::new(dest_dir);
        if !dest.is_dir() {
            return Err(Error::new("destMissing", "Zielordner existiert nicht."));
        }
        for src_s in srcs {
            if self.cancel.cancelled() {
                return Err(Error::new("canceled", "Abgebrochen"));
            }
            let src = Path::new(src_s);
            if !src.exists() {
                continue;
            }
            if src == dest {
                continue;
            }
            let name = src
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| src_s.clone());
            let mut target = dest.join(&name);
            if target.exists() {
                match self.resolve(src_s, &target.display().to_string(), src.is_dir()).as_str() {
                    "overwrite" => {
                        if fs::remove_file(&target).is_err() && target.is_dir() {
                            let _ = self.remove_tree(&target);
                        }
                    }
                    "keep_both" => {
                        target = fsutil::unique_dest(dest, &name);
                    }
                    "skip" => {
                        self.emit("info", "skip", ui_error::par("name", &name));
                        continue;
                    }
                    _ => return Err(Error::new("canceled", "Abgebrochen")),
                }
            }
            // Schneller Pfad: gleiches Dateisystem.
            if fs::rename(&src, &target).is_ok() {
                self.emit(
                    "move",
                    "target",
                    ui_error::par("name", &name).with_chain(dest),
                );
            } else {
                // Cross-Device: kopieren + löschen.
                self.emit("move", "copying", ui_error::par("name", &name));
                self.copy_one(src, &target)?;
                let _ = self.remove_tree(src);
            }
        }
        Ok(())
    }

    fn run_delete(&mut self, srcs: &[String]) -> Result<(), Error> {
        for src_s in srcs {
            if self.cancel.cancelled() {
                return Err(Error::new("canceled", "Abgebrochen"));
            }
            let src = Path::new(src_s);
            if !src.exists() {
                continue;
            }
            self.emit("delete", "path", ui_error::par("path", src.display()));
            self.remove_tree(src)?;
        }
        Ok(())
    }
}

/// Собирает `{name, dest}` для target-событий.
/// Builds the `{name, dest}` params for target events.
trait WithDest {
    fn with_chain(self, dest: &Path) -> std::collections::BTreeMap<String, String>;
}

impl WithDest for std::collections::BTreeMap<String, String> {
    fn with_chain(self, dest: &Path) -> std::collections::BTreeMap<String, String> {
        let mut m = self;
        m.insert("dest".into(), dest.display().to_string());
        m
    }
}

fn run_blocking<R, F>(app: AppHandle, op_id: String, f: F)
where
    R: Send + 'static,
    F: FnOnce() -> Result<R, Error> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || {
        let res = f();
        let ok = res.is_ok();
        let error = res.err();
        let _ = app.emit(
            "op-finished",
            OpFinished {
                op_id: op_id.clone(),
                ok,
                error,
            },
        );
        let state = app.state::<AppState>();
        unregister_op(&state, &op_id);
    });
}

/// Сумма размеров (для текста прогресса).
/// Total size (for the progress text).
fn total_size(srcs: &[String]) -> u64 {
    let mut total = 0u64;
    for s in srcs {
        let p = Path::new(s);
        if let Ok(meta) = fs::metadata(p) {
            if meta.is_file() {
                total += meta.len();
            }
        }
    }
    total.max(1)
}

#[tauri::command]
pub async fn copy_items(
    app: AppHandle,
    state: State<'_, AppState>,
    srcs: Vec<String>,
    dest_dir: String,
    policy: String,
    progress: Channel<ProgressEvent>,
    conflicts: Channel<ConflictRequest>,
) -> Result<OpStarted, Error> {
    let policy = if policy.is_empty() { "ask".into() } else { policy };
    let (op_id, cancel) = register_op(&state, "op.copy");
    let total = total_size(&srcs);
    let total_items = srcs.len() as u64;
    let mut runner = OpRunner::new(
        app.clone(),
        op_id.clone(),
        cancel,
        policy,
        progress,
        conflicts,
        "copy",
        total,
        total_items,
    );
    run_blocking(app, op_id.clone(), move || {
        runner.run_copy(&srcs, &dest_dir)
    });
    Ok(OpStarted {
        op_id,
        kind: "op.copy".into(),
        total,
    })
}

#[tauri::command]
pub async fn move_items(
    app: AppHandle,
    state: State<'_, AppState>,
    srcs: Vec<String>,
    dest_dir: String,
    policy: String,
    progress: Channel<ProgressEvent>,
    conflicts: Channel<ConflictRequest>,
) -> Result<OpStarted, Error> {
    let policy = if policy.is_empty() { "ask".into() } else { policy };
    let (op_id, cancel) = register_op(&state, "op.move");
    let total = total_size(&srcs);
    let mut runner = OpRunner::new(
        app.clone(),
        op_id.clone(),
        cancel,
        policy,
        progress,
        conflicts,
        "move",
        total,
        srcs.len() as u64,
    );
    run_blocking(app, op_id.clone(), move || {
        runner.run_move(&srcs, &dest_dir)
    });
    Ok(OpStarted {
        op_id,
        kind: "op.move".into(),
        total,
    })
}

#[tauri::command]
pub async fn delete_permanent(
    app: AppHandle,
    state: State<'_, AppState>,
    srcs: Vec<String>,
    progress: Channel<ProgressEvent>,
) -> Result<OpStarted, Error> {
    let (op_id, cancel) = register_op(&state, "op.delete");
    let mut runner = OpRunner::new(
        app.clone(),
        op_id.clone(),
        cancel,
        "overwrite".into(),
        progress,
        Channel::new(|_| Ok(())),
        "delete",
        0,
        0,
    );
    run_blocking(app, op_id.clone(), move || runner.run_delete(&srcs));
    Ok(OpStarted {
        op_id,
        kind: "op.delete".into(),
        total: 0,
    })
}

// --- Управление долгими операциями / long-op control ---
#[tauri::command]
pub fn cancel_op(state: State<'_, AppState>, op_id: String) -> Result<(), Error> {
    let ops = state
        .ops
        .lock()
        .map_err(|_| ui_error::lock())?;
    if let Some(h) = ops.get(&op_id) {
        CancelToken(h.cancel.clone()).cancel();
    }
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpInfo {
    pub op_id: String,
    pub kind: String,
    pub created_at_ms: i64,
}

/// Список идущих операций (окно прогресса при старте приложения).
/// Lists all running ops (for the progress window on app start).
#[tauri::command]
pub fn list_ops(state: State<'_, AppState>) -> Result<Vec<OpInfo>, Error> {
    let ops = state.ops.lock().map_err(|_| ui_error::lock())?;
    let mut out: Vec<OpInfo> = ops
        .iter()
        .map(|(id, h)| OpInfo {
            op_id: id.clone(),
            kind: h.kind.clone(),
            created_at_ms: h.created_at_ms,
        })
        .collect();
    // Jüngste zuerst – UI zeigt laufende Operationen oben.
    out.sort_by(|a, b| b.created_at_ms.cmp(&a.created_at_ms));
    Ok(out)
}

#[tauri::command]
pub fn resolve_conflict(
    state: State<'_, AppState>,
    op_id: String,
    key: String,
    action: String,
    apply_to_all: bool,
) -> Result<(), Error> {
    let waiters = state
        .conflict_waiters
        .lock()
        .map_err(|_| ui_error::lock())?;
    if let Some(tx) = waiters.get(&format!("{op_id}:{key}")) {
        let _ = tx.send(ConflictChoice {
            action,
            apply_to_all,
        });
    }
    Ok(())
}

// --- Создание и переименование / create + rename ---
#[tauri::command]
pub fn create_item(
    state: State<'_, AppState>,
    dest_dir: String,
    name: String,
    is_dir: bool,
) -> Result<FileEntry, Error> {
    let _ = state;
    fsutil::valid_name(&name)?;
    let dest = Path::new(&dest_dir);
    if !dest.is_dir() {
        return Err(Error::new("destMissing", "Zielordner existiert nicht."));
    }
    let target = fsutil::unique_dest(dest, &name);
    if is_dir {
        fs::create_dir(&target).map_err(|e| ui_error::io(target.display(), e))?;
    } else {
        fs::File::create(&target).map_err(|e| ui_error::io(target.display(), e))?;
    }
    let meta = fs::metadata(&target).map_err(|e| ui_error::io(target.display(), e))?;
    let p = target.display().to_string();
    Ok(FileEntry {
        path: p.clone(),
        name: target
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        ext: fsutil::file_ext(
            &target
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
        ),
        is_dir,
        is_symlink: false,
        is_hidden: fsutil::is_hidden_name(&name),
        size: if is_dir { 0 } else { meta.len() },
        mtime_ms: fsutil::mtime_ms(&meta),
        perms: crate::fsutil::perm_string(&meta),
        kind: if is_dir { "dir".into() } else { fsutil::kind_for_ext(&fsutil::file_ext(&name)) },
        link_target: None,
    })
}

#[tauri::command]
pub fn rename_item(path: String, new_name: String) -> Result<String, Error> {
    fsutil::valid_name(&new_name)?;
    let src = Path::new(&path);
    if !src.exists() {
        return Err(Error::new("sourceMissing", "Quelle existiert nicht."));
    }
    let parent = src
        .parent()
        .ok_or_else(|| Error::new("noParent", "Kein übergeordneter Ordner"))?;
    let target = parent.join(&new_name);
    if target != src && target.exists() {
        return Err(Error::new("targetExists", "Zielname existiert bereits."));
    }
    fs::rename(src, &target).map_err(|e| ui_error::io(target.display(), e))?;
    Ok(target.display().to_string())
}

#[tauri::command]
pub fn duplicate_item(path: String) -> Result<String, Error> {
    let src = Path::new(&path);
    if !src.exists() {
        return Err(Error::new("sourceMissing", "Quelle existiert nicht."));
    }
    let parent = src
        .parent()
        .ok_or_else(|| Error::new("noParent", "Kein übergeordneter Ordner"))?;
    let name = src
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.clone());
    let target = fsutil::unique_dest(parent, &name);
    let mut runner = OpRunner::new_duplicate();
    runner.copy_one(src, &target)?;
    Ok(target.display().to_string())
}

impl OpRunner {
    fn new_duplicate() -> Self {
        Self {
            op_id: String::new(),
            op_name: "copy",
            app: None,
            cancel: CancelToken::default(),
            policy: "overwrite".into(),
            progress: Channel::new(|_| Ok(())),
            conflicts: Channel::new(|_| Ok(())),
            total_bytes: 0,
            bytes_done: Arc::new(AtomicU64::new(0)),
            files_done: Arc::new(AtomicU64::new(0)),
            total_items: 0,
            conflict_index: 0,
            last_emit: Instant::now(),
        }
    }
}

// --- Корзина / trash ---
#[tauri::command]
pub fn trash_items(paths: Vec<String>) -> Result<(), Error> {
    let items: Vec<PathBuf> = paths
        .iter()
        .filter(|p| Path::new(p).exists())
        .map(PathBuf::from)
        .collect();
    if items.is_empty() {
        return Ok(());
    }
    trash::delete_all(&items).map_err(|e| {
        let first = items
            .first()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        Error::new("io", format!("Papierkorb: {e}"))
            .with("path", first)
            .with("detail", e.to_string())
    })
}

/// Listet den (XDG) Papierkorb. Linux: ~/.local/share/Trash.
#[tauri::command]
pub fn list_trash() -> Result<Vec<TrashItem>, Error> {
    let trash_dir = trash_dir_path()
        .ok_or_else(|| Error::new("noTrashFound", "Kein Papierkorb gefunden"))?;
    let files_dir = trash_dir.join("files");
    let info_dir = trash_dir.join("info");
    let mut out = Vec::new();
    if !files_dir.is_dir() {
        return Ok(out);
    }
    for entry in fs::read_dir(&files_dir)
        .map_err(|e| ui_error::io(files_dir.display(), e))?
        .flatten()
    {
        let name = entry.file_name().to_string_lossy().into_owned();
        let is_dir = entry
            .metadata()
            .map(|m| m.is_dir())
            .unwrap_or(false);
        let mut original = String::new();
        let mut trashed_at = 0i64;
        let info_path = info_dir.join(format!("{name}.trashinfo"));
        if let Ok(text) = fs::read_to_string(&info_path) {
            for line in text.lines() {
                if let Some(val) = line.strip_prefix("Path=") {
                    original = fsutil::percent_decode(val.trim());
                } else if let Some(val) = line.strip_prefix("DeletionDate=") {
                    if let Some(parsed) = parse_iso_date(val.trim()) {
                        trashed_at = parsed;
                    }
                }
            }
        }
        out.push(TrashItem {
            name_in_trash: name,
            original_path: original,
            trashed_at_ms: trashed_at,
            is_dir,
        });
    }
    Ok(out)
}

fn parse_iso_date(s: &str) -> Option<i64> {
    // YYYY-MM-DDTHH:MM:SS
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() < 8 {
        return None;
    }
    let y: i64 = digits[0..4].parse().ok()?;
    let mo: i64 = digits[4..6].parse().ok()?;
    let d: i64 = digits[6..8].parse().ok()?;
    let h: i64 = digits.get(8..10).and_then(|s| s.parse().ok()).unwrap_or(0);
    let mi: i64 = digits
        .get(10..12)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let se: i64 = digits
        .get(12..14)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let secs = y * 31_536_000 + mo * 2_592_000 + d * 86_400 + h * 3600 + mi * 60 + se;
    Some(secs * 1000)
}

fn trash_dir_path() -> Option<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        if !xdg.is_empty() {
            return Some(PathBuf::from(xdg).join("Trash"));
        }
    }
    dirs::home_dir().map(|h| h.join(".local/share/Trash"))
}

#[tauri::command]
pub fn restore_trash(name_in_trash: String) -> Result<String, Error> {
    let trash_dir = trash_dir_path().ok_or_else(|| Error::new("noTrash", "Kein Papierkorb"))?;
    let files_dir = trash_dir.join("files");
    let info_dir = trash_dir.join("info");
    let src = files_dir.join(&name_in_trash);
    if !src.exists() {
        return Err(Error::new("entryMissing", "Eintrag existiert nicht mehr."));
    }
    let mut original = String::new();
    let info_path = info_dir.join(format!("{name_in_trash}.trashinfo"));
    if let Ok(text) = fs::read_to_string(&info_path) {
        for line in text.lines() {
            if let Some(val) = line.strip_prefix("Path=") {
                original = fsutil::percent_decode(val.trim());
                break;
            }
        }
    }
    let dest = if original.starts_with('/') {
        PathBuf::from(&original)
    } else {
        dirs::home_dir()
            .ok_or_else(|| Error::new("noHome", "Kein Home"))?
            .join(original.trim_start_matches('/'))
    };
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| ui_error::io(parent.display(), e))?;
    }
    let dest = if dest.exists() {
        let name = dest
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "wiederhergestellt".into());
        let unique = fsutil::unique_dest(
            dest.parent().unwrap_or(Path::new("/")),
            &name,
        );
        unique
    } else {
        dest
    };
    fs::rename(&src, &dest).map_err(|e| ui_error::io(dest.display(), e))?;
    let _ = fs::remove_file(&info_path);
    Ok(dest.display().to_string())
}

#[tauri::command]
pub fn empty_trash() -> Result<(), Error> {
    let trash_dir = trash_dir_path().ok_or_else(|| Error::new("noTrash", "Kein Papierkorb"))?;
    for sub in ["files", "info"] {
        let dir = trash_dir.join(sub);
        if dir.is_dir() {
            for entry in fs::read_dir(&dir).map_err(|e| ui_error::io(dir.display(), e))?.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    fs::remove_dir_all(&p).map_err(|e| ui_error::io(p.display(), e))?;
                } else {
                    fs::remove_file(&p).map_err(|e| ui_error::io(p.display(), e))?;
                }
            }
        }
    }
    Ok(())
}

/// Terminal in externer App öffnen (falls vorhanden).
// --- Внешний терминал / external terminal ---
#[tauri::command]
pub fn open_external_terminal(path: String) -> Result<(), Error> {
    let candidates: &[(&str, &[&str])] = &[
        ("konsole", &["--workdir"]),
        ("kitty", &["--directory"]),
        ("alacritty", &["--working-directory"]),
        ("gnome-terminal", &["--working-directory"]),
        ("xfce4-terminal", &["--working-directory"]),
        ("x-terminal-emulator", &["--working-directory"]),
        ("wezterm", &["start", "--cwd"]),
        ("sakura", &["-d"]),
    ];
    for (bin, args) in candidates {
        let full = PathBuf::from(format!("/usr/bin/{bin}"));
        let found = if full.exists() {
            Some(full)
        } else {
            std::env::var("PATH")
                .ok()
                .and_then(|paths| {
                    paths.split(':').find_map(|dir| {
                        let p = Path::new(dir).join(bin);
                        if p.exists() {
                            Some(p)
                        } else {
                            None
                        }
                    })
                })
        };
        if let Some(bin_path) = found {
            let mut cmd = std::process::Command::new(&bin_path);
            cmd.args(*args);
            cmd.arg(&path);
            cmd.spawn()
                .map_err(|e| Error::new("io", format!("{bin}: {e}")).with("detail", e.to_string()))?;
            return Ok(());
        }
    }
    Err(Error::new("noTerminal", "Kein Terminal-Emulator gefunden."))
}

pub const TRASH_VIRTUAL: &str = "trash://";

/// Версия без ошибок для списка каталога (корзина).
/// Infallible variant for directory listing (trash).
pub fn list_trash_inner() -> Vec<TrashItem> {
    list_trash().unwrap_or_default()
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    fn tmpdir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "compound_ops_{}_{}",
            tag,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn runner(policy: &str, cancel: Option<Arc<AtomicBool>>) -> OpRunner {
        OpRunner {
            op_id: "test-op".into(),
            op_name: "test",
            app: None,
            cancel: CancelToken(cancel.unwrap_or_else(|| Arc::new(AtomicBool::new(false)))),
            policy: policy.into(),
            progress: Channel::<ProgressEvent>::new(|_| Ok(())),
            conflicts: Channel::<ConflictRequest>::new(|_| Ok(())),
            total_bytes: 0,
            bytes_done: Arc::new(AtomicU64::new(0)),
            files_done: Arc::new(AtomicU64::new(0)),
            total_items: 1,
            conflict_index: 0,
            last_emit: Instant::now(),
        }
    }

    #[test]
    fn copy_file_roundtrip_preserves_content() {
        let dir = tmpdir("copy");
        let src = dir.join("a.txt");
        let dst = dir.join("b.txt");
        std::fs::write(&src, b"hello world").unwrap();
        let mut r = runner("ask", None);
        r.copy_file(&src, &dst).unwrap();
        assert_eq!(std::fs::read(&dst).unwrap(), b"hello world");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn copy_into_existing_dir_with_overwrite() {
        let dir = tmpdir("overwrite");
        let src = dir.join("d");
        let dest = dir.join("dest");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("f.txt"), b"neu").unwrap();
        std::fs::create_dir_all(&dest).unwrap();
        // Konflikt vorbereiten
        std::fs::create_dir_all(dest.join("d")).unwrap();
        std::fs::write(dest.join("d/f.txt"), b"alt").unwrap();
        let mut r = runner("overwrite", None);
        r.run_copy(&[src.display().to_string()], &dest.display().to_string())
            .unwrap();
        assert_eq!(std::fs::read(dest.join("d/f.txt")).unwrap(), b"neu");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn copy_keep_both_creates_unique_name() {
        let dir = tmpdir("keep_both");
        let src = dir.join("a.txt");
        let dest = dir.join("dest");
        std::fs::write(&src, b"quellinhalt").unwrap();
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("a.txt"), b"original").unwrap();
        let mut r = runner("keep_both", None);
        r.run_copy(&[src.display().to_string()], &dest.display().to_string())
            .unwrap();
        assert_eq!(std::fs::read(dest.join("a.txt")).unwrap(), b"original");
        assert_eq!(std::fs::read(dest.join("a (2).txt")).unwrap(), b"quellinhalt");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn copy_skip_keeps_existing() {
        let dir = tmpdir("skip");
        let src = dir.join("a.txt");
        let dest = dir.join("dest");
        std::fs::write(&src, b"neu").unwrap();
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("a.txt"), b"original").unwrap();
        let mut r = runner("skip", None);
        r.run_copy(&[src.display().to_string()], &dest.display().to_string())
            .unwrap();
        assert_eq!(std::fs::read(dest.join("a.txt")).unwrap(), b"original");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn precheck_rejects_copy_into_own_subtree() {
        let dir = tmpdir("precheck");
        let inner = dir.join("src");
        std::fs::create_dir_all(&inner).unwrap();
        let err = runner("ask", None)
            .run_copy(&[inner.display().to_string()], &inner.display().to_string())
            .unwrap_err();
        let m = err.to_string();
        assert!(
            m.contains("in sich selbst") || m.contains("identisch"),
            "unerwartete Meldung: {m}"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn move_same_fs_renames() {
        let dir = tmpdir("move");
        let dest = dir.join("dest");
        std::fs::create_dir_all(&dest).unwrap();
        let f = dir.join("m.txt");
        std::fs::write(&f, b"inhalt").unwrap();
        let mut r = runner("ask", None);
        r.run_move(&[f.display().to_string()], &dest.display().to_string())
            .unwrap();
        assert!(!f.exists());
        assert_eq!(std::fs::read(dest.join("m.txt")).unwrap(), b"inhalt");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn move_keep_both_on_conflict() {
        let dir = tmpdir("move_kb");
        let dest = dir.join("dest");
        std::fs::create_dir_all(&dest).unwrap();
        let f = dir.join("m.txt");
        std::fs::write(&f, b"neuer").unwrap();
        std::fs::write(dest.join("m.txt"), b"alter").unwrap();
        let mut r = runner("keep_both", None);
        r.run_move(&[f.display().to_string()], &dest.display().to_string())
            .unwrap();
        assert!(!f.exists());
        assert_eq!(std::fs::read(dest.join("m.txt")).unwrap(), b"alter");
        assert_eq!(std::fs::read(dest.join("m (2).txt")).unwrap(), b"neuer");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn delete_removes_nested_tree() {
        let dir = tmpdir("delete");
        let tree = dir.join("tree");
        std::fs::create_dir_all(tree.join("sub/deep")).unwrap();
        std::fs::write(tree.join("top.txt"), b"x").unwrap();
        std::fs::write(tree.join("sub/deep/inner.txt"), b"y").unwrap();
        let mut r = runner("ask", None);
        r.run_delete(&[tree.display().to_string()]).unwrap();
        assert!(!tree.exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn cancel_token_aborts_before_copy() {
        let dir = tmpdir("cancel");
        let src = dir.join("a.txt");
        let dest = dir.join("dest");
        std::fs::write(&src, b"x").unwrap();
        std::fs::create_dir_all(&dest).unwrap();
        let flag = Arc::new(AtomicBool::new(true));
        let mut r = runner("ask", Some(flag));
        let err = r.run_copy(&[src.display().to_string()], &dest.display().to_string()).unwrap_err();
        assert_eq!(err.to_string(), "Abgebrochen");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn copy_preserves_exec_permission() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tmpdir("perm");
        let src = dir.join("tool.sh");
        let dst = dir.join("tool2.sh");
        std::fs::write(&src, b"#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&src, std::fs::Permissions::from_mode(0o755)).unwrap();
        let mut r = runner("ask", None);
        r.copy_file(&src, &dst).unwrap();
        let mode = std::fs::metadata(&dst).unwrap().permissions().mode();
        assert_eq!(mode & 0o111, 0o111, "Ausführ-Bit muss erhalten bleiben");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
