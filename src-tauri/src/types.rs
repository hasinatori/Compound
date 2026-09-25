// Serde-типы на границе frontend/backend. Контракт — не меняем без надобности.
// Serde types on the frontend/backend boundary. Contract — change with care.
// TS-зеркало: src/lib/types.ts, держим синхронно по полям.
// TS mirror: src/lib/types.ts, keep field-by-field in sync.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Ein Eintrag in einer Verzeichnisliste.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub path: String,
    pub name: String,
    pub ext: String,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub is_hidden: bool,
    pub size: u64,
    pub mtime_ms: i64,
    pub perms: String,
    pub kind: String,
    pub link_target: Option<String>,
}

/// Пункт сайдбара (дом, папки, диски, корзина, корень).
/// Sidebar place (home, folders, drives, trash, root).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Place {
    pub name: String,
    pub path: String,
    pub kind: String,
}

/// Сегмент пути для хлебных крошек.
/// One path segment for the breadcrumb bar.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathPart {
    pub name: String,
    pub path: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskUsage {
    pub total: u64,
    pub free: u64,
}

/// Событие прогресса для долгих операций (Channel).
/// Progress event for running operations (channel).
/// `kind` — машинный токен (path/target/skip/skipMissing/copying/done/groups/pct),
/// `kind` is a machine token (path/target/skip/skipMissing/copying/done/groups/pct),
/// `params` несёт плейсхолдеры, UI переводит по `op.<kind>`.
/// `params` carries placeholders; the UI maps them via `op.<kind>`.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub op: String,
    pub phase: String,
    pub current: u64,
    pub total: u64,
    pub kind: String,
    pub params: BTreeMap<String, String>,
}

impl ProgressEvent {
    pub fn new(
        op: &str,
        phase: &str,
        current: u64,
        total: u64,
        kind: impl Into<String>,
        params: BTreeMap<String, String>,
    ) -> Self {
        Self {
            op: op.into(),
            phase: phase.into(),
            current,
            total,
            kind: kind.into(),
            params,
        }
    }
}

/// Ответ на запуск операции (сама операция идёт асинхронно).
/// Return value of an op start (the op itself runs async).
/// `kind` — машинный токен (`op.copy` и т.п.), UI его переводит.
/// `kind` is a machine token (`op.copy` etc.) that the UI translates.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpStarted {
    pub op_id: String,
    pub kind: String,
    pub total: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpFinished {
    pub op_id: String,
    pub ok: bool,
    pub error: Option<crate::ui_error::Error>,
}

/// Запрос в UI при конфликте имени или перезаписи.
/// Request to the UI on a name or overwrite conflict.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictRequest {
    pub op_id: String,
    pub key: String,
    pub src: String,
    pub dest: String,
    pub is_dir: bool,
    pub index: u64,
    pub total: u64,
}

/// Ответ UI на конфликт.
/// UI answer to a conflict.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictChoice {
    /// overwrite | skip | keep_both | abort
    pub action: String,
    pub apply_to_all: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchOpts {
    #[serde(default)]
    pub include_hidden: bool,
    #[serde(default)]
    pub name_only: bool,
    #[serde(default)]
    pub max_size: Option<u64>,
    #[serde(default)]
    pub kinds: Vec<String>,
}

impl Default for SearchOpts {
    fn default() -> Self {
        Self {
            include_hidden: false,
            name_only: false,
            max_size: None,
            kinds: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub mtime_ms: i64,
    pub kind: String,
    pub line_no: Option<u64>,
    pub snippet: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchReturn {
    pub results: Vec<SearchResult>,
    pub used_index: bool,
    pub cancelled: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexSummary {
    pub files: u64,
    pub dirs: u64,
    pub ms: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexedRoot {
    pub root: String,
    pub indexed_at_ms: i64,
    pub file_count: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateGroup {
    pub size: u64,
    pub hash: String,
    pub files: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompareItem {
    pub relative: String,
    /// only_in_a | only_in_b | different | identical
    pub status: String,
    pub is_dir: bool,
    pub size_a: i64,
    pub size_b: i64,
    pub mtime_a: i64,
    pub mtime_b: i64,
    pub msg: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameRule {
    /// find_replace | regex | remove | insert | case | ext | numbering
    pub rule_type: String,
    #[serde(default)]
    pub find: Option<String>,
    #[serde(default)]
    pub replace: Option<String>,
    #[serde(default)]
    pub insert_at: Option<u32>,
    #[serde(default)]
    pub insert_text: Option<String>,
    /// upper | lower | title
    #[serde(default)]
    pub case_mode: Option<String>,
    /// Только для "ext": новое расширение без точки
    /// "ext" only: the new extension without the dot
    #[serde(default)]
    pub new_ext: Option<String>,
    /// Для numbering: шаблон с плейсхолдерами {n}, {name}, {ext}
    /// For numbering: template with {n}, {name}, {ext} placeholders
    #[serde(default)]
    pub template: Option<String>,
    #[serde(default)]
    pub start: Option<u32>,
    #[serde(default)]
    pub step: Option<u32>,
    #[serde(default)]
    pub digits: Option<u32>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenamePreviewItem {
    pub from: String,
    pub to: String,
    pub conflict: bool,
    pub error: Option<crate::ui_error::Error>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveEntry {
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveSummary {
    pub entries: u64,
    pub bytes: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorRead {
    pub text: String,
    pub encoding: String,
    pub readonly: bool,
    pub binary: bool,
    pub truncated: bool,
    pub converted: bool,
    pub mtime_ms: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveResult {
    pub saved: bool,
    pub changed: bool,
    pub error: Option<String>,
    pub mtime_ms: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashItem {
    pub name_in_trash: String,
    pub original_path: String,
    pub trashed_at_ms: i64,
    pub is_dir: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalInfo {
    pub id: String,
    pub cwd: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AboutInfo {
    pub name: String,
    pub version: String,
    pub identifier: String,
}