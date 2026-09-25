// Обёртки над Rust-командами (src-tauri/src/lib.rs). Только invoke, без ран.
// Typed wrappers around the Rust commands. Pure invoke, no runes here.
// Каналы: прогресс/конфликты летят через Tauri Channel — буферизуем до opId.
// Channels: progress/conflicts arrive via Tauri Channel — buffer until opId.
import { invoke, Channel } from '@tauri-apps/api/core';
import type {
	AboutInfo,
	ArchiveEntry,
	ArchiveSummary,
	CompareItem,
	ConflictChoice,
	ConflictRequest,
	DiskUsage,
	DuplicateGroup,
	EditorRead,
	FileEntry,
	IndexSummary,
	IndexedRoot,
	OpStarted,
	PathPart,
	Place,
	ProgressEvent,
	RenamePreviewItem,
	RenameRule,
	SaveResult,
	SearchOpts,
	SearchReturn,
	TerminalInfo,
	TrashItem,
} from './types';

// Разрешение конфликтов: приложение ставит хендлер, диалог
	// Conflict resolution: the app registers a handler; the conflict dialog
// показывает запрос и отвечает через resolveConflict().
	// shows the request and answers it through resolveConflict().
let conflictHandler: ((req: ConflictRequest) => void) | null = null;

export function setConflictHandler(h: ((req: ConflictRequest) => void) | null) {
	conflictHandler = h;
}

// Прогресс копим (события приходят до ответа invoke) и
	// Progress is buffered (events arrive before the invoke reply) and
// после появления opId отдаём пачкой в стор.
	// flushed into the store in one go once the opId is known.
// `started` всегда прокидываем (даже без событий), чтобы стор
	// `started` is always passed through (even with no events) so the store
// мог завести операцию.
	// can create the operation.
let progressHandler: ((opId: string, started: OpStarted, events: ProgressEvent[]) => void) | null = null;

export function setProgressHandler(
	h: ((opId: string, started: OpStarted, events: ProgressEvent[]) => void) | null,
) {
	progressHandler = h;
}

function makeProgressChannel(onEvt: (e: ProgressEvent) => void): Channel<ProgressEvent> {
	const ch = new Channel<ProgressEvent>();
	ch.onmessage = (e) => onEvt(e);
	return ch;
}

function makeConflictChannel(): Channel<ConflictRequest> {
	const ch = new Channel<ConflictRequest>();
	ch.onmessage = (req) => {
		if (conflictHandler) conflictHandler(req);
	};
	return ch;
}

// Общая обвязка запуска операции + буфер событий прогресса.
// Wraps op start + buffers progress events.
async function runOp(
	cmd: string,
	args: Record<string, unknown>,
): Promise<OpStarted> {
	const buf: ProgressEvent[] = [];
	const started = await invoke<OpStarted>(cmd, {
		...args,
		progress: makeProgressChannel((e) => buf.push(e)),
		conflicts: makeConflictChannel(),
	});
	if (progressHandler) {
		progressHandler(started.opId, started, buf);
	}
	return started;
}

/** Fortschritt direkt an den globalen Op-Store (stores.svelte.ts) weiterreichen. */
export type OpProgressSink = (opId: string) => (e: ProgressEvent) => void;

// ---- Konflikt ----

export async function resolveConflict(opId: string, key: string, choice: ConflictChoice) {
	await invoke('resolve_conflict', { opId, key, ...choice });
}

// ---- Navigation / Basis ----

export async function listDir(path: string): Promise<FileEntry[]> {
	return invoke('list_dir', { path });
}

export async function getPlaces(): Promise<Place[]> {
	return invoke('get_places');
}

export async function pathParts(path: string): Promise<PathPart[]> {
	return invoke('path_parts', { path });
}

export async function diskUsage(path: string): Promise<DiskUsage> {
	return invoke('disk_usage', { path });
}

export async function thumb(path: string, size: number): Promise<string> {
	return invoke('thumb', { path, size });
}

export async function openDefault(path: string): Promise<void> {
	return invoke('open_default', { path });
}

// ---- Datei-Operationen ----

export function copyItems(srcs: string[], destDir: string, policy: string): Promise<OpStarted> {
	return runOp('copy_items', { srcs, destDir, policy });
}

export function moveItems(srcs: string[], destDir: string, policy: string): Promise<OpStarted> {
	return runOp('move_items', { srcs, destDir, policy });
}

export function deletePermanent(srcs: string[]): Promise<OpStarted> {
	return runOp('delete_permanent', { srcs });
}

