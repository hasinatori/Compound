use crate::fsutil;
use crate::state::{register_op, unregister_op, AppState, CancelToken};
use crate::types::*;
use crate::ui_error::{self, Error};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, Manager, State};

fn emit(
    progress: &Channel<ProgressEvent>,
    op: &str,
    phase: &str,
    current: u64,
    total: u64,
    kind: &str,
    params: std::collections::BTreeMap<String, String>,
) {
    let _ = progress.send(ProgressEvent::new(op, phase, current, total, kind, params));
}

fn throttle(last: &mut Instant) -> bool {
    if last.elapsed() >= Duration::from_millis(100) {
        *last = Instant::now();
        true
    } else {
        false
    }
}

fn which(bin: &str) -> Option<PathBuf> {
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
}

fn is_dir_archive(archive: &Path) -> Result<(), Error> {
    if !archive.is_file() {
        return Err(
            Error::new("archiveMissing", format!("Archiv existiert nicht: {}", archive.display()))
                .with("path", archive.display()),
        );
    }
    Ok(())
}

fn arch_ext(archive: &Path) -> String {
    let name = archive.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let lower = name.to_lowercase();
    for pat in [
        "tar.gz", "tar.bz2", "tar.xz", ".tgz", ".tbz2", ".txz", ".tar", ".zip", ".7z",
        ".rar", ".gz", ".bz2", ".xz", ".zst", ".iso", ".cab",
    ] {
        if lower.ends_with(pat) {
            return pat.trim_start_matches('.').to_string();
        }
    }
    fsutil::file_ext(&name)
}

/// Liste der Archiveinträge (für Vorschau).
#[tauri::command]
pub fn archive_list(archive: String) -> Result<Vec<ArchiveEntry>, Error> {
    let p = Path::new(&archive);
    is_dir_archive(p)?;
    match arch_ext(p).as_str() {
        "zip" => zip_list(p),
        "tar" | "tar.gz" | "tgz" | "tar.bz2" | "tbz2" | "tar.xz" | "txz" => tar_list(p),
        _ => sevenz_list(p),
    }
}

fn zip_list(p: &Path) -> Result<Vec<ArchiveEntry>, Error> {
    let f = File::open(p).map_err(|e| ui_error::io(p.display(), e))?;
    let mut z = zip::ZipArchive::new(f).map_err(|e| ui_error::io(p.display(), e))?;
    let mut out = Vec::with_capacity(z.len());
    for i in 0..z.len() {
        let inner = z.by_index(i).map_err(|e| ui_error::io(p.display(), e))?;
        out.push(ArchiveEntry {
            path: inner.name().to_string(),
            size: inner.size(),
            is_dir: inner.is_dir(),
        });
    }
    Ok(out)
}

fn tar_list(p: &Path) -> Result<Vec<ArchiveEntry>, Error> {
    let open: Box<dyn Read> = match arch_ext(p).as_str() {
        "tar.gz" | "tgz" => Box::new(flate2::read::GzDecoder::new(File::open(p).map_err(|e| ui_error::io(p.display(), e))?)),
        "tar.bz2" | "tbz2" => Box::new(bzip2::read::BzDecoder::new(File::open(p).map_err(|e| ui_error::io(p.display(), e))?)),
        "tar.xz" | "txz" => Box::new(xz2::read::XzDecoder::new(File::open(p).map_err(|e| ui_error::io(p.display(), e))?)),
        _ => Box::new(File::open(p).map_err(|e| ui_error::io(p.display(), e))?),
    };
    let mut ar = tar::Archive::new(open);
    let mut out = Vec::new();
    for entry in ar.entries().map_err(|e| ui_error::io(p.display(), e))? {
        let entry = entry.map_err(|e| ui_error::io(p.display(), e))?;
        let path = entry
            .path()
            .map_err(|e| ui_error::io(p.display(), e))?
            .display()
            .to_string();
        let is_dir = entry.header().entry_type().is_dir();
        out.push(ArchiveEntry {
            path,
            size: entry.size(),
            is_dir,
        });
    }
    Ok(out)
}

