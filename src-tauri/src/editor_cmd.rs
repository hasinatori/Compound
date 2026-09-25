// Чтение/запись текста для редактора с защитой от гонки по mtime.
// Read/write text for the editor with an mtime race guard.
// expected_mTIME не совпал -> ошибка, значит файл меняли извне.
// expected_mtime mismatch -> error, the file changed externally.

use crate::fsutil;
use crate::types::*;
use crate::ui_error::{self, Error};
use std::fs;
use std::path::Path;

const DEFAULT_CAP: u64 = 4 * 1024 * 1024;
const MAX_CAP: u64 = 16 * 1024 * 1024;

/// Читает текст (UTF-8, фолбэк Latin-1), отсекает бинарь.
/// Reads a text file (UTF-8, Latin-1 fallback) and rejects binaries.
#[tauri::command]
pub fn read_text(path: String, max_bytes: Option<u64>) -> Result<EditorRead, Error> {
    let p = Path::new(&path);
    let meta = fs::metadata(p).map_err(|e| ui_error::io(path.clone(), e))?;
    if meta.is_dir() {
        return Err(Error::new("isDir", "Ist ein Ordner."));
    }
    let cap = max_bytes
        .map(|c| c.clamp(1024, MAX_CAP))
        .unwrap_or(DEFAULT_CAP);
    let bytes = fsutil::read_capped(p, cap).map_err(|e| ui_error::io(path.clone(), e))?;
    let mtime = fsutil::mtime_ms(&meta);
    let truncated = meta.len() > cap;
    if fsutil::is_binary(&bytes) {
        return Ok(EditorRead {
            text: String::new(),
            encoding: "binary".into(),
            readonly: true,
            binary: true,
            truncated: true,
            converted: false,
            mtime_ms: mtime,
        });
    }
    match String::from_utf8(bytes.clone()) {
        Ok(text) => Ok(EditorRead {
            text,
            encoding: "UTF-8".into(),
            readonly: false,
            binary: false,
            truncated,
            converted: false,
            mtime_ms: mtime,
        }),
        Err(_) => {
            // Latin-1: 1:1 byte->char, пишется обратно без потерь.
        // Latin-1: 1:1 byte->char, writes back losslessly.
            let text = bytes
                .iter()
                .map(|&b| char::from_u32(b as u32).unwrap_or('\u{FFFD}'))
                .collect::<String>();
            Ok(EditorRead {
                text,
                encoding: "Latin-1 (konvertiert nach UTF-8)".into(),
                readonly: false,
                binary: false,
                truncated,
                converted: true,
                mtime_ms: mtime,
            })
        }
    }
}

/// Сохраняет текст атомарно (временный файл + rename).
/// Saves a text file atomically (temp file + rename).
#[tauri::command]
pub fn save_text(path: String, content: String, expected_mtime: i64) -> Result<SaveResult, Error> {
    let p = Path::new(&path);
    let meta = fs::metadata(p).map_err(|e| ui_error::io(path.clone(), e))?;
    if meta.is_dir() {
        return Err(Error::new("isDir", "Ist ein Ordner."));
    }
    let current_mtime = fsutil::mtime_ms(&meta);
    if current_mtime != expected_mtime {
        // Datei hat sich geändert -> Konflikt.
        return Ok(SaveResult {
            saved: false,
            changed: true,
            error: None,
            mtime_ms: current_mtime,
        });
    }
    let parent = p
        .parent()
        .ok_or_else(|| Error::new("noParent", "Kein übergeordneter Ordner"))?;
    let tmp = parent.join(format!(".compound_save_{}", uuid::Uuid::new_v4()));
    fs::write(&tmp, content.as_bytes()).map_err(|e| ui_error::io(tmp.display(), e))?;
    // Права оригинала переносим.
    // Keep the original file's permissions.
    fs::set_permissions(&tmp, meta.permissions())
        .map_err(|e| ui_error::io(tmp.display(), e))?;
    fs::rename(&tmp, p).map_err(|e| ui_error::io(path.clone(), e))?;
    let new_meta = fs::metadata(p).map_err(|e| ui_error::io(path.clone(), e))?;
    Ok(SaveResult {
        saved: true,
        changed: false,
        error: None,
        mtime_ms: fsutil::mtime_ms(&new_meta),
    })
}