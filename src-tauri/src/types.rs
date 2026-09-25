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

/// Ort in der Seitenleiste (Home, Ordner, Laufwerke, Papierkorb, Root).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Place {
    pub name: String,
    pub path: String,
    pub kind: String,
}

/// Ein Pfadsegment für die Breadcrumb-Leiste.
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

/// Fortschritts-Ereignis für laufende Operationen (Channel).
/// `kind` ist ein Maschinen-Token (path/target/skip/skipMissing/copying/done/groups/pct),
/// `params` trägt die Platzhalter; die UI übersetzt über `op.<kind>` (i18n).
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

/// Rückgabe einer Operation an den Aufrufer (op wird asynchron ausgeführt).
/// `kind` ist ein Maschinen-Token (`op.copy` usw.), das die UI übersetzt.
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

/// Anfrage an die UI bei Namens- oder Überschreibkonflikt.
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

/// Antwort der UI auf einen Konflikt.
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
    /// Nur für "ext": neue Endung ohne Punkt
    #[serde(default)]
    pub new_ext: Option<String>,
    /// Für numbering: Vorlage mit Platzhaltern {n}, {name}, {ext}
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