fn sevenz_list(p: &Path) -> Result<Vec<ArchiveEntry>, Error> {
    let bin = ["7z", "7zz", "7za"]
        .iter()
        .find_map(|b| which(b))
        .ok_or_else(|| Error::new("sevenZipMissing", "7z nicht gefunden (pacman -S p7zip)"))?;
    let out = std::process::Command::new(&bin)
        .arg("l")
        .arg("-slt")
        .arg(p)
        .output()
        .map_err(|e| ui_error::io(p.display(), e))?;
    let text = String::from_utf8_lossy(&out.stdout);
    let mut entries = Vec::new();
    let mut path = String::new();
    let mut size: u64 = 0;
    let mut is_dir = false;
    let mut in_entry = false;
    for line in text.lines() {
        if let Some(v) = line.strip_prefix("Path = ") {
            path = v.trim().to_string();
            in_entry = true;
            is_dir = false;
            size = 0;
        } else if let Some(v) = line.strip_prefix("Size = ") {
            size = v.trim().parse().unwrap_or(0);
        } else if let Some(v) = line.strip_prefix("Attributes = ") {
            is_dir = v.contains('D');
        } else if line.is_empty() && in_entry {
            entries.push(ArchiveEntry { path: path.clone(), size, is_dir });
            in_entry = false;
        }
    }
    if in_entry {
        entries.push(ArchiveEntry { path, size, is_dir });
    }
    Ok(entries)
}