export async function trashItems(paths: string[]): Promise<void> {
	return invoke('trash_items', { paths });
}

export async function restoreTrash(nameInTrash: string): Promise<string> {
	return invoke('restore_trash', { nameInTrash });
}

export async function emptyTrash(): Promise<void> {
	return invoke('empty_trash');
}

export async function listTrash(): Promise<TrashItem[]> {
	return invoke('list_trash');
}

export async function duplicateItem(path: string): Promise<string> {
	return invoke('duplicate_item', { path });
}

export async function renameItem(path: string, newName: string): Promise<string> {
	return invoke('rename_item', { path, newName });
}

export async function createItem(destDir: string, name: string, isDir: boolean): Promise<FileEntry> {
	return invoke('create_item', { destDir, name, isDir });
}

export async function openExternalTerminal(path: string): Promise<void> {
	return invoke('open_external_terminal', { path });
}

// ---- Suche ----

export async function indexFolder(
	root: string,
	force: boolean,
	onEvt: (e: ProgressEvent) => void,
): Promise<IndexSummary> {
	return invoke('index_folder', { root, force, progress: makeProgressChannel(onEvt) });
}

export async function indexedRoots(): Promise<IndexedRoot[]> {
	return invoke('indexed_roots');
}

export async function clearIndex(root: string): Promise<void> {
	return invoke('clear_index', { root });
}

export async function searchIndex(
	query: string,
	root: string,
	opts: SearchOpts,
	onEvt: (e: ProgressEvent) => void,
): Promise<SearchReturn> {
	return invoke('search_index', { query, root, opts, progress: makeProgressChannel(onEvt) });
}

export async function searchNow(
	query: string,
	root: string,
	content: boolean,
	opts: SearchOpts,
	onEvt: (e: ProgressEvent) => void,
): Promise<SearchReturn> {
	return invoke('search_now', { query, root, content, opts, progress: makeProgressChannel(onEvt) });
}

// ---- Werkzeuge ----

export async function findDuplicates(
	root: string,
	onEvt: (e: ProgressEvent) => void,
): Promise<DuplicateGroup[]> {
	return invoke('find_duplicates', { root, progress: makeProgressChannel(onEvt) });
}

export async function compareFolders(
	a: string,
	b: string,
	deep: boolean,
	onEvt: (e: ProgressEvent) => void,
): Promise<CompareItem[]> {
	return invoke('compare_folders', { a, b, deep, progress: makeProgressChannel(onEvt) });
}

export async function multiRename(
	items: string[],
	rules: RenameRule[],
	dryRun: boolean,
): Promise<RenamePreviewItem[]> {
	return invoke('multi_rename', { items, rules, dryRun });
}

// ---- Archive ----

export async function archiveCreate(
	srcs: string[],
	dest: string,
	onEvt: (e: ProgressEvent) => void,
): Promise<ArchiveSummary> {
	return invoke('archive_create', { srcs, dest, progress: makeProgressChannel(onEvt) });
}

export async function archiveExtract(
	archive: string,
	destDir: string,
	onEvt: (e: ProgressEvent) => void,
): Promise<ArchiveSummary> {
	return invoke('archive_extract', { archive, destDir, progress: makeProgressChannel(onEvt) });
}

export async function archiveList(archive: string): Promise<ArchiveEntry[]> {
	return invoke('archive_list', { archive });
}

// ---- Editor ----

export async function readText(path: string, maxBytes?: number): Promise<EditorRead> {
	return invoke('read_text', { path, maxBytes });
}

export async function saveText(path: string, content: string, expectedMtime: number): Promise<SaveResult> {
	return invoke('save_text', { path, content, expectedMtime });
}

// ---- Terminal ----

export async function terminalOpen(
	cwd: string,
	onData: (s: string) => void,
): Promise<TerminalInfo> {
	const ch = new Channel<string>();
	ch.onmessage = onData;
	return invoke('terminal_open', { cwd, out: ch });
}

export async function terminalWrite(id: string, data: string): Promise<void> {
	return invoke('terminal_write', { id, data });
}

export async function terminalResize(id: string, cols: number, rows: number): Promise<void> {
	return invoke('terminal_resize', { id, cols, rows });
}

export async function terminalClose(id: string): Promise<void> {
	return invoke('terminal_close', { id });
}

// ---- Sonstiges ----

export async function getAbout(): Promise<AboutInfo> {
	return invoke('about');
}