use crate::types::FileEntry;
use crate::ui_error::{self, Error};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub fn is_hidden_name(name: &str) -> bool {
    name.starts_with('.')
}

pub fn file_ext(name: &str) -> String {
    match name.rfind('.') {
        Some(i) if i > 0 && i < name.len() - 1 => name[i + 1..].to_lowercase(),
        _ => String::new(),
    }
}

pub fn mtime_ms(meta: &fs::Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Kategorie eines Eintrags, bestimmt Icon + Suchfilter.
pub fn kind_for_ext(ext: &str) -> String {
    match ext {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "svg" | "tif" | "tiff" | "heic"
        | "heif" | "ico" | "avif" | "jxl" => "image".into(),
        "mp3" | "wav" | "flac" | "ogg" | "opus" | "m4a" | "aac" | "mid" | "midi" | "wma"
        | "aiff" | "ape" => "audio".into(),
        "mp4" | "mkv" | "webm" | "avi" | "mov" | "m4v" | "wmv" | "flv" | "3gp" | "ts" | "m2ts"
        => "video".into(),
        "zip" | "tar" | "gz" | "tgz" | "bz2" | "tbz2" | "xz" | "txz" | "7z" | "rar" | "zst"
        | "iso" | "cab" | "deb" | "rpm" | "apk" | "cpio" | "lz4" | "z" => "archive".into(),
        "txt" | "md" | "markdown" | "log" | "ini" | "conf" | "cfg" | "env" | "yaml" | "yml"
        | "json" | "xml" | "csv" | "tsv" | "rst" | "toml" | "nfo" | "srt" | "vtt" | "gitignore"
        | "license" => "text".into(),
        "rs" | "py" | "js" | "tsx" | "jsx" | "c" | "h" | "cpp" | "hpp" | "cs" | "go"
        | "java" | "rb" | "php" | "sh" | "bash" | "zsh" | "html" | "css" | "scss" | "sql"
        | "kt" | "kts" | "swift" | "lua" | "pl" | "svelte" | "vue" | "dart" | "ex" | "exs"
        | "erl" | "hs" | "clj" | "zig" | "m" | "mm" | "asm" | "s" | "pro" | "ino" => "code".into(),
        "pdf" => "pdf".into(),
        "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt" | "ods" | "odp" | "rtf" => "office".into(),
        "ttf" | "otf" | "woff" | "woff2" => "font".into(),
        "exe" | "msi" | "bin" | "appimage" | "so" | "dll" | "dylib" | "o" => "executable".into(),
        _ => "other".into(),
    }
}

pub fn kind_for(is_dir: bool, name: &str) -> String {
    if is_dir {
        return "dir".into();
    }
    kind_for_ext(&file_ext(name))
}

#[cfg(unix)]
pub fn perm_string(meta: &fs::Metadata) -> String {
    use std::os::unix::fs::PermissionsExt;
    let m = meta.permissions().mode();
    let mut s = String::with_capacity(9);
    let bits = [
        (0o400, 'r'),
        (0o200, 'w'),
        (0o100, 'x'),
        (0o040, 'r'),
        (0o020, 'w'),
        (0o010, 'x'),
        (0o004, 'r'),
        (0o002, 'w'),
        (0o001, 'x'),
    ];
    for (bit, c) in bits {
        s.push(if m & bit != 0 { c } else { '-' });
    }
    s
}

#[cfg(not(unix))]
pub fn perm_string(_: &fs::Metadata) -> String {
    "--------".into()
}

pub fn read_dir_entries(path: &Path) -> Result<Vec<FileEntry>, Error> {
    let mut out = Vec::new();
    let rd = fs::read_dir(path).map_err(|e| ui_error::io(path.display(), e))?;
    for entry in rd.flatten() {
        let p = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let ft = match entry.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };
        let is_symlink = ft.is_symlink();
        // Entry::metadata() folgt Symlinks – genau das wollen wir hier.
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue, // defekter Symlink / verlorener Eintrag -> überspringen
        };
        let is_dir = meta.is_dir();
        let kind = if is_symlink && is_dir {
            "dir".into()
        } else {
            kind_for(is_dir, &name)
        };
        let link_target = if is_symlink {
            fs::read_link(&p)
                .ok()
                .map(|t| t.display().to_string())
        } else {
            None
        };
        let ext = file_ext(&name);
        out.push(FileEntry {
            path: p.display().to_string(),
            name,
            ext,
            is_dir, // Entry::metadata() folgt Symlinks -> Symlink auf Ordner zählt hier bereits als Ordner
            is_symlink,
            is_hidden: is_hidden_name(&entry.file_name().to_string_lossy()),
            size: if is_dir { 0 } else { meta.len() },
            mtime_ms: mtime_ms(&meta),
            perms: perm_string(&fs::symlink_metadata(&p).unwrap_or_else(|_| meta.clone())),
            kind,
            link_target,
        });
    }
    Ok(out)
}