/// Erstellt ein Archiv (zip/tar/tar.gz/tar.bz2/tar.xz/7z).
#[tauri::command]
pub async fn archive_create(
    app: AppHandle,
    state: State<'_, AppState>,
    srcs: Vec<String>,
    dest: String,
    progress: Channel<ProgressEvent>,
) -> Result<ArchiveSummary, Error> {
    if srcs.is_empty() {
        return Err(Error::new("nothingToPack", "Nichts zum Verpacken ausgewählt."));
    }
    let total = total_bytes(&srcs);
    let (op_id, cancel) = register_op(&state, "op.archive");
    let app2 = app.clone();
    let p2 = progress.clone();
    let res = tauri::async_runtime::spawn_blocking(move || {
        let out = run_create(&srcs, &dest, total, &p2, &cancel);
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
    .map_err(|e| Error::new("internal", format!("Hintergrund-Fehler: {e}")).with("detail", e.to_string()))?;
    res
}

/// Extrahiert ein Archiv in einen Zielordner.
#[tauri::command]
pub async fn archive_extract(
    app: AppHandle,
    state: State<'_, AppState>,
    archive: String,
    dest_dir: String,
    progress: Channel<ProgressEvent>,
) -> Result<ArchiveSummary, Error> {
    is_dir_archive(Path::new(&archive))?;
    let dest = PathBuf::from(&dest_dir);
    fs::create_dir_all(&dest).map_err(|e| ui_error::io(dest.display(), e))?;
    let (op_id, cancel) = register_op(&state, "op.extract");
    let app2 = app.clone();
    let p2 = progress.clone();
    let res = tauri::async_runtime::spawn_blocking(move || {
        let out = run_extract(&archive, &dest, &p2, &cancel);
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
    .map_err(|e| Error::new("internal", format!("Hintergrund-Fehler: {e}")).with("detail", e.to_string()))?;
    res
}

fn total_bytes(srcs: &[String]) -> u64 {
    srcs.iter().fold(0u64, |acc, s| acc + path_bytes(Path::new(s)))
}

fn path_bytes(p: &Path) -> u64 {
    let meta = match fs::metadata(p) {
        Ok(m) => m,
        Err(_) => return 0,
    };
    if meta.is_file() {
        return meta.len();
    }
    if meta.is_dir() {
        let mut total = 0u64;
        if let Ok(rd) = fs::read_dir(p) {
            for e in rd.flatten() {
                total += path_bytes(&e.path());
            }
        }
        return total;
    }
    0
}

fn collect_files(srcs: &[String], base: &Path) -> Result<Vec<(PathBuf, String, bool)>, Error> {
    // (absoluter Pfad, relativer Pfad im Archiv, is_dir)
    fn walk(
        p: &Path,
        base: &Path,
        out: &mut Vec<(PathBuf, String, bool)>,
    ) -> Result<(), Error> {
        let rel = p.strip_prefix(base).unwrap_or(p);
        let mut rel_s = rel.to_string_lossy().into_owned();
        if rel_s.is_empty() {
            // Quelle ist selbst die Basis: Ordnernamen als Wurzeleintrag nehmen.
            rel_s = p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| p.display().to_string());
        }
        let meta = fs::symlink_metadata(p).map_err(|e| ui_error::io(p.display(), e))?;
        if meta.file_type().is_dir() {
            out.push((p.to_path_buf(), rel_s, true));
            for e in fs::read_dir(p)
                .map_err(|e| ui_error::io(p.display(), e))?
                .flatten()
            {
                walk(&e.path(), base, out)?;
            }
        } else {
            out.push((p.to_path_buf(), rel_s, false));
        }
        Ok(())
    }
    let mut out = Vec::new();
    for s in srcs {
        walk(Path::new(s), base, &mut out)?;
    }
    Ok(out)
}

/// Basis für Archivpfade: bei einer einzelnen Ordner-Quelle auf das
/// Elternverzeichnis anheben, damit der Ordnername im Archiv erhalten bleibt
/// und keine leeren Wurzelpfade entstehen (tar-Fehler „at least one component“).
fn archive_base(srcs: &[String]) -> PathBuf {
    let base = common_base(srcs);
    if srcs.len() == 1 {
        let p = Path::new(&srcs[0]);
        if p.is_dir() {
            if let Some(parent) = p.parent() {
                return parent.to_path_buf();
            }
        }
    }
    base
}

fn run_create(
    srcs: &[String],
    dest: &str,
    total: u64,
    progress: &Channel<ProgressEvent>,
    cancel: &CancelToken,
) -> Result<ArchiveSummary, Error> {
    let dest_p = Path::new(dest);
    // Basis = gemeinsames übergeordnetes Verzeichnis der Quellen.
    let base = archive_base(srcs);
    let ext = arch_ext(dest_p);
    let bytes_done = AtomicU64::new(0);
    let callbacks = |label: String| -> Result<(), Error> {
        if cancel.cancelled() {
            return Err(Error::new("canceled", "Abgebrochen"));
        }
        let cur = bytes_done.load(Ordering::Relaxed);
        emit(
            progress,
            "archive",
            "write",
            cur,
            total,
            "path",
            ui_error::par("path", label),
        );
        Ok(())
    };

    match ext.as_str() {
        "zip" => create_zip(srcs, dest_p, &base, &bytes_done, &callbacks),
        "tar.gz" | "tgz" => create_tar(srcs, dest_p, &base, Some("gz"), &bytes_done, &callbacks),
        "tar.bz2" | "tbz2" => create_tar(srcs, dest_p, &base, Some("bz2"), &bytes_done, &callbacks),
        "tar.xz" | "txz" => create_tar(srcs, dest_p, &base, Some("xz"), &bytes_done, &callbacks),
        "tar" => create_tar(srcs, dest_p, &base, None, &bytes_done, &callbacks),
        "7z" => create_7z(srcs, dest_p, &base, progress, cancel),
        other => Err(
            Error::new(
                "formatUnsupported",
                format!(
                    "Format „{}“ nicht unterstützt (zip, tar, tar.gz, tar.bz2, tar.xz, 7z)",
                    other
                ),
            )
            .with("format", other),
        ),
    }
}

fn common_base(srcs: &[String]) -> PathBuf {
    let first = Path::new(&srcs[0]);
    let first_file = if first.is_dir() { first } else { first.parent().unwrap_or(first) };
    let mut base = first_file.to_path_buf();
    for s in &srcs[1..] {
        let p = Path::new(s);
        let p = if p.is_dir() { p } else { p.parent().unwrap_or(p) };
        while !p.starts_with(&base) {
            if !base.pop() {
                break;
            }
        }
    }
    if base.as_os_str().is_empty() {
        PathBuf::from("/")
    } else {
        base
    }
}

fn create_zip<F>(
    srcs: &[String],
    dest: &Path,
    base: &Path,
    bytes_done: &AtomicU64,
    cb: &F,
) -> Result<ArchiveSummary, Error>
where
    F: Fn(String) -> Result<(), Error>,
{
    let mut entries = 0u64;
    let file = File::create(dest).map_err(|e| ui_error::io(dest.display(), e))?;
    let mut zipw = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);

    let files = collect_files(srcs, base)?;
    for (p, rel, is_dir) in files {
        if is_dir {
            zipw.start_file(format!("{}/", rel.trim_end_matches('/')), opts)
                .map_err(|e| ui_error::io(dest.display(), e))?;
        } else {
            zipw.start_file(rel.clone(), opts)
                .map_err(|e| ui_error::io(dest.display(), e))?;
            let mut reader =
                BufReader::new(File::open(&p).map_err(|e| ui_error::io(p.display(), e))?);
            let mut buf = vec![0u8; 1024 * 1024];
            loop {
                let n = reader.read(&mut buf).map_err(|e| ui_error::io(p.display(), e))?;
                if n == 0 {
                    break;
                }
                zipw.write_all(&buf[..n])
                    .map_err(|e| ui_error::io(dest.display(), e))?;
                bytes_done.fetch_add(n as u64, Ordering::Relaxed);
            }
            cb(rel.clone())?;
            entries += 1;
        }
    }
    zipw.finish().map_err(|e| ui_error::io(dest.display(), e))?;
    Ok(ArchiveSummary {
        entries,
        bytes: bytes_done.load(Ordering::Relaxed),
    })
}

fn create_tar<F>(
    srcs: &[String],
    dest: &Path,
    base: &Path,
    compress: Option<&str>,
    bytes_done: &AtomicU64,
    cb: &F,
) -> Result<ArchiveSummary, Error>
where
    F: Fn(String) -> Result<(), Error>,
{
    use tar::Builder;
    let file = File::create(dest).map_err(|e| ui_error::io(dest.display(), e))?;
    let mut builder: Builder<Box<dyn Write>> = match compress {
        Some("gz") => Builder::new(Box::new(flate2::write::GzEncoder::new(file, flate2::Compression::default()))),
        Some("bz2") => Builder::new(Box::new(bzip2::write::BzEncoder::new(file, bzip2::Compression::default()))),
        Some("xz") => Builder::new(Box::new(xz2::write::XzEncoder::new(file, 6))),
        _ => Builder::new(Box::new(file)),
    };
    builder.mode(tar::HeaderMode::Complete);

    let files = collect_files(srcs, base)?;
    let mut entries = 0u64;
    for (p, rel, _is_dir) in files {
        if cancel_state(cb, rel.clone())? {
            return Err(Error::new("canceled", "Abgebrochen"));
        }
        // append_path_with_name kopiert Größe in den Header – Größe via Cluster.
        let size = path_bytes(&p);
        builder
            .append_path_with_name(&p, &rel)
            .map_err(|e| ui_error::io(rel.clone(), e))?;
        bytes_done.fetch_add(size, Ordering::Relaxed);
        entries += 1;
    }
    let inner = builder.into_inner().map_err(|e| ui_error::io(dest.display(), e))?;
    // Encoder finalisieren (gzip/bz2/xz Trailer).
    let mut inner = inner;
    let _ = inner.flush();
    drop(inner);
    Ok(ArchiveSummary {
        entries,
        bytes: bytes_done.load(Ordering::Relaxed),
    })
}

fn cancel_state<F>(cb: &F, label: String) -> Result<bool, Error>
where
    F: Fn(String) -> Result<(), Error>,
{
    cb(label)?;
    Ok(false)
}

fn create_7z(
    srcs: &[String],
    dest: &Path,
    base: &Path,
    progress: &Channel<ProgressEvent>,
    cancel: &CancelToken,
) -> Result<ArchiveSummary, Error> {
    let bin = ["7z", "7zz", "7za"]
        .iter()
        .find_map(|b| which(b))
        .ok_or_else(|| Error::new("sevenZipMissing", "7z nicht gefunden (sudo pacman -S p7zip)"))?;
    let files: Vec<String> = srcs
        .iter()
        .map(|s| {
            Path::new(s)
                .strip_prefix(base)
                .unwrap_or(Path::new(s))
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    let mut child = std::process::Command::new(&bin)
        .arg("a")
        .arg("-y")
        .arg("-bb0")
        .arg(dest)
        .args(&files)
        .current_dir(base)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| ui_error::io(bin.display(), e))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| Error::new("sevenZipOutput", "7z-Ausgabe fehlgeschlagen"))?;
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line).map_err(|e| ui_error::io(bin.display(), e))?;
        if n == 0 {
            break;
        }
        if cancel.cancelled() {
            let _ = child.kill();
            return Err(Error::new("canceled", "Abgebrochen"));
        }
        if let Some(pct) = line.trim().strip_suffix('%') {
            if let Ok(p) = pct.trim().parse::<u64>() {
                emit(
                    progress,
                    "archive",
                    "write",
                    p,
                    100,
                    "pct",
                    ui_error::par("pct", p),
                );
            }
        }
    }
    let status = child.wait().map_err(|e| ui_error::io(bin.display(), e))?;
    if !status.success() {
        return Err(Error::new("sevenZipFailed", "7z fehlgeschlagen."));
    }
    Ok(ArchiveSummary {
        entries: collect_files(srcs, base)?.len() as u64,
        bytes: dest.metadata().map(|m| m.len()).unwrap_or(0),
    })
}

