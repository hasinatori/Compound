// Команды чтения ФС: листинг, места, путь, размер диска, превью, открыть.
// FS read commands: listing, places, path parts, disk usage, thumb, open.
// Только чтение — мутации живут в ops_cmd.rs.
// Read-only here; mutations live in ops_cmd.rs.

use crate::fsutil;
use crate::state::AppState;
use crate::types::*;
use crate::ui_error::{self, Error};
use std::cmp::Ordering;
use std::fs;
use std::path::Path;
use tauri::State;

fn nat_cmp(a: &str, b: &str) -> Ordering {
    let mut ai = a.chars().peekable();
    let mut bi = b.chars().peekable();
    loop {
        let ac = ai.next();
        let bc = bi.next();
        match (ac, bc) {
            (None, None) => return Ordering::Equal,
            (None, _) => return Ordering::Less,
            (_, None) => return Ordering::Greater,
            (Some(x), Some(y)) => {
                let xa = x.to_ascii_lowercase();
                let ya = y.to_ascii_lowercase();
                if xa.is_ascii_digit() && ya.is_ascii_digit() {
                    let (na, multi) = gather_num(&mut ai, xa);
                    let (nb, _) = gather_num(&mut bi, ya);
                    let ord = na.cmp(&nb);
                    if ord != Ordering::Equal {
                        return ord;
                    }
                    if !multi {
                        continue;
                    }
                    continue;
                }
                let ord = xa.cmp(&ya);
                if ord != Ordering::Equal {
                    return ord;
                }
            }
        }
    }
}

fn gather_num(iter: &mut std::iter::Peekable<std::str::Chars>, first: char) -> (u64, bool) {
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
    (s.parse::<u64>().unwrap_or(0), s.len() > 1)
}

fn compare_entries(a: &FileEntry, b: &FileEntry) -> Ordering {
    match (a.is_dir, b.is_dir) {
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        _ => nat_cmp(&a.name, &b.name),
    }
}

#[tauri::command]
pub fn list_dir(path: String) -> Result<Vec<FileEntry>, Error> {
    if path.starts_with(crate::ops_cmd::TRASH_VIRTUAL) {
        return Ok(list_trash_entries());
    }
    let mut entries = fsutil::read_dir_entries(Path::new(&path))?;
    entries.sort_by(compare_entries);
    Ok(entries)
}

fn list_trash_entries() -> Vec<FileEntry> {
    let items = crate::ops_cmd::list_trash_inner();
    items
        .into_iter()
        .map(|t| FileEntry {
            path: format!("{}{}", crate::ops_cmd::TRASH_VIRTUAL, t.name_in_trash),
            name: t
                .original_path
                .rsplit('/')
                .next()
                .unwrap_or(&t.name_in_trash)
                .to_string(),
            ext: fsutil::file_ext(&t.original_path),
            is_dir: t.is_dir,
            is_symlink: false,
            is_hidden: false,
            size: 0,
            mtime_ms: t.trashed_at_ms,
            perms: "--------".into(),
            kind: if t.is_dir { "dir".into() } else { "trash".into() },
            link_target: Some(t.original_path),
        })
        .collect()
}

#[tauri::command]
pub fn get_places() -> Vec<Place> {
    let mut places = Vec::new();
    if let Some(home) = dirs::home_dir() {
        places.push(Place {
            name: "Home".into(),
            path: home.display().to_string(),
            kind: "home".into(),
        });
        let xdg: &[(&str, fn() -> Option<std::path::PathBuf>)] = &[
            ("Desktop", dirs::desktop_dir),
            ("Dokumente", dirs::document_dir),
            ("Downloads", dirs::download_dir),
            ("Bilder", dirs::picture_dir),
            ("Musik", dirs::audio_dir),
            ("Videos", dirs::video_dir),
        ];
        for (label, f) in xdg {
            if let Some(p) = f() {
                if p.exists() {
                    places.push(Place {
                        name: (*label).into(),
                        path: p.display().to_string(),
                        kind: "folder".into(),
                    });
                }
            }
        }
    }
    places.push(Place {
        name: "Papierkorb".into(),
        path: crate::ops_cmd::TRASH_VIRTUAL.into(),
        kind: "trash".into(),
    });
    #[cfg(unix)]
    {
        let home = dirs::home_dir().unwrap_or_default();
        if let Ok(mounts) = fs::read_to_string("/proc/mounts") {
            let mut seen: Vec<String> = Vec::new();
            for line in mounts.lines() {
                let mut parts = line.split_whitespace();
                let Some(src) = parts.next() else { continue };
                let Some(mp) = parts.next() else { continue };
                let Some(fstype) = parts.next() else { continue };
                let allowed = matches!(
                    fstype,
                    "ext4" | "ext3" | "ext2" | "btrfs" | "xfs" | "f2fs" | "ntfs" | "ntfs3"
                        | "vfat" | "exfat" | "fuseblk" | "reiserfs" | "jfs" | "zfs" | "bcachefs"
                );
                if !allowed || !src.starts_with('/') || mp.contains(' ') {
                    continue;
                }
                if src.starts_with("/dev/loop") {
                    continue;
                }
                if seen.contains(&mp.to_string()) {
                    continue;
                }
                seen.push(mp.to_string());
                let name = if mp == "/" {
                    "Dateisystem-Root".into()
                } else {
                    let base = mp.rsplit('/').next().unwrap_or(mp).to_string();
                    if base.is_empty() {
                        mp.trim_start_matches('/').to_string()
                    } else {
                        base
                    }
                };
                let home_s = home.to_string_lossy().into_owned();
                let is_home = home.starts_with(mp) && mp != "/";
                let is_prefixed = mp.starts_with(&home_s);
                if is_home || is_prefixed {
                    continue;
                }
                places.push(Place {
                    name,
                    path: mp.to_string(),
                    kind: "mount".into(),
                });
            }
        }
    }
    places.push(Place {
        name: "Dateisystem-Root".into(),
        path: "/".into(),
        kind: "root".into(),
    });
    places.sort_by(|a, b| {
        let ak = place_order(&a.kind);
        let bk = place_order(&b.kind);
        ak.cmp(&bk).then_with(|| nat_cmp(&a.name, &b.name))
    });
    places
}