/// Eindeutigen Zielpfad finden: "Name.ext", "Name (2).ext", …
pub fn unique_dest(dir: &Path, name: &str) -> PathBuf {
    let candidate = dir.join(name);
    if !candidate.exists() {
        return candidate;
    }
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name, ""),
    };
    for n in 2u32.. {
        let cand = format!("{stem} ({n}){ext}");
        if !dir.join(&cand).exists() {
            return dir.join(cand);
        }
    }
    unreachable!()
}

/// Grobe Binär-Erkennung (NUL-Bytes / Steuerzeichen).
pub fn is_binary(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    let sample = &bytes[..bytes.len().min(8192)];
    let nulls = sample.iter().filter(|&&b| b == 0x00).count();
    let ctrl = sample
        .iter()
        .filter(|&&b| b < 9 || (b > 13 && b < 32))
        .count();
    nulls > 0 || ctrl > sample.len() / 10
}

/// Pfad ohne Trailing-Slash (außer Root).
pub fn norm_path(p: &str) -> String {
    if p == "/" || p.is_empty() {
        return "/".into();
    }
    let p = p.trim_end_matches('/');
    if p.is_empty() {
        "/".into()
    } else {
        p.into()
    }
}

/// Sicherer Umgang mit Dateinamen-Validierung.
pub fn valid_name(name: &str) -> Result<(), Error> {
    if name.is_empty() {
        return Err(Error::new("nameEmpty", "Name darf nicht leer sein"));
    }
    if name == "." || name == ".." {
        return Err(Error::new("invalidName", "Ungültiger Name"));
    }
    if name.contains('/') {
        return Err(Error::new("nameSlash", "Der Name darf kein '/' enthalten"));
    }
    #[cfg(windows)]
    {
        if name.contains('\\') || name.contains(':') {
            return Err(Error::new("nameWindows", "Ungültiger Name für Windows"));
        }
    }
    Ok(())
}