fn run_extract(
    archive: &str,
    dest: &Path,
    progress: &Channel<ProgressEvent>,
    cancel: &CancelToken,
) -> Result<ArchiveSummary, Error> {
    let p = Path::new(archive);
    match arch_ext(p).as_str() {
        "zip" => extract_zip(p, dest, progress, cancel),
        "tar" | "tar.gz" | "tgz" | "tar.bz2" | "tbz2" | "tar.xz" | "txz" => extract_tar(p, dest, progress, cancel),
        _ => extract_7z(p, dest, progress, cancel),
    }
}

fn extract_zip(
    p: &Path,
    dest: &Path,
    progress: &Channel<ProgressEvent>,
    cancel: &CancelToken,
) -> Result<ArchiveSummary, Error> {
    let f = File::open(p).map_err(|e| ui_error::io(p.display(), e))?;
    let mut z = zip::ZipArchive::new(f).map_err(|e| ui_error::io(p.display(), e))?;
    let total = z.len() as u64;
    let mut entries = 0u64;
    let mut last = Instant::now();
    for i in 0..z.len() {
        if cancel.cancelled() {
            return Err(Error::new("canceled", "Abgebrochen"));
        }
        let mut inner = z.by_index(i).map_err(|e| ui_error::io(p.display(), e))?;
        let name = inner.name().to_string();
        if name.ends_with('/') {
            fs::create_dir_all(dest.join(&name)).map_err(|e| {
                ui_error::io(dest.join(&name).display(), e)
            })?;
        } else {
            let target = safe_join(dest, &name)?;
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| ui_error::io(parent.display(), e))?;
            }
            let mut out = File::create(&target)
                .map_err(|e| ui_error::io(target.display(), e))?;
            std::io::copy(&mut inner, &mut out)
                .map_err(|e| ui_error::io(target.display(), e))?;
        }
        entries += 1;
        if throttle(&mut last) {
            emit(
                progress,
                "extract",
                "extract",
                entries,
                total,
                "path",
                ui_error::par("path", name),
            );
        }
    }
    Ok(ArchiveSummary {
        entries,
        bytes: 0,
    })
}

