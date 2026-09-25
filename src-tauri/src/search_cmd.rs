use crate::fsutil;
use crate::state::{register_op, unregister_op, AppState, CancelToken};
use crate::types::*;
use crate::ui_error::{self, Error};
use rusqlite::{params, Connection, OptionalExtension};
use std::fs;
use std::path::Path;
use std::sync::MutexGuard;
use std::time::Instant;
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, Manager, State};

const INDEX_TEXT_CAP: u64 = 2 * 1024 * 1024;
const SEARCH_TEXT_CAP: u64 = 8 * 1024 * 1024;
const RESULT_CAP: usize = 10_000;

fn with_conn<'a, T>(
    state: &'a State<'_, AppState>,
    f: impl FnOnce(&Connection) -> Result<T, Error>,
) -> Result<T, Error> {
    let guard: MutexGuard<'a, Option<Connection>> = state.conn()?;
    let conn = guard.as_ref().unwrap();
    f(conn)
}

/// Löscht (samt FTS) alle Einträge eines Teilbaums.
fn delete_subtree(conn: &Connection, root: &str) -> Result<(), Error> {
    if root == "/" {
        conn.execute_batch("DELETE FROM filetext; DELETE FROM files;")
            .map_err(ui_error::db)?;
        return Ok(());
    }
    let like = format!("{}%", fsutil::esc_like(root));
    conn.execute(
        "DELETE FROM filetext WHERE rowid IN
           (SELECT rowid FROM files WHERE path = ?1 OR path LIKE ?2 ESCAPE '\\')",
        params![root, like],
    )
    .map_err(ui_error::db)?;
    conn.execute(
        "DELETE FROM files WHERE path = ?1 OR path LIKE ?2 ESCAPE '\\'",
        params![root, like],
    )
    .map_err(ui_error::db)?;
    Ok(())
}