/// Prozent-Decodierung (für .trashinfo-Pfade).
pub fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                out.push((h << 4) | l);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Escape LIKE-Wildcards.
pub fn esc_like(s: &str) -> String {
    s.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

pub fn is_text_like_kind(kind: &str) -> bool {
    matches!(kind, "text" | "code")
}

/// Liest bis zu `cap` Bytes (begrenzt, nie mehr als cap+1 anfassen).
pub fn read_capped(path: &std::path::Path, cap: u64) -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    let f = fs::File::open(path)?;
    let mut buf = Vec::with_capacity(cap.min(16 * 1024 * 1024) as usize);
    f.take(cap + 1).read_to_end(&mut buf)?;
    if buf.len() as u64 > cap {
        buf.truncate(cap as usize);
    }
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn file_ext_basic() {
        assert_eq!(file_ext("photo.JPG"), "jpg");
        assert_eq!(file_ext("archive.tar.gz"), "gz");
        assert_eq!(file_ext("noext"), "");
        assert_eq!(file_ext(".hidden"), "");
        assert_eq!(file_ext("trailing."), "");
        assert_eq!(file_ext("a.b.c"), "c");
    }

    #[test]
    fn hidden_detection() {
        assert!(is_hidden_name(".config"));
        assert!(!is_hidden_name("config"));
    }

    #[test]
    fn kinds() {
        assert_eq!(kind_for_ext("png"), "image");
        assert_eq!(kind_for_ext("mp3"), "audio");
        assert_eq!(kind_for_ext("mkv"), "video");
        assert_eq!(kind_for_ext("zip"), "archive");
        assert_eq!(kind_for_ext("rs"), "code");
        assert_eq!(kind_for_ext("pdf"), "pdf");
        assert_eq!(kind_for_ext("zzz"), "other");
        assert_eq!(kind_for(true, "ordner"), "dir");
    }

    #[test]
    fn unique_dest_skips_existing() {
        let dir = std::env::temp_dir().join(format!(
            "compound_test_unique_{}",
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.txt"), b"x").unwrap();
        assert_eq!(unique_dest(&dir, "b.txt"), dir.join("b.txt"));
        assert_eq!(unique_dest(&dir, "a.txt"), dir.join("a (2).txt"));
        std::fs::write(dir.join("a (2).txt"), b"y").unwrap();
        assert_eq!(unique_dest(&dir, "a.txt"), dir.join("a (3).txt"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn binary_detection() {
        assert!(is_binary(&[0u8; 100]));
        assert!(is_binary(&b"hello\x00world"[..]));
        assert!(!is_binary(b"plain text without control bytes"));
        assert!(!is_binary(b""));
        assert!(!is_binary(&[0x0a; 200]));
    }

    #[test]
    fn norm_paths() {
        assert_eq!(norm_path("/home/"), "/home");
        assert_eq!(norm_path("/"), "/");
        assert_eq!(norm_path(""), "/");
        assert_eq!(norm_path("/a/b///"), "/a/b");
    }

    #[test]
    fn names_valid() {
        assert!(valid_name("datei.txt").is_ok());
        assert!(valid_name("a/b").is_err());
        assert!(valid_name("").is_err());
        assert!(valid_name("..").is_err());
        assert!(valid_name(".").is_err());
    }

    #[test]
    fn percent_decode_roundtrip() {
        assert_eq!(percent_decode("Hallo%20Welt"), "Hallo Welt");
        assert_eq!(percent_decode("100%25"), "100%");
        assert_eq!(percent_decode("kein%zz"), "kein%zz");
        assert_eq!(percent_decode("caf%C3%A9"), "caf\u{e9}");
    }

    #[test]
    fn like_escaping() {
        assert_eq!(esc_like("50%_\\x"), "50\\%\\_\\\\x");
    }

    #[test]
    fn read_capped_limits() {
        let dir = std::env::temp_dir();
        let p = dir.join("compound_cap_test.txt");
        std::fs::write(&p, b"1234567890").unwrap();
        assert_eq!(read_capped(&p, 4).unwrap().len(), 4);
        assert_eq!(read_capped(&p, 100).unwrap().len(), 10);
        std::fs::remove_file(&p).unwrap();
    }

    #[test]
    fn dir_entries_lists_files() {
        let dir = std::env::temp_dir().join(format!(
            "compound_test_entries_{}",
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("datei.tar.gz"), b"x").unwrap();
        let entries = read_dir_entries(&dir).unwrap();
        let by_name = |n: &str| entries.iter().find(|e| e.name == n).unwrap();
        assert!(by_name("sub").is_dir);
        assert_eq!(by_name("datei.tar.gz").ext, "gz");
        assert_eq!(by_name("datei.tar.gz").kind, "archive");
        assert_eq!(by_name("datei.tar.gz").size, 1);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}