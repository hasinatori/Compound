// Локализованные ошибки: наружу только {code, params}, текст — во фронте.
// Localized errors: only {code, params} crosses the wire, text lives in the UI.
// Код -> ключ i18n errors.<code>. Немецкий текст только для лога.
// code -> i18n key errors.<code>. German text is log-only.

use serde::ser::{Serialize, SerializeMap, Serializer};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    code: &'static str,
    params: BTreeMap<String, String>,
    msg: String,
}

impl Error {
    pub fn new(code: &'static str, msg: impl Into<String>) -> Self {
        Self {
            code,
            params: BTreeMap::new(),
            msg: msg.into(),
        }
    }

    pub fn with(mut self, key: &str, val: impl fmt::Display) -> Self {
        self.params.insert(key.to_string(), val.to_string());
        self
    }

    /// Только тесты/хелперы фронта; сериализация через `Serialize`.
    /// Used by tests/frontend helpers only; serialization goes via `Serialize`.
    #[allow(dead_code)]
    pub fn code(&self) -> &'static str {
        self.code
    }

    #[allow(dead_code)]
    pub fn params(&self) -> &BTreeMap<String, String> {
        &self.params
    }
}

/// I/O-ошибка с путём и текстом ОС.
/// I/O error carrying the path and the OS message.
pub fn io(path: impl fmt::Display, err: impl fmt::Display) -> Error {
    Error::new("io", format!("I/O-Fehler: {path}: {err}"))
        .with("path", path.to_string())
        .with("detail", err.to_string())
}

/// SQLite-/Datenbankfehler.
pub fn db(err: impl fmt::Display) -> Error {
    Error::new("db", format!("Datenbankfehler: {err}")).with("err", err.to_string())
}

/// Kurzer Lock-Fehler (interner Zustand blockiert).
pub fn lock() -> Error {
    Error::new("stateLock", "Interner Zustand blockiert")
}

/// Хелпер параметров для событий прогресса (`{name: value}`).
/// Param helper for progress events (`{name: value}`).
pub fn par(name: impl fmt::Display, val: impl fmt::Display) -> BTreeMap<String, String> {
    BTreeMap::from([(name.to_string(), val.to_string())])
}

/// Добавляет ещё одну пару в карту параметров.
/// Adds one more entry to a param map.
pub fn param(
    mut map: BTreeMap<String, String>,
    name: impl fmt::Display,
    val: impl fmt::Display,
) -> BTreeMap<String, String> {
    map.insert(name.to_string(), val.to_string());
    map
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.msg)
    }
}

impl std::error::Error for Error {}

impl Serialize for Error {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut map = s.serialize_map(Some(2))?;
        map.serialize_entry("code", &self.code)?;
        map.serialize_entry("params", &self.params)?;
        map.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_to_code_and_params() {
        let e = Error::new("destMissing", "Zielordner existiert nicht.");
        let json = serde_json::to_string(&e).unwrap();
        assert_eq!(json, r#"{"code":"destMissing","params":{}}"#);
    }

    #[test]
    fn with_adds_params() {
        let e = Error::new("selfCopy", "x")
            .with("path", "/a/b")
            .with("detail", "nested source");
        assert_eq!(e.params().get("path").map(|s| s.as_str()), Some("/a/b"));
        assert_eq!(e.params().get("detail").map(|s| s.as_str()), Some("nested source"));
    }

    #[test]
    fn display_keeps_german_fallback() {
        let e = Error::new("canceled", "Abgebrochen");
        assert_eq!(e.to_string(), "Abgebrochen");
    }

    #[test]
    fn io_carries_path_and_detail() {
        let e = io("/x/y", "Permission denied");
        assert_eq!(e.code(), "io");
        assert_eq!(e.params().get("path").map(|s| s.as_str()), Some("/x/y"));
        assert_eq!(e.params().get("detail").map(|s| s.as_str()), Some("Permission denied"));
        assert!(e.to_string().contains("Permission denied"));
    }

    #[test]
    fn db_and_lock_codes() {
        assert_eq!(db("disk full").code(), "db");
        assert_eq!(lock().code(), "stateLock");
    }

    #[test]
    fn par_builds_single_entry_map() {
        let m = par("path", "/a");
        assert_eq!(m.get("path").map(|s| s.as_str()), Some("/a"));
        assert_eq!(m.len(), 1);
    }
}