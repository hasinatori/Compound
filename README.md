# Compound

Ein eigener, flotter Dateimanager für den Desktop. **Tauri 2 + Svelte 5 (TypeScript)**, Linux zuerst, dann Windows.

## Funktionen

- **Navigation**: Ein- oder Zwei-Panel-Ansicht, Breadcrumb, Verlauf (zurück/vor), Lesezeichen, Sortieren (Name/Endung/Größe/Datum), Liste oder Kacheln
- **Datei-Operationen**: Kopieren, Verschieben (mit Konflikt-Dialog und „Beide behalten"), Duplizieren, Umbenennen (inline und per Dialog), Neue Datei/Ordner, in den Papierkorb, endgültiges Löschen, Wiederherstellen aus dem Papierkorb (`trash://`-Ansicht), OS-Drag&Drop-Import
- **Suche**: Differential-Index in SQLite (FTS5) mit Dateinamen- und Textinhalt-Suche, Live-Suche ohne Index als Fallback
- **Editor**: CodeMirror 6 mit Syntax-Highlighting (Rust, JavaScript/TypeScript, Python, JSON, Markdown, CSS, HTML/XML, SQL), Latin-1-Erkennung, Konflikt-Schutz bei externer Änderung, Abbruch bei Binär-/Riesen-Dateien
- **Terminal**: integrierte interaktive Shell (xterm.js + portable-pty) oder externes Terminal (Kitty etc.)
- **Werkzeuge**: Duplikat-Finder (Hash-basiert), Ordner-Vergleich, Massen-Umbenennen (Ersetzen/RegEx/Entfernen/Einfügen/Case/Endung/Nummerierung) mit Vorschau
- **Archive**: Erstellen und Entpacken von zip / tar / tar.gz / tar.bz2 / tar.xz, 7z über systemweites `7z` (falls vorhanden)
- **UI**: Deutsches und englisches Interface, dunkles Theme, Tastaturkürzel (F5 Refresh, F2 Umbenennen, Ctrl+F Suche, Ctrl+N Neue Datei, Delete → Papierkorb, Ctrl+D → Löschen, Tab Panel-Wechsel, Alt+←/→/↑ Navigation)

## Voraussetzungen (Linux)

- Linux mit WebKit2Gtk 4.1, GTK3, glibc ≥ 2.41 (aktuelle Distributionen)
- Rust ≥ 1.77, Node ≥ 20, pnpm
- Archive: `7z` (zusätzlich), `tar`, `xz`, `bzip2` (Voreinstellung des Systems)

CachyOS/Arch:

```bash
sudo pacman -S --needed base-devel webkit2gtk-4.1 gtk3 librsvg libappindicator-gtk3 7zip
```

## Entwicklung

```bash
pnpm install
pnpm dev          # Vite-Devserver + tauri dev (pnpm tauri dev)
pnpm check        # svelte-check
pnpm build        # Vite-Build des Frontends
```

Backend-Tests:

```bash
cd src-tauri && cargo test
cargo check       # statische Prüfung
```

## Build & Pakete

```bash
pnpm tauri build                    # deb + AppImage (Linux)
pnpm tauri build --bundles deb      # nur .deb (Debian/Ubuntu)
pnpm tauri build --bundles appimage # nur .appimage (universell)
```

Ausgabe:

- `src-tauri/target/release/bundle/deb/Compound_1.0.0_amd64.deb`
- `src-tauri/target/release/bundle/appimage/Compound_1.0.0_amd64.AppImage`

AppImage installieren (ohne Installation):

```bash
chmod +x Compound_1.0.0_amd64.AppImage
./Compound_1.0.0_amd64.AppImage
```

## Architektur

```
frontend (Svelte 5, Vite)
  src/lib/stores.svelte.ts   globale Runes-Zustände (Panels, Ops, Clipboard, Dialoge)
  src/lib/api.ts             typisierte invoke-Wrapper um die Rust-Commands
  src/lib/components/…       Panels, Dialoge, Editor (CodeMirror), Terminal (xterm.js)

backend (Rust, Tauri 2)
  src-tauri/src/lib.rs       33 Commands registriert
  fs_cmd.rs, ops_cmd.rs      Navigation, Datei-Operationen, Papierkorb (trash://)
  search_cmd.rs              SQLite/FTS5-Index
  archive_cmd.rs             zip/tar-Gzip/Bzip2/Xz, 7z via CLI
  editor_cmd.rs, terminal_cmd.rs, tools_cmd.rs, misc_cmd.rs
```

Wichtige Konventionen:

- Svelte-Runen dürfen aus Modulen nicht reassigned exportiert werden → veränderliche Werte liegen in `$state`-Objekten (`.value`) plus Setter-Funktionen (z. B. `view.value`, `setView(…)`). `.svelte.ts`-Dateiendung für Rune-Stores ist Pflicht.
- Der Fortschritt läuft über Tauri-`Channel`s; das Frontend puffert Events und setzt sie nach Bekanntwerden der `opId` in den Op-Store (`setProgressHandler` legt die Op bei `OpStarted` an — kein `label` mehr, sondern `kind`; Labels rendert `renderOpLabel` aus `op.*`-i18n-Keys + Params).
- Konflikte kommen als `ConflictRequest` über einen Channel und werden vom `PendingConflict`-Dialog beantwortet (`resolve_conflict`).
- Fehler über Commands sind **lokalisiert**: Das Backend sendet nur `{code, params}` (`src-tauri/src/ui_error.rs`); die UI übersetzt über `errors.*`-i18n-Keys (`errMsg` in stores.svelte.ts, englischer Fallback bei unbekannten Codes). Unbekannte/`{code, params}`-fremde Fehler zeigen den `errors.unknown`-Text. Deutsche Texte existieren nur im Backend-Log (`Display`-Implementierung). Hinweis: Der Toast-Stil ist unverändert geblieben (Umsetzung nur der Textebene).

## Lizenz

MIT