fn place_order(kind: &str) -> u8 {
    match kind {
        "home" => 0,
        "folder" => 1,
        "trash" => 2,
        "mount" => 3,
        "root" => 4,
        _ => 5,
    }
}

#[tauri::command]
pub fn path_parts(path: String) -> Vec<PathPart> {
    let path = fsutil::norm_path(&path);
    if path == "/" {
        return vec![PathPart {
            name: "/".into(),
            path: "/".into(),
        }];
    }
    let mut parts = Vec::new();
    let mut acc = String::new();
    for seg in path.split('/').filter(|s| !s.is_empty()) {
        acc.push('/');
        acc.push_str(seg);
        parts.push(PathPart {
            name: seg.to_string(),
            path: acc.clone(),
        });
    }
    parts
}

#[tauri::command]
pub fn disk_usage(path: String) -> Result<DiskUsage, Error> {
    let p = Path::new(&path);
    let probe = if p.is_dir() { p } else { p.parent().unwrap_or(p) };
    let cstr = std::ffi::CString::new(probe.display().to_string())
        .map_err(|e| Error::new("io", format!("Pfad: {e}")).with("detail", e.to_string()))?;
    let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::statvfs(cstr.as_ptr(), &mut st) };
    if rc != 0 {
        return Err(
            Error::new("statvfs", format!("statvfs fehlgeschlagen ({rc}): {}", probe.display()))
                .with("rc", rc)
                .with("path", probe.display()),
        );
    }
    let bsize = st.f_frsize as u64;
    Ok(DiskUsage {
        total: st.f_blocks.saturating_mul(bsize),
        free: st.f_bavail.saturating_mul(bsize),
    })
}

fn fit_dim(w: u32, h: u32, max: u32) -> (u32, u32) {
    if w == 0 || h == 0 {
        return (max, max);
    }
    let scale = (max as f32 / w.max(h) as f32).min(1.0);
    (
        ((w as f32 * scale).round() as u32).max(1),
        ((h as f32 * scale).round() as u32).max(1),
    )
}

/// Делает (и кэширует) превью, отдаёт путь в кэше.
/// Creates and caches a thumbnail, returns the cache file path.
/// UI грузит его через asset-протокол (convertFileSrc).
/// The UI loads it through the asset protocol (convertFileSrc).
#[tauri::command]
pub fn thumb(state: State<'_, AppState>, path: String) -> Result<String, Error> {
    let p = Path::new(&path);
    let meta = fs::metadata(p).map_err(|e| ui_error::io(path.clone(), e))?;
    if !meta.is_file() {
        return Err(Error::new("noFile", "Keine Datei"));
    }
    let mtime = fsutil::mtime_ms(&meta);
    let mut hasher = blake3::Hasher::new();
    hasher.update(path.as_bytes());
    hasher.update(&mtime.to_le_bytes());
    let key = hasher.finalize().to_hex();
    let thumb_file = state.cache_dir.join("thumbs").join(format!("{key}.png"));
    if !thumb_file.exists() {
        let img = image::ImageReader::open(&p)
            .map_err(|e| Error::new("imageRead", format!("Bild nicht lesbar: {}", e)).with("path", &path).with("detail", e.to_string()))?
            .decode()
            .map_err(|e| Error::new("imageDecode", format!("Bild nicht dekodierbar: {}", e)).with("path", &path).with("detail", e.to_string()))?;
        let (w, h) = (img.width(), img.height());
        let (nw, nh) = fit_dim(w, h, 256);
        let thumb = image::imageops::resize(&img, nw, nh, image::imageops::FilterType::Triangle);
        thumb
            .save_with_format(&thumb_file, image::ImageFormat::Png)
            .map_err(|e| Error::new("thumbnail", format!("Thumbnail: {}", e)).with("path", &path).with("detail", e.to_string()))?;
    }
    Ok(thumb_file.display().to_string())
}

/// Открывает файл/папку программой по умолчанию.
/// Opens a file/folder with the default application.
#[tauri::command]
pub fn open_default(app: tauri::AppHandle, path: String) -> Result<(), Error> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_path(path.clone(), None::<&str>)
        .map_err(|e| Error::new("io", format!("Öffnen: {e}")).with("path", path).with("detail", e.to_string()))
}