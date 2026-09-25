// TS-зеркало Rust-типов (src-tauri/src/types.rs), serde camelCase.
// TS mirror of the Rust types, serde camelCase on the wire.
// Меняем Rust-тип -> меняем тут же, иначе рантайм-тихо разъедется.
// Change a Rust type -> change it here too, else runtime drifts silently.

// Локализованная ошибка от Rust: code -> i18n-ключ errors.<code>.
// Localized error from Rust: code -> i18n key errors.<code>.
export interface AppErr {
	code: string;
	params: Record<string, string>;
}

export interface FileEntry {
	path: string;
	name: string;
	ext: string;
	isDir: boolean;
	isSymlink: boolean;
	isHidden: boolean;
	size: number; // u64 -> number (JS safe int bis 2^53)
	mtimeMs: number;
	perms: string;
	kind: string;
	linkTarget: string | null;
}

export interface Place {
	name: string;
	path: string;
	kind: string;
}

export interface PathPart {
	name: string;
	path: string;
}

export interface DiskUsage {
	total: number;
	free: number;
}

export interface ProgressEvent {
	op: string;
	phase: string;
	current: number;
	total: number;
	kind: string;
	params: Record<string, string>;
}

export interface OpStarted {
	opId: string;
	kind: string;
	total: number;
}

export interface OpFinished {
	opId: string;
	ok: boolean;
	error: AppErr | null;
}

export interface OpInfo {
	opId: string;
	kind: string;
	createdAtMs: number;
}

export interface ConflictRequest {
	opId: string;
	key: string;
	src: string;
	dest: string;
	isDir: boolean;
	index: number;
	total: number;
}

export interface ConflictChoice {
	action: 'overwrite' | 'skip' | 'keep_both' | 'abort';
	applyToAll: boolean;
}

export interface SearchOpts {
	includeHidden: boolean;
	nameOnly: boolean;
	maxSize: number | null;
	kinds: string[];
}

export interface SearchResult {
	path: string;
	name: string;
	isDir: boolean;
	size: number;
	mtimeMs: number;
	kind: string;
	lineNo: number | null;
	snippet: string | null;
}

export interface SearchReturn {
	results: SearchResult[];
	usedIndex: boolean;
	cancelled: boolean;
}

export interface IndexSummary {
	files: number;
	dirs: number;
	ms: number;
}

export interface IndexedRoot {
	root: string;
	indexedAtMs: number;
	fileCount: number;
}

export interface DuplicateGroup {
	size: number;
	hash: string;
	files: string[];
}

export interface CompareItem {
	relative: string;
	status: 'only_in_a' | 'only_in_b' | 'different' | 'identical';
	isDir: boolean;
	sizeA: number;
	sizeB: number;
	mtimeA: number;
	mtimeB: number;
	msg: string;
}

export type RuleType = 'find_replace' | 'regex' | 'remove' | 'insert' | 'case' | 'ext' | 'numbering';
export type CaseMode = 'upper' | 'lower' | 'title';

export interface RenameRule {
	ruleType: RuleType;
	find?: string | null;
	replace?: string | null;
	insertAt?: number | null;
	insertText?: string | null;
	caseMode?: CaseMode | null;
	newExt?: string | null;
	template?: string | null;
	start?: number | null;
	step?: number | null;
	digits?: number | null;
}

export interface RenamePreviewItem {
	from: string;
	to: string;
	conflict: boolean;
	error: AppErr | null;
}

export interface ArchiveEntry {
	path: string;
	size: number;
	isDir: boolean;
}

export interface ArchiveSummary {
	entries: number;
	bytes: number;
}

export interface EditorRead {
	text: string;
	encoding: string;
	readonly: boolean;
	binary: boolean;
	truncated: boolean;
	converted: boolean;
	mtimeMs: number;
}

export interface SaveResult {
	saved: boolean;
	changed: boolean;
	error: string | null;
	mtimeMs: number;
}

export interface TrashItem {
	nameInTrash: string;
	originalPath: string;
	trashedAtMs: number;
	isDir: boolean;
}

export interface TerminalInfo {
	id: string;
	cwd: string;
}

export interface AboutInfo {
	name: string;
	version: string;
	identifier: string;
}

// ---- App-interne Typen ----

export type SortKey = 'name' | 'ext' | 'size' | 'mtime';
export type ViewMode = 'list' | 'grid';

export interface PanelState {
	cwd: string;
	entries: FileEntry[];
	selected: string[];
	sortBy: SortKey;
	sortDesc: boolean;
	viewMode: ViewMode;
	loading: boolean;
	error: string | null;
	scrollTop: number;
	history: string[];
	historyIndex: number;
	activeIndex: number; // fokussierte Zeile bei Tastatur-Navigation
}

export type ActiveView = 'browser' | 'editor' | 'terminal' | 'search' | 'tools' | 'settings';

export interface OpState {
	opId: string;
	label: string;
	total: number;
	current: number;
	phase: string;
	done: boolean;
	error: string | null;
}

export interface Settings {
	language: 'de' | 'en';
	singlePanel: boolean;
	sortBy: SortKey;
	sortDesc: boolean;
	confirmDelete: boolean;
	showHidden: boolean;
}

export type NoticeKind = 'info' | 'ok' | 'warn' | 'error';

export interface Notice {
	id: number;
	kind: NoticeKind;
	text: string;
}