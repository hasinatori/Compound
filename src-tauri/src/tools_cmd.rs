// Инструменты: дубликаты по хэшу, сравнение папок, массовое переименование.
// Tools: hash-based duplicates, folder compare, mass rename.
// Правила переименования применяем без перезаписи — только считаем и preview.
// Rename rules are applied to a copy — we only count and preview.

use crate::fsutil;
use crate::state::{register_op, unregister_op, AppState, CancelToken};
use crate::types::*;
use crate::ui_error::{self, Error};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, Manager, State};

fn finish(app: &AppHandle, op_id: &str, ok: bool, error: Option<Error>) {
    let _ = app.emit(
        "op-finished",
        OpFinished {
            op_id: op_id.into(),
            ok,
            error,
        },
    );
    let st = app.state::<AppState>();
    unregister_op(&st, op_id);
}

fn hash_file(path: &Path) -> Result<String, Error> {
    use std::io::Read;
    let f = fs::File::open(path).map_err(|e| ui_error::io(path.display(), e))?;
    let mut reader = std::io::BufReader::new(f);
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| ui_error::io(path.display(), e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

fn run_dedup(
    app: &AppHandle,
    root: &str,
    progress: &Channel<ProgressEvent>,
    cancel: &CancelToken,
) -> Result<Vec<DuplicateGroup>, Error> {
    let _ = app;
    // Zähle Kandidaten nach Größe.
    let mut by_size: HashMap<u64, Vec<PathBuf>> = HashMap::new();
    let walk = ignore::WalkBuilder::new(root)
        .hidden(false)
        .follow_links(false)
        .filter_entry(|e| !is_virtual(e.path()))
        .build();
    for entry in walk.filter_map(Result::ok) {
        if cancel.cancelled() {
            return Err(Error::new("canceled", "Abgebrochen"));
        }
        let ft = entry.file_type();
        let Some(ft) = ft else { continue };
        if !ft.is_file() || ft.is_symlink() {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        let size = meta.len();
        if size == 0 {
            continue;
        }
        by_size.entry(size).or_default().push(entry.path().to_path_buf());
    }

    let total_candidates: u64 = by_size
        .values()
        .filter(|v| v.len() > 1)
        .map(|v| v.len() as u64)
        .sum();
    let mut done = 0u64;

    let mut groups: Vec<DuplicateGroup> = Vec::new();
    for (size, paths) in by_size.iter().filter(|(_, v)| v.len() > 1) {
        let mut by_hash: HashMap<String, Vec<String>> = HashMap::new();
        for p in paths {
            if cancel.cancelled() {
                return Err(Error::new("canceled", "Abgebrochen"));
            }
            let h = hash_file(p)?;
            by_hash
                .entry(h)
                .or_default()
                .push(p.display().to_string());
            done += 1;
            if done % 8 == 0 {
                let _ = progress.send(ProgressEvent::new(
                    "dedup",
                    "hash",
                    done,
                    total_candidates,
                    "path",
                    ui_error::par("path", p.display()),
                ));
            }
        }
        for (hash, files) in by_hash {
            if files.len() > 1 {
                groups.push(DuplicateGroup {
                    size: *size,
                    hash,
                    files,
                });
            }
        }
    }
    groups.sort_by(|a, b| b.size.cmp(&a.size));
    let _ = progress.send(ProgressEvent::new(
        "dedup",
        "done",
        done,
        total_candidates,
        "groups",
        ui_error::par("n", groups.len()),
    ));
    Ok(groups)
}

fn is_virtual(path: &Path) -> bool {
    let p = path.to_string_lossy();
    p == "/proc" || p == "/sys" || p == "/dev" || p == "/run"
}

#[tauri::command]
pub async fn find_duplicates(
    app: AppHandle,
    state: State<'_, AppState>,
    root: String,
    progress: Channel<ProgressEvent>,
) -> Result<Vec<DuplicateGroup>, Error> {
    let root_n = fsutil::norm_path(&root);
    let (op_id, cancel) = register_op(&state, "op.dedup");
    let app2 = app.clone();
    let progress2 = progress.clone();
    let res = tauri::async_runtime::spawn_blocking(move || {
        let out = run_dedup(&app2, &root_n, &progress2, &cancel);
        finish(&app2, &op_id, out.is_ok(), out.as_ref().err().cloned());
        out
    })
    .await
    .map_err(|e| Error::new("internal", "Thread-Pool-Fehler").with("detail", e.to_string()))?;
    res
}

fn walk_map(root: &Path, map: &mut HashMap<String, (bool, u64, i64)>) {
    let walk = ignore::WalkBuilder::new(root)
        .hidden(false)
        .follow_links(false)
        .build();
    for entry in walk.filter_map(Result::ok) {
        let Some(rel) = entry.path().strip_prefix(root).ok() else {
            continue;
        };
        let rel_s = rel.to_string_lossy().into_owned();
        if rel_s.is_empty() {
            continue;
        }
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let is_dir = meta.is_dir();
        map.insert(
            rel_s,
            (is_dir, if is_dir { 0 } else { meta.len() }, fsutil::mtime_ms(&meta)),
        );
    }
}

fn run_compare(
    root_a: &str,
    root_b: &str,
    deep: bool,
    progress: &Channel<ProgressEvent>,
    cancel: &CancelToken,
) -> Result<Vec<CompareItem>, Error> {
    let mut map_a: HashMap<String, (bool, u64, i64)> = HashMap::new();
    let mut map_b: HashMap<String, (bool, u64, i64)> = HashMap::new();
    walk_map(Path::new(root_a), &mut map_a);
    if cancel.cancelled() {
        return Err(Error::new("canceled", "Abgebrochen"));
    }
    walk_map(Path::new(root_b), &mut map_b);
    if cancel.cancelled() {
        return Err(Error::new("canceled", "Abgebrochen"));
    }

    let mut keys: Vec<&String> = map_a
        .keys()
        .chain(map_b.keys())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    keys.sort_by(|a, b| a.cmp(b));

    let mut out = Vec::with_capacity(keys.len());
    let total = keys.len() as u64;
    let mut done = 0u64;
    for key in keys {
        if cancel.cancelled() {
            return Err(Error::new("canceled", "Abgebrochen"));
        }
        let (is_dir_a, size_a, mtime_a) = map_a.get(key).map(|x| (Some(x.0), x.1, x.2)).unwrap_or((None, 0, 0));
        let (is_dir_b, size_b, mtime_b) = map_b.get(key).map(|x| (Some(x.0), x.1, x.2)).unwrap_or((None, 0, 0));

        let (status, msg) = match (is_dir_a, is_dir_b) {
            (Some(_), None) => ("only_in_a", String::new()),
            (None, Some(_)) => ("only_in_b", String::new()),
            (Some(da), Some(db)) => {
                if da != db {
                    ("different", String::new())
                } else if da {
                    ("identical", String::new())
                } else if size_a != size_b {
                    ("different", String::new())
                } else if mtime_a != mtime_b {
                    ("different", String::new())
                } else if deep {
                    let ha = hash_file(&Path::new(root_a).join(key)).ok();
                    let hb = hash_file(&Path::new(root_b).join(key)).ok();
                    if ha.is_some() && ha == hb {
                        ("identical", String::new())
                    } else {
                        ("different", String::new())
                    }
                } else {
                    ("identical", String::new())
                }
            }
            _ => ("only_in_a", String::new()),
        };
        out.push(CompareItem {
            relative: key.clone(),
            status: status.into(),
            is_dir: is_dir_a.unwrap_or(false),
            size_a: size_a as i64,
            size_b: size_b as i64,
            mtime_a,
            mtime_b,
            msg,
        });
        done += 1;
        if done % 256 == 0 {
            let _ = progress.send(ProgressEvent::new(
                "compare",
                "compare",
                done,
                total,
                "path",
                ui_error::par("path", key.clone()),
            ));
        }
    }
    Ok(out)
}

#[tauri::command]
pub async fn compare_folders(
    app: AppHandle,
    state: State<'_, AppState>,
    a: String,
    b: String,
    deep: bool,
    progress: Channel<ProgressEvent>,
) -> Result<Vec<CompareItem>, Error> {
    if !Path::new(&a).is_dir() || !Path::new(&b).is_dir() {
        return Err(Error::new("bothFoldersRequired", "Beide Ordner müssen existieren."));
    }
    let (op_id, cancel) = register_op(&state, "op.compare");
    let app2 = app.clone();
    let progress2 = progress.clone();
    let res = tauri::async_runtime::spawn_blocking(move || {
        let out = run_compare(&a, &b, deep, &progress2, &cancel);
        finish(&app2, &op_id, out.is_ok(), out.as_ref().err().cloned());
        out
    })
    .await
    .map_err(|e| Error::new("internal", "Thread-Pool-Fehler").with("detail", e.to_string()))?;
    res
}

fn apply_rules(name: &str, is_dir: bool, rules: &[RenameRule], counter: &mut u32) -> Result<String, Error> {
    let mut stem = name.to_string();
    let mut ext = String::new();
    if !is_dir {
        if let Some(i) = stem.rfind('.') {
            if i > 0 {
                ext = stem[i..].to_string();
                stem.truncate(i);
            }
        }
    }
    let orig_stem = stem.clone();
    let orig_ext = ext.clone();

    for rule in rules {
        match rule.rule_type.as_str() {
            "find_replace" => {
                if let (Some(f), Some(r)) = (&rule.find, &rule.replace) {
                    stem = stem.replace(f, r);
                }
            }
            "regex" => {
                if let Some(f) = &rule.find {
                    let re = regex::Regex::new(f)
                        .map_err(|e| Error::new("regexError", format!("Regex-Fehler: {e}")).with("detail", e.to_string()))?;
                    stem = re
                        .replace_all(&stem, rule.replace.as_deref().unwrap_or(""))
                        .into_owned();
                }
            }
            "remove" => {
                if let Some(f) = &rule.find {
                    stem = stem.replace(f, "");
                }
            }
            "insert" => {
                if let (Some(at), Some(text)) = (rule.insert_at, &rule.insert_text) {
                    let mut chars: Vec<char> = stem.chars().collect();
                    let at = (at as usize).min(chars.len());
                    let to_insert: Vec<char> = text.chars().collect();
                    for (i, c) in to_insert.iter().enumerate() {
                        let pos = (at + i).min(chars.len());
                        chars.insert(pos, *c);
                    }
                    stem = chars.into_iter().collect();
                }
            }
            "case" => match rule.case_mode.as_deref().unwrap_or("lower") {
                "upper" => stem = stem.to_uppercase(),
                "lower" => stem = stem.to_lowercase(),
                "title" => {
                    stem = stem
                        .split(' ')
                        .map(|w| {
                            let mut c = w.chars();
                            match c.next() {
                                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                                None => String::new(),
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" ");
                }
                _ => {}
            },
            "ext" => {
                if !is_dir {
                    if let Some(ne) = &rule.new_ext {
                        ext = format!(".{}", ne.trim_start_matches('.'));
                        if ext == "." {
                            ext = String::new();
                        }
                    }
                }
            }
            "numbering" => {
                let template = rule
                    .template
                    .clone()
                    .unwrap_or_else(|| "{name}_{n}".into());
                let start = rule.start.unwrap_or(1);
                let step = rule.step.unwrap_or(1).max(1);
                let digits = rule.digits.unwrap_or(2);
                let num = *counter as u64;
                *counter += 1;
                let n = format!("{:0width$}", start as u64 + num * step as u64, width = digits as usize);
                stem = template
                    .replace("{n}", &n)
                    .replace("{name}", &orig_stem)
                    .replace("{ext}", orig_ext.trim_start_matches('.'));
            }
            _ => {}
        }
    }
    Ok(if is_dir { stem } else { format!("{stem}{ext}") })
}

#[tauri::command]
pub fn multi_rename(
    items: Vec<String>,
    rules: Vec<RenameRule>,
    dry_run: bool,
) -> Result<Vec<RenamePreviewItem>, Error> {
    let mut out = Vec::with_capacity(items.len());
    let mut counter = 0u32;
    let mut seen_targets: std::collections::HashSet<String> = std::collections::HashSet::new();

    for item in &items {
        let p = Path::new(item);
        let meta = match fs::metadata(p) {
            Ok(m) => m,
            Err(e) => {
                out.push(RenamePreviewItem {
                    from: item.clone(),
                    to: item.clone(),
                    conflict: false,
                    error: Some(ui_error::io(item.clone(), e)),
                });
                continue;
            }
        };
        let is_dir = meta.is_dir();
        let name = p
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let new_name = match apply_rules(&name, is_dir, &rules, &mut counter) {
            Ok(n) => n,
            Err(e) => {
                out.push(RenamePreviewItem {
                    from: item.clone(),
                    to: item.clone(),
                    conflict: false,
                    error: Some(e),
                });
                continue;
            }
        };

        let mut conflict = false;
        let final_path;
        if !dry_run && new_name != name {
            let parent = p.parent().unwrap_or(Path::new("/"));
            let mut target = parent.join(&new_name);
            if target.exists() && target != p {
                // Automatisch eindeutig machen.
                target = fsutil::unique_dest(parent, &new_name);
            }
            match fs::rename(p, &target) {
                Ok(_) => final_path = target.display().to_string(),
                Err(e) => {
                    out.push(RenamePreviewItem {
                        from: item.clone(),
                        to: item.clone(),
                        conflict: true,
                        error: Some(ui_error::io(item.clone(), e)),
                    });
                    continue;
                }
            }
        } else {
            if new_name != name {
                conflict = seen_targets.contains(&new_name) || Path::new(item).parent().map(|par| par.join(&new_name)).map(|t| t.exists()).unwrap_or(false);
                seen_targets.insert(new_name.clone());
            }
            final_path = Path::new(item)
                .parent()
                .unwrap_or(Path::new("/"))
                .join(&new_name)
                .display()
                .to_string();
        }
        out.push(RenamePreviewItem {
            from: item.clone(),
            to: final_path,
            conflict,
            error: None,
        });
    }
    Ok(out)
}