fn run_index(
    app: &AppHandle,
    root: &str,
    force: bool,
    progress: &Channel<ProgressEvent>,
    cancel: &CancelToken,
) -> Result<IndexSummary, Error> {
    let _ = force;
    let start = Instant::now();
    let state = app.state::<AppState>();
    let _indexing = state
        .indexing
        .lock()
        .map_err(|_| Error::new("indexingRunning", "Indizierung läuft bereits"))?;

    let mut guard = state.conn()?;
    let conn = guard.as_mut().unwrap();

    // Anzahl vorab bestimmen (Fortschritt).
    let mut total = 0u64;
    for _ in ignore::WalkBuilder::new(root)
        .hidden(false)
        .follow_links(false)
        .build()
        .filter_map(Result::ok)
    {
        total += 1;
        if total % 4096 == 0 && cancel.cancelled() {
            return Err(Error::new("canceled", "Abgebrochen"));
        }
    }

    delete_subtree(conn, root)?;

    type Row = (String, String, String, bool, u64, i64, String, Option<String>);

    fn commit_batch(
        conn: &mut Connection,
        rows: &mut Vec<Row>,
    ) -> Result<(), Error> {
        if rows.is_empty() {
            return Ok(());
        }
        let tx = conn.transaction().map_err(ui_error::db)?;
        {
            let mut ins = tx
                .prepare(
                    "INSERT INTO files(path,name,parent,is_file,size,mtime,kind)
                     VALUES(?1,?2,?3,?4,?5,?6,?7)
                     ON CONFLICT(path) DO UPDATE SET
                       name=excluded.name, parent=excluded.parent, is_file=excluded.is_file,
                       size=excluded.size, mtime=excluded.mtime, kind=excluded.kind",
                )
                .map_err(ui_error::db)?;
            let mut del_fts = tx
                .prepare("DELETE FROM filetext WHERE path=?1")
                .map_err(ui_error::db)?;
            let mut ins_fts = tx
                .prepare(
                    "INSERT INTO filetext(rowid, path, content)
                     VALUES((SELECT rowid FROM files WHERE path=?1), ?1, ?2)",
                )
                .map_err(ui_error::db)?;
            for (path, name, parent, is_file, size, mtime, kind, text) in rows.iter() {
                ins.execute(params![path, name, parent, *is_file as i64, *size as i64, mtime, kind])
                    .map_err(ui_error::db)?;
                if let Some(text) = text {
                    del_fts.execute(params![path]).map_err(ui_error::db)?;
                    ins_fts.execute(params![path, text]).map_err(ui_error::db)?;
                }
            }
        }
        tx.commit().map_err(ui_error::db)?;
        rows.clear();
        Ok(())
    }

    let mut rows: Vec<Row> = Vec::with_capacity(800);
    let mut files: u64 = 0;
    let mut dirs: u64 = 0;
    let mut done: u64 = 0;

    let walk = ignore::WalkBuilder::new(root)
        .hidden(false)
        .follow_links(false)
        .filter_entry(|e| !is_virtual_mount(e.path()))
        .build();

    for entry in walk.filter_map(Result::ok) {
        if cancel.cancelled() {
            return Err(Error::new("canceled", "Abgebrochen"));
        }
        let path = entry.path();
        let ft = entry.file_type();
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let is_dir = ft.map(|t| t.is_dir()).unwrap_or(false) || meta.is_dir();
        let is_file = !is_dir;
        let name = entry.file_name().to_string_lossy().into_owned();
        let parent = path
            .parent()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "/".into());
        let mtime = fsutil::mtime_ms(&meta);
        let kind = fsutil::kind_for(is_dir, &name);
        let size = if is_file { meta.len() } else { 0 };

        // Text-Inhalt für FTS (nur kleine Text-/Codedateien).
        let mut text = None;
        if is_file && size > 0 && size <= INDEX_TEXT_CAP && fsutil::is_text_like_kind(&kind) {
            if let Ok(bytes) = fsutil::read_capped(path, INDEX_TEXT_CAP) {
                if !fsutil::is_binary(&bytes) {
                    if let Ok(text_s) = String::from_utf8(bytes) {
                        text = Some(text_s);
                    }
                }
            }
        }

        rows.push((
            path.display().to_string(),
            name,
            parent,
            is_file,
            size,
            mtime,
            kind,
            text,
        ));

        if is_dir {
            dirs += 1;
        } else {
            files += 1;
        }
        done += 1;
        if done % 512 == 0 {
            let _ = progress.send(ProgressEvent::new(
                "index",
                "index",
                done,
                total,
                "path",
                ui_error::par("path", path.display()),
            ));
        }
        if rows.len() >= 800 {
            commit_batch(conn, &mut rows)?;
        }
    }
    commit_batch(conn, &mut rows)?;

    conn.execute(
        "INSERT INTO roots(root, indexed_at, file_count) VALUES(?1,?2,?3)
         ON CONFLICT(root) DO UPDATE SET indexed_at=excluded.indexed_at, file_count=excluded.file_count",
        params![root, crate::state::now_ms(), files],
    )
    .map_err(ui_error::db)?;

    let _ = progress.send(ProgressEvent::new(
        "index",
        "done",
        done,
        total,
        "done",
        ui_error::param(ui_error::par("files", files), "dirs", dirs),
    ));

    Ok(IndexSummary {
        files,
        dirs,
        ms: start.elapsed().as_millis() as u64,
    })
}

fn is_virtual_mount(path: &Path) -> bool {
    // Grobe Heuristik: /proc, /sys, /dev, /run als Wurzel-Orte nicht indizieren.
    let p = path.to_string_lossy();
    p == "/proc" || p == "/sys" || p == "/dev" || p == "/run"
}