fn extract_tar(
    p: &Path,
    dest: &Path,
    progress: &Channel<ProgressEvent>,
    cancel: &CancelToken,
) -> Result<ArchiveSummary, Error> {
    let open: Box<dyn Read> = match arch_ext(p).as_str() {
        "tar.gz" | "tgz" => Box::new(flate2::read::GzDecoder::new(File::open(p).map_err(|e| ui_error::io(p.display(), e))?)),
        "tar.bz2" | "tbz2" => Box::new(bzip2::read::BzDecoder::new(File::open(p).map_err(|e| ui_error::io(p.display(), e))?)),
        "tar.xz" | "txz" => Box::new(xz2::read::XzDecoder::new(File::open(p).map_err(|e| ui_error::io(p.display(), e))?)),
        _ => Box::new(File::open(p).map_err(|e| ui_error::io(p.display(), e))?),
    };
    let mut ar = tar::Archive::new(open);
    let mut total = 0u64;
    let entries = ar
        .entries()
        .map_err(|e| ui_error::io(p.display(), e))?;
    let mut count = 0u64;
    for entry in entries {
        if cancel.cancelled() {
            return Err(Error::new("canceled", "Abgebrochen"));
        }
        let mut entry = match entry {
            Ok(e) => e,
            Err(e) => return Err(ui_error::io(p.display(), e)),
        };
        total += 1;
        total = total.max(count);
        let name = entry
            .path()
            .map_err(|e| ui_error::io(p.display(), e))?
            .display()
            .to_string();
        entry
            .unpack_in(dest)
            .map_err(|e| ui_error::io(name.clone(), e))?;
        count += 1;
        if count % 64 == 0 {
            emit(
                progress,
                "extract",
                "extract",
                count,
                total.max(count),
                "path",
                ui_error::par("path", name),
            );
        }
    }
    Ok(ArchiveSummary {
        entries: total,
        bytes: 0,
    })
}

