# Compound

A fast, self-built desktop file manager. **Tauri 2 + Svelte 5 (TypeScript)** — Linux first, then Windows.

## Credits

Built by:

| Who | Nick |
|-----|------|
| Sam | **S4m** |
| Димитрий | **дима** |
| Наталья | **натя** |
| Jake | **JK** |

## Features

- **Navigation**: one- or two-panel view, breadcrumb, history (back/forward), bookmarks, sorting (name/ext/size/date), list or grid
- **File ops**: copy, move (with conflict dialog + "keep both"), duplicate, rename (inline + dialog), new file/folder, to trash, permanent delete, restore from trash (`trash://` view), OS drag & drop import
- **Search**: differential index in SQLite (FTS5) over file names and text content, live search without index as fallback
- **Editor**: CodeMirror 6 with syntax highlighting (Rust, JS/TS, Python, JSON, Markdown, CSS, HTML/XML, SQL), Latin-1 detection, conflict guard on external change, abort on binary/huge files
- **Terminal**: built-in interactive shell (xterm.js + portable-pty) or external terminal (kitty etc.)
- **Tools**: duplicate finder (hash based), folder compare, mass rename (replace/regex/remove/insert/case/ext/numbering) with preview
- **Archives**: create and extract zip / tar / tar.gz / tar.bz2 / tar.xz, 7z via system `7z` (if present)
- **UI**: German and English interface, dark theme, keyboard shortcuts (F5 refresh, F2 rename, Ctrl+F search, Ctrl+N new file, Delete → trash, Ctrl+D → delete, Tab switch panel, Alt+←/→/↑ navigation, Backspace → parent dir, Alt+Ctrl+←/→ move selection to the other panel, Space mark, Enter move marked into focused panel)

## Requirements (Linux)

- Linux with WebKit2Gtk 4.1, GTK3, glibc >= 2.41 (current distros)
- Rust >= 1.77, Node >= 20, pnpm
- Archives: `7z` (extra), `tar`, `xz`, `bzip2` (system default)

CachyOS/Arch:

```bash
sudo pacman -S --needed base-devel webkit2gtk-4.1 gtk3 librsvg libappindicator-gtk3 7zip
```

## Development

```bash
pnpm install
pnpm dev          # Vite dev server + tauri dev
pnpm check        # svelte-check
pnpm build        # frontend-only Vite build
```

Backend tests:

```bash
cd src-tauri && cargo test
cargo check       # static check
```

## Build & packages

```bash
pnpm tauri build                    # deb + AppImage (Linux)
pnpm tauri build --bundles deb      # .deb only (Debian/Ubuntu)
pnpm tauri build --bundles appimage # .appimage only (universal)
```

Output:

- `src-tauri/target/release/bundle/deb/Compound_1.0.0_amd64.deb`
- `src-tauri/target/release/bundle/appimage/Compound_1.0.0_amd64.AppImage`

Run the AppImage without installing:

```bash
chmod +x Compound_1.0.0_amd64.AppImage
./Compound_1.0.0_amd64.AppImage
```

## Architecture

```
frontend (Svelte 5, Vite)
  src/App.svelte                  app shell: layout, global keymap, view switch
  src/lib/stores.svelte.ts        global rune state (panels, ops, clipboard, dialogs)
  src/lib/api.ts                  typed invoke wrappers around the Rust commands
  src/lib/i18n.ts                 de/en strings + t() lookup
  src/lib/components/Panel.svelte one file panel: list, selection, context menu
  src/lib/components/…            dialogs, editor (CodeMirror), terminal (xterm.js)

backend (Rust, Tauri 2)
  src-tauri/src/lib.rs            registers all commands
  fs_cmd.rs, ops_cmd.rs           navigation, file ops, trash (trash://)
  search_cmd.rs                   SQLite / FTS5 index
  archive_cmd.rs                  zip/tar/gzip/bzip2/xz, 7z via CLI
  editor_cmd.rs, terminal_cmd.rs, tools_cmd.rs, misc_cmd.rs
```

### File map — what each file is for

Comments inside the sources are short bilingual (RU + EN) reminders of intent, not documentation.
`sprache/language`: RU first, EN second; abbreviations are deliberate (`спс` = спасибо, `TH` = thank you,
`пжл` = пожалуйста, `pls` = please, `дир` = директория, `ист` = источник).

| File | Purpose / коротко |
|------|------------------|
| `src/main.ts` | mounts App into #app / монтирует App |
| `src/App.svelte` | shell + global keys / оболочка + глобальные хоткеи |
| `src/lib/api.ts` | invoke wrappers / обёртки над Rust-командами |
| `src/lib/stores.svelte.ts` | global state / глобальное состояние |
| `src/lib/types.ts` | TS mirror of Rust types / TS-зеркало Rust-типов |
| `src/lib/i18n.ts` | de/en strings / строки de/en |
| `src/lib/format.ts` | bytes, dates, perms, plurals / байты, даты, права |
| `src/lib/sort.ts` | shared entry sort / общая сортировка |
| `src/lib/icons.ts` | lucide-style icon paths / пути иконок |
| `src/lib/global.css` | dark theme + layout / тёмная тема + layout |
| `src/lib/components/Panel.svelte` | the file panel / файловая панель |
| `src/lib/components/*Dialog.svelte` | dialogs / диалоги |
| `src-tauri/src/lib.rs` | command registry / реестр команд |
| `src-tauri/src/state.rs` | app state, ops, cancel tokens / состояние, операции |
| `src-tauri/src/fs_cmd.rs` | list_dir, places, disk usage / листинг, диски |
| `src-tauri/src/fsutil.rs` | fs helpers / хелперы ФС |
| `src-tauri/src/ops_cmd.rs` | copy/move/delete/rename/trash / оп-ции над файлами |
| `src-tauri/src/search_cmd.rs` | FTS5 index + search / индекс FTS5 |
| `src-tauri/src/archive_cmd.rs` | zip/tar/7z / архивы |
| `src-tauri/src/tools_cmd.rs` | dupes, compare, mass rename / дубли, сравнение, переименование |
| `src-tauri/src/editor_cmd.rs` | read/save text with mtime guard / чтение и запись |
| `src-tauri/src/terminal_cmd.rs` | pty sessions / pty-сессии |
| `src-tauri/src/ui_error.rs` | localized {code, params} errors / локализованные ошибки |
| `src-tauri/src/types.rs` | serde types / serde-типы |

> Note: `package.json`, `tauri.conf.json`, `capabilities/*.json` stay comment-free on purpose —
> they are strict JSON and a comment would break the parser. `pnpm-lock.yaml`, `Cargo.lock`
> and `src-tauri/gen/` are generated, never edited by hand.

## Conventions

- Svelte runes must not be reassigned across modules → mutable values live in `$state` objects (`.value`) plus setter functions (e.g. `view.value`, `setView(…)`). The `.svelte.ts` suffix is mandatory for rune stores.
- Progress flows over Tauri `Channel`s; the frontend buffers events and assigns them to the op store once the `opId` is known (`setProgressHandler` creates the op at `OpStarted`; labels come from `renderOpLabel` with `op.*` i18n keys + params).
- Conflicts arrive as a `ConflictRequest` over a channel and are answered by the conflict dialog (`resolve_conflict`).
- Command errors are **localized**: the backend only sends `{code, params}` (`src-tauri/src/ui_error.rs`); the UI translates via `errors.*` keys (`errMsg` in `stores.svelte.ts`, English fallback for unknown codes). German text exists only in the backend log (`Display` impl).

## License

MIT