#[tauri::command]
pub async fn index_folder(
    app: AppHandle,
    state: State<'_, AppState>,
    root: String,
    force: bool,
    progress: Channel<ProgressEvent>,
) -> Result<IndexSummary, Error> {
    let root_n = fsutil::norm_path(&root);
    if !Path::new(&root_n).is_dir() {
        return Err(Error::new("noFolder", "Kein Ordner"));
    }
    let (op_id, cancel) = register_op(&state, "op.index");
    let app2 = app.clone();
    let progress2 = progress.clone();
    let res = tauri::async_runtime::spawn_blocking(move || {
        let out = run_index(&app2, &root_n, force, &progress2, &cancel);
        let _ = app2.emit(
            "op-finished",
            OpFinished {
                op_id: op_id.clone(),
                ok: out.is_ok(),
                error: out.as_ref().err().cloned(),
            },
        );
        {
            let st = app2.state::<AppState>();
            unregister_op(&st, &op_id);
        }
        out
    })
    .await
    .map_err(|e| Error::new("internal", "Thread-Pool-Fehler").with("detail", e.to_string()))?;
    res
}

#[tauri::command]
pub fn indexed_roots(state: State<'_, AppState>) -> Result<Vec<IndexedRoot>, Error> {
    with_conn(&state, |conn| {
        let mut stmt = conn
            .prepare("SELECT root, indexed_at, file_count FROM roots ORDER BY root")
            .map_err(ui_error::db)?;
        let rows = stmt
            .query_map([], |r| {
                Ok(IndexedRoot {
                    root: r.get(0)?,
                    indexed_at_ms: r.get(1)?,
                    file_count: r.get(2)?,
                })
            })
            .map_err(ui_error::db)?;
        let mut out = Vec::new();
        for row in rows.flatten() {
            if Path::new(&row.root).is_dir() {
                out.push(row);
            }
        }
        Ok(out)
    })
}

#[tauri::command]
pub fn clear_index(state: State<'_, AppState>) -> Result<(), Error> {
    with_conn(&state, |conn| {
        conn.execute_batch("DELETE FROM filetext; DELETE FROM files; DELETE FROM roots;")
            .map_err(ui_error::db)?;
        Ok(())
    })
}

fn pass_filters(opts: &SearchOpts, name: &str, kind: &str, size: u64) -> bool {
    if !opts.include_hidden && fsutil::is_hidden_name(name) {
        return false;
    }
    if !opts.kinds.is_empty() && !opts.kinds.contains(&kind.to_string()) {
        return false;
    }
    if let Some(max) = opts.max_size {
        if size > max {
            return false;
        }
    }
    true
}

fn fts_query(q: &str) -> String {
    let tokens: Vec<String> = q
        .split_whitespace()
        .map(|t| {
            let clean: String = t.chars().filter(|c| c.is_alphanumeric()).collect();
            clean
        })
        .filter(|t| !t.is_empty())
        .map(|t| format!("\"{}\"", t))
        .collect();
    tokens.join(" AND ")
}

fn do_search_index(
    app: &AppHandle,
    query: &str,
    root: &str,
    opts: &SearchOpts,
) -> Result<SearchReturn, Error> {
    let state = app.state::<AppState>();
    let guard = state.conn()?;
    let conn = guard.as_ref().unwrap();

    let indexed: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM roots WHERE root=?1",
            params![root],
            |r| r.get::<_, i64>(0),
        )
        .optional()
        .map_err(ui_error::db)?
        .unwrap_or(0)
        > 0;

    if !indexed {
        return Ok(SearchReturn {
            results: vec![],
            used_index: false,
            cancelled: false,
        });
    }

    let ql = query.to_lowercase();
    let mut results: Vec<SearchResult> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let like = format!("%{}%", fsutil::esc_like(&ql));

    {
        let mut stmt = conn
            .prepare(
                "SELECT path, name, is_file, size, mtime, kind FROM files
                 WHERE (parent = ?1 OR parent LIKE ?2 ESCAPE '\\') AND name LIKE ?3 ESCAPE '\\'
                 ORDER BY name LIMIT 4000",
            )
            .map_err(ui_error::db)?;
        let rows = stmt
            .query_map(params![root, format!("{}%", fsutil::esc_like(root)), like], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)? != 0,
                    r.get::<_, i64>(3)? as u64,
                    r.get::<_, i64>(4)?,
                    r.get::<_, String>(5)?,
                ))
            })
            .map_err(ui_error::db)?;
        for row in rows.flatten() {
            let (path, name, is_dir, size, mtime, kind) = row;
            if !pass_filters(opts, &name, &kind, size) {
                continue;
            }
            if seen.insert(path.clone()) {
                results.push(SearchResult {
                    path,
                    name,
                    is_dir,
                    size,
                    mtime_ms: mtime,
                    kind,
                    line_no: None,
                    snippet: None,
                });
            }
        }
    }

    if !opts.name_only {
        let fq = fts_query(query);
        if !fq.is_empty() {
            let mut stmt = conn
                .prepare(
                    "SELECT path, snippet(filetext, 1, '[', ']', '…', 40) AS snip
                     FROM filetext WHERE filetext MATCH ?1 LIMIT 3000",
                )
                .map_err(ui_error::db)?;
            let rows = stmt
                .query_map(params![fq], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
                })
                .map_err(ui_error::db)?;
            for row in rows.flatten() {
                let (path, snip) = row;
                if seen.contains(&path) {
                    continue;
                }
                let name = path.rsplit('/').next().unwrap_or(&path).to_string();
                let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                let mtime = fs::metadata(&path)
                    .map(|m| fsutil::mtime_ms(&m))
                    .unwrap_or(0);
                let kind = fsutil::kind_for(false, &name);
                if !pass_filters(opts, &name, &kind, size) {
                    continue;
                }
                seen.insert(path.clone());
                results.push(SearchResult {
                    path,
                    name,
                    is_dir: false,
                    size,
                    mtime_ms: mtime,
                    kind,
                    line_no: None,
                    snippet: Some(snip),
                });
            }
        }
    }

    results.sort_by(|a, b| {
        fsutil_nat_name(a, b)
    });
    results.truncate(RESULT_CAP);
    Ok(SearchReturn {
        results,
        used_index: true,
        cancelled: false,
    })
}

fn fsutil_nat_name(a: &SearchResult, b: &SearchResult) -> std::cmp::Ordering {
    let mut ai = a.name.chars().peekable();
    let mut bi = b.name.chars().peekable();
    loop {
        match (ai.next(), bi.next()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, _) => return std::cmp::Ordering::Less,
            (_, None) => return std::cmp::Ordering::Greater,
            (Some(x), Some(y)) => {
                let xa = x.to_ascii_lowercase();
                let ya = y.to_ascii_lowercase();
                if xa.is_ascii_digit() && ya.is_ascii_digit() {
                    let (na, _) = gather_digits(&mut ai, xa);
                    let (nb, _) = gather_digits(&mut bi, ya);
                    let o = na.cmp(&nb);
                    if o != std::cmp::Ordering::Equal {
                        return o;
                    }
                } else {
                    let o = xa.cmp(&ya);
                    if o != std::cmp::Ordering::Equal {
                        return o;
                    }
                }
            }
        }
    }
}

fn gather_digits(iter: &mut std::iter::Peekable<std::str::Chars>, first: char) -> (u64, usize) {
    let mut s = String::new();
    s.push(first);
    while let Some(&c) = iter.peek() {
        if c.is_ascii_digit() {
            s.push(c);
            iter.next();
        } else {
            break;
        }
    }
    (s.parse::<u64>().unwrap_or(0), s.len())
}

fn run_search_now(
    app: &AppHandle,
    query: &str,
    root: &str,
    opts: &SearchOpts,
    content: bool,
    progress: &Channel<ProgressEvent>,
    cancel: &CancelToken,
) -> Result<SearchReturn, Error> {
    let _ = app;
    let ql = query.to_lowercase();
    if ql.is_empty() {
        return Err(Error::new("emptyQuery", "Suchbegriff darf nicht leer sein"));
    }
    let mut results: Vec<SearchResult> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut done: u64 = 0;
    let mut cancelled = false;

    let walk = ignore::WalkBuilder::new(root)
        .hidden(true)
        .follow_links(false)
        .build();

    for entry in walk.filter_map(Result::ok) {
        if cancel.cancelled() {
            cancelled = true;
            break;
        }
        done += 1;
        if done % 512 == 0 {
            let _ = progress.send(ProgressEvent::new(
                "search",
                "search",
                done,
                0,
                "path",
                ui_error::par("path", entry.path().display()),
            ));
        }
        let Some(ft) = entry.file_type() else { continue };
        if ft.is_dir() {
            continue;
        }
        if ft.is_symlink() {
            continue;
        }
        if results.len() >= RESULT_CAP {
            break;
        }
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if !opts.include_hidden && fsutil::is_hidden_name(&name) {
            continue;
        }
        let size = meta.len();
        if let Some(max) = opts.max_size {
            if size > max {
                continue;
            }
        }
        let kind = fsutil::kind_for(false, &name);
        if !opts.kinds.is_empty() && !opts.kinds.contains(&kind) {
            continue;
        }
        let name_hits = name.to_lowercase().contains(&ql);
        if !name_hits && !content {
            continue;
        }

        let mut line_no = None;
        let mut snippet = None;
        let mut hit = name_hits;

        if content && kind_has_text(&kind) && size <= SEARCH_TEXT_CAP && size > 0 {
            if let Ok(bytes) = fsutil::read_capped(path, SEARCH_TEXT_CAP) {
                if !fsutil::is_binary(&bytes) {
                    let text = String::from_utf8_lossy(&bytes);
                    let mut found_line = None;
                    for (i, line) in text.lines().enumerate() {
                        if line.to_lowercase().contains(&ql) {
                            found_line = Some((i as u64 + 1, line.trim().to_string()));
                            if found_line.is_some() {
                                break;
                            }
                        }
                    }
                    if let Some((l, s)) = found_line {
                        line_no = Some(l);
                        let mut s: String = s.chars().take(200).collect();
                        if s.len() > 190 {
                            s.push('…');
                        }
                        snippet = Some(s);
                        hit = true;
                    }
                }
            }
        }

        if hit && seen.insert(path.display().to_string()) {
            results.push(SearchResult {
                path: path.display().to_string(),
                name,
                is_dir: false,
                size,
                mtime_ms: fsutil::mtime_ms(&meta),
                kind,
                line_no,
                snippet,
            });
        }
    }

    results.sort_by(fsutil_nat_name);
    Ok(SearchReturn {
        results,
        used_index: false,
        cancelled,
    })
}

fn kind_has_text(kind: &str) -> bool {
    fsutil::is_text_like_kind(kind)
}

#[tauri::command]
pub async fn search_now(
    app: AppHandle,
    state: State<'_, AppState>,
    query: String,
    root: String,
    opts: SearchOpts,
    content: bool,
    progress: Channel<ProgressEvent>,
) -> Result<SearchReturn, Error> {
    let root_n = fsutil::norm_path(&root);
    let (op_id, cancel) = register_op(&state, "op.search");
    let app2 = app.clone();
    let progress2 = progress.clone();
    let res = tauri::async_runtime::spawn_blocking(move || {
        let out = run_search_now(&app2, &query, &root_n, &opts, content, &progress2, &cancel);
        let _ = app2.emit(
            "op-finished",
            OpFinished {
                op_id: op_id.clone(),
                ok: out.is_ok(),
                error: out.as_ref().err().cloned(),
            },
        );
        {
            let st = app2.state::<AppState>();
            unregister_op(&st, &op_id);
        }
        out
    })
    .await
    .map_err(|e| Error::new("internal", "Thread-Pool-Fehler").with("detail", e.to_string()))?;
    res
}

#[tauri::command]
pub async fn search_index(
    app: AppHandle,
    state: State<'_, AppState>,
    query: String,
    root: String,
    opts: SearchOpts,
) -> Result<SearchReturn, Error> {
    let _ = &state;
    let root_n = fsutil::norm_path(&root);
    let app2 = app.clone();
    let res = tauri::async_runtime::spawn_blocking(move || {
        do_search_index(&app2, &query, &root_n, &opts)
    })
    .await
    .map_err(|e| Error::new("internal", "Thread-Pool-Fehler").with("detail", e.to_string()))?;
    res
}
#[cfg(test)]
mod tests {
    use super::*;

    fn opts(name_only: bool, kinds: Vec<String>, max_size: Option<u64>) -> SearchOpts {
        SearchOpts {
            include_hidden: false,
            name_only,
            max_size,
            kinds,
        }
    }

    #[test]
    fn fts_query_quotes_and_joins_tokens() {
        assert_eq!(fts_query("hallo welt"), "\"hallo\" AND \"welt\"");
        // Sonderzeichen werden entfernt, Leer-Token gefiltert
        assert_eq!(fts_query("foo-bar! 123"), "\"foobar\" AND \"123\"");
        assert_eq!(fts_query("!!!"), "");
    }

    #[test]
    fn pass_filters_hidden_and_kinds_and_size() {
        let o = opts(false, vec![], None);
        assert!(!pass_filters(&o, ".bashrc", "text", 10));
        assert!(pass_filters(&o, "lese.klausur.txt", "text", 10));

        let o = opts(false, vec!["image".into()], None);
        assert!(!pass_filters(&o, "a.txt", "text", 0));
        assert!(pass_filters(&o, "a.png", "image", 0));

        let o = opts(false, vec![], Some(100));
        assert!(!pass_filters(&o, "gross.bin", "binary", 101));
        assert!(pass_filters(&o, "klein.bin", "binary", 100));

        // include_hidden erlaubt Punktdateien
        let mut o = opts(false, vec![], None);
        o.include_hidden = true;
        assert!(pass_filters(&o, ".bashrc", "text", 10));
    }

    #[test]
    fn kind_has_text_only_for_text_like() {
        assert!(kind_has_text("text"));
        assert!(kind_has_text("code"));
        assert!(!kind_has_text("binary"));
        assert!(!kind_has_text("image"));
    }

    #[test]
    fn fts5_match_end_to_end() {
        let dir = std::env::temp_dir().join(format!(
            "compound_search_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let db = Connection::open(dir.join("index.db")).unwrap();
        db.execute_batch(
            "CREATE VIRTUAL TABLE files USING fts5(name, content, tokenize='unicode61');",
        )
        .unwrap();
        db.execute(
            "INSERT INTO files(name, content) VALUES (?1, ?2)",
            params!["neues Testament", "Das ist der Inhalt."],
        )
        .unwrap();
        db.execute(
            "INSERT INTO files(name, content) VALUES (?1, ?2)",
            params!["alte Rechnung", "Irrelevant."],
        )
        .unwrap();

        let query = fts_query("testament inhalt");
        let mut stmt = db
            .prepare(&format!("SELECT name FROM files WHERE files MATCH '{query}'"))
            .unwrap();
        let names: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        // FTS5-MATCH mit AND wertet pro Zeile (Name+Inhalt) aus → nur Dokument 1
        assert_eq!(names, vec!["neues Testament"], "nur Zeile 1 muss matchen: {names:?}");

        // Einzelbegriff trifft das andere Dokument
        let query = fts_query("rechnung");
        let mut stmt = db
            .prepare(&format!("SELECT name FROM files WHERE files MATCH '{query}'"))
            .unwrap();
        let names: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(names, vec!["alte Rechnung"], "nur Zeile 2 muss matchen: {names:?}");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