fn extract_7z(
    p: &Path,
    dest: &Path,
    progress: &Channel<ProgressEvent>,
    cancel: &CancelToken,
) -> Result<ArchiveSummary, Error> {
    let bin = ["7z", "7zz", "7za"]
        .iter()
        .find_map(|b| which(b))
        .ok_or_else(|| Error::new("sevenZipMissing", "7z nicht gefunden (sudo pacman -S p7zip)"))?;
    let mut child = std::process::Command::new(&bin)
        .arg("x")
        .arg("-y")
        .arg("-bb0")
        .arg(format!("-o{}", dest.display()))
        .arg(p)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| ui_error::io(bin.display(), e))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| Error::new("sevenZipOutput", "7z-Ausgabe fehlgeschlagen"))?;
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line).map_err(|e| ui_error::io(bin.display(), e))?;
        if n == 0 {
            break;
        }
        if cancel.cancelled() {
            let _ = child.kill();
            return Err(Error::new("canceled", "Abgebrochen"));
        }
        if let Some(pct) = line.trim().strip_suffix('%') {
            if let Ok(p) = pct.trim().parse::<u64>() {
                emit(
                    progress,
                    "extract",
                    "extract",
                    p,
                    100,
                    "pct",
                    ui_error::par("pct", p),
                );
            }
        }
    }
    let status = child.wait().map_err(|e| ui_error::io(bin.display(), e))?;
    if !status.success() {
        return Err(Error::new("sevenZipFailed", "7z-Extraktion fehlgeschlagen."));
    }
    Ok(ArchiveSummary {
        entries: 0,
        bytes: 0,
    })
}

fn safe_join(dest: &Path, name: &str) -> Result<PathBuf, Error> {
    let rel = Path::new(name);
    if rel.is_absolute() || rel.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return Err(Error::new("archivePathInvalid", format!("Ungültiger Archivpfad: {name}")).with("path", name));
    }
    Ok(dest.join(rel))
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::CancelToken;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, AtomicU64};
    use std::sync::Arc;

    fn tmpdir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "compound_arch_{}_{}",
            tag,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn no_cancel() -> CancelToken {
        CancelToken(Arc::new(AtomicBool::new(false)))
    }

    fn progress_chan() -> Channel<ProgressEvent> {
        Channel::<ProgressEvent>::new(|_| Ok(()))
    }

    fn setup_tree(dir: &Path) -> PathBuf {
        let src = dir.join("projekt");
        std::fs::create_dir_all(src.join("sub")).unwrap();
        std::fs::write(src.join("index.html"), b"<h1>Hallo</h1>").unwrap();
        std::fs::write(src.join("sub/style.css"), b"body{}").unwrap();
        src
    }

    fn assert_tree_matches(src: &Path, out: &Path) {
        // Wurzeleintrag „projekt/“ bleibt im Archiv erhalten
        assert_eq!(std::fs::read(out.join("projekt/index.html")).unwrap(), b"<h1>Hallo</h1>");
        assert_eq!(std::fs::read(out.join("projekt/sub/style.css")).unwrap(), b"body{}");
        assert_eq!(std::fs::read(src.join("index.html")).unwrap(), b"<h1>Hallo</h1>");
    }

    #[test]
    fn zip_roundtrip_create_list_extract() {
        let dir = tmpdir("zip");
        let src = setup_tree(&dir);
        let zip_path = dir.join("out.zip");
        let srcs = vec![src.display().to_string()];
        let base = archive_base(&srcs);
        let done = AtomicU64::new(0);
        let cb = |_: String| Ok::<(), Error>(());
        let summary = create_zip(&srcs, &zip_path, &base, &done, &cb).unwrap();
        assert_eq!(summary.entries, 2); // nur Dateien (index.html + style.css)

        // Liste zeigt relativ Pfade
        let listed: Vec<String> = zip_list(&zip_path)
            .unwrap()
            .into_iter()
            .map(|e| e.path)
            .collect();
        assert!(listed.contains(&"projekt/index.html".to_string()), "{listed:?}");
        assert!(listed.contains(&"projekt/sub/style.css".to_string()), "{listed:?}");

        // Extraktion stellt Baum wieder her
        let out = dir.join("out");
        std::fs::create_dir_all(&out).unwrap();
        extract_zip(&zip_path, &out, &progress_chan(), &no_cancel()).unwrap();
        assert_tree_matches(&src, &out);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn tar_gz_roundtrip() {
        let dir = tmpdir("targz");
        let src = setup_tree(&dir);
        let archive = dir.join("out.tar.gz");
        let srcs = vec![src.display().to_string()];
        let base = archive_base(&srcs);
        let done = AtomicU64::new(0);
        let cb = |_: String| Ok::<(), Error>(());
        create_tar(&srcs, &archive, &base, Some("gz"), &done, &cb).unwrap();
        let out = dir.join("out");
        std::fs::create_dir_all(&out).unwrap();
        extract_tar(&archive, &out, &progress_chan(), &no_cancel()).unwrap();
        assert_tree_matches(&src, &out);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn untarred_plain_tar_roundtrip() {
        let dir = tmpdir("tar");
        let src = setup_tree(&dir);
        let archive = dir.join("out.tar");
        let srcs = vec![src.display().to_string()];
        let base = archive_base(&srcs);
        let done = AtomicU64::new(0);
        let cb = |_: String| Ok::<(), Error>(());
        create_tar(&srcs, &archive, &base, None, &done, &cb).unwrap();
        let out = dir.join("out");
        std::fs::create_dir_all(&out).unwrap();
        extract_tar(&archive, &out, &progress_chan(), &no_cancel()).unwrap();
        assert_tree_matches(&src, &out);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn safe_join_rejects_traversal_and_absolute() {
        let dest = Path::new("/tmp/compound_arch_safe");
        assert!(safe_join(dest, "normal.txt").is_ok());
        assert!(safe_join(dest, "../etc/passwd").is_err());
        assert!(safe_join(dest, "/etc/passwd").is_err());
        assert!(safe_join(dest, "a/../../b").is_err());
        assert!(safe_join(dest, "a/b/c.txt").is_ok());
    }

    #[test]
    fn arch_ext_detection() {
        assert_eq!(arch_ext(Path::new("a.zip")), "zip");
        assert_eq!(arch_ext(Path::new("a.tar.gz")), "tar.gz");
        assert_eq!(arch_ext(Path::new("a.tgz")), "tgz");
        assert_eq!(arch_ext(Path::new("a.tar.bz2")), "tar.bz2");
        assert_eq!(arch_ext(Path::new("a.tar.xz")), "tar.xz");
        assert_eq!(arch_ext(Path::new("a.7z")), "7z");
        assert_eq!(arch_ext(Path::new("a.tar")), "tar");
        assert_eq!(arch_ext(Path::new("keineendung")), "");
    }

    #[test]
    fn is_dir_archive_rejects_folder() {
        let dir = tmpdir("isdir");
        assert!(is_dir_archive(&dir).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn common_base_detects_shared_parent() {
        let d = tmpdir("base");
        let a = d.join("x/a.txt");
        let b = d.join("y/b.txt");
        std::fs::create_dir_all(a.parent().unwrap()).unwrap();
        std::fs::create_dir_all(b.parent().unwrap()).unwrap();
        std::fs::write(&a, b"a").unwrap();
        std::fs::write(&b, b"b").unwrap();
        let base = common_base(&[
            a.display().to_string(),
            b.display().to_string(),
        ]);
        assert_eq!(base, d);
        std::fs::remove_dir_all(&d).unwrap();
    }
}
