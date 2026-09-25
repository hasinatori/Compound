// Globale Laufzeit-Zustände (Svelte-5-Runen; Dateiendung .svelte.ts erforderlich).
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import type {
	AppErr,
	ConflictRequest,
	FileEntry,
	Notice,
	NoticeKind,
	OpState,
	PanelState,
	Settings,
} from './types';
import { defaultSettings, t } from './i18n';

const STORE_KEY = 'compound.settings.v1';

function loadSettings(): Settings {
	try {
		const raw = localStorage.getItem(STORE_KEY);
		if (!raw) return defaultSettings();
		return { ...defaultSettings(), ...(JSON.parse(raw) as Partial<Settings>) };
	} catch {
		return defaultSettings();
	}
}

export function saveSettings(s: Settings) {
	try {
		localStorage.setItem(STORE_KEY, JSON.stringify(s));
	} catch {
		// localStorage nicht verfügbar -> ignorieren
	}
}

export const settings = $state<Settings>(loadSettings());

/// Aktive UI-Sprache (für i18n t()).
export function lang() {
	return settings.language;
}

// ---- Fehler- und Op-Label-Lokalisierung ----

/**
 * Wandelt einen Backend-Fehler (`{code, params}`) in den lokalisierten
 * UI-Text um. Unbekannte Codes -> englischer Fallback, reine Strings
 * passieren unverändert (Frontend-seitige Fehler).
 */
export function errMsg(e: unknown): string {
	if (e && typeof e === 'object' && typeof (e as AppErr).code === 'string') {
		const err = e as AppErr;
		const key = `errors.${err.code}`;
		const s = t(key, lang(), err.params ?? {});
		if (s !== key) return s;
		return t('errors.unknown', 'en', {});
	}
	if (typeof e === 'string' && e.trim() !== '') return e;
	return t('errors.unknown', lang(), {});
}

/**
 * Op/Event-Kinds des Backends -> lokalisierte Anzeige-Labels.
 * Unbekannte Kinds liefern den i18n-Key selbst (transparent, kein Crash).
 */
export function renderOpLabel(kind: string, params: Record<string, string | number> = {}): string {
	return t(`op.${kind}`, lang(), params);
}

export function draftPanel(cwd: string): PanelState {
	return {
		cwd,
		entries: [],
		selected: [],
		sortBy: settings.sortBy,
		sortDesc: settings.sortDesc,
		viewMode: 'list',
		loading: false,
		error: null,
		scrollTop: 0,
		history: [cwd],
		historyIndex: 0,
		activeIndex: 0,
	};
}

export const panelA = $state<PanelState>(draftPanel('/home/sam'));
export const panelB = $state<PanelState>(draftPanel('/home/sam'));

export type ViewName = 'browser' | 'editor' | 'terminal' | 'search' | 'tools' | 'settings';

// Svelte erlaubt das Exportieren von reassignierten Runes nicht.
// Deshalb liegen veränderliche Werte in $state-Objekten (.value) + Setter-Funktionen.
export const view = $state<{ value: ViewName }>({ value: 'browser' });
export function setView(v: ViewName) {
	view.value = v;
}
export const editorPath = $state<{ value: string | null }>({ value: null });
export function setEditorPath(p: string | null) {
	editorPath.value = p;
}
export const terminalStartCwd = $state<{ value: string }>({ value: '/home/sam' });
export function setTerminalCwd(c: string) {
	terminalStartCwd.value = c;
}
export const focusedPanel = $state<{ value: 0 | 1 }>({ value: 0 });
export function setFocusedPanel(i: 0 | 1) {
	focusedPanel.value = i;
}

// Lesezeichen (localStorage-persistent).
const BM_KEY = 'compound.bookmarks.v1';

function loadBookmarks(): string[] {
	try {
		return JSON.parse(localStorage.getItem(BM_KEY) ?? '[]') as string[];
	} catch {
		return [];
	}
}

export const bookmarks = $state<string[]>(loadBookmarks());

export function addBookmark(path: string) {
	if (!bookmarks.includes(path)) {
		bookmarks.push(path);
		saveBookmarks();
	}
}

export function removeBookmark(path: string) {
	const i = bookmarks.indexOf(path);
	if (i >= 0) {
		bookmarks.splice(i, 1);
		saveBookmarks();
	}
}

function saveBookmarks() {
	try {
		localStorage.setItem(BM_KEY, JSON.stringify(bookmarks));
	} catch {
		// ignorieren
	}
}

// App-interne Zwischenablage (Kopieren/Ausschneiden zwischen Panels).
export const clipboard = $state<{ paths: string[]; cut: boolean }>({ paths: [], cut: false });
export function setClipboard(paths: string[], cut: boolean) {
	clipboard.paths.splice(0, clipboard.paths.length, ...paths);
	clipboard.cut = cut;
}

// Laufende Operationen: opId -> Zustand.
export const ops = $state<Record<string, OpState>>({});

// Aktiver Konflikt (wird vom ConflictDialog bearbeitet).
export const pendingConflict = $state<ConflictRequest | null>(null);

// Modal-Dialoge der App: kind steuert, welcher Dialog gerendert wird.
export interface UIDialog {
	kind:
		| 'newitem'
		| 'rename'
		| 'search'
		| 'tools'
		| 'archive'
		| 'settings'
		| 'properties'
		| 'confirmDelete'
		| 'confirmTrash'
		| 'emptyTrash'
		| 'shortcuts';
	payload?: unknown;
}

export const uiDialog = $state<{ d: UIDialog | null }>({ d: null });

export function openDialog(d: UIDialog) {
	uiDialog.d = d;
}

export function closeDialog() {
	uiDialog.d = null;
}

// Notizen / Fehlerbanner.
let noticeId = 0;
export const notices = $state<Notice[]>([]);

export function notify(kind: NoticeKind, text: string, ms = 6000) {
	const id = ++noticeId;
	notices.push({ id, kind, text });
	setTimeout(() => {
		const i = notices.findIndex((n) => n.id === id);
		if (i >= 0) notices.splice(i, 1);
	}, ms);
}

export function clearNotices() {
	notices.splice(0, notices.length);
}

export function opNew(opId: string, label: string, total: number) {
	ops[opId] = { opId, label, total, current: 0, phase: 'running', done: false, error: null };
}

export function opProgress(opId: string, current: number, total: number, label: string) {
	const o = ops[opId];
	if (o) {
		o.current = current;
		o.total = total;
		if (label) o.label = label;
	}
}

export function opFinish(opId: string, ok: boolean, error: string | null) {
	const o = ops[opId];
	if (o) {
		o.done = true;
		o.error = error;
		if (ok) o.phase = 'ok';
	}
}

/** abgelaufene Ops automatisch nach einigen Sekunden aus der Liste entfernen */
export function sweepOps() {
	for (const k of Object.keys(ops)) {
		if (ops[k].done) {
			setTimeout(() => {
				if (ops[k]?.done) delete ops[k];
			}, 8000);
		}
	}
}

// op-finished-Ereignisse vom Backend an den Op-Store schließen.
export function bindOpFinished() {
	return getCurrentWebviewWindow().listen<{ opId: string; ok: boolean; error: AppErr | null }>(
		'op-finished',
		(e) => {
			opFinish(e.payload.opId, e.payload.ok, e.payload.error ? errMsg(e.payload.error) : null);
		},
	);
}

export async function runCancelOp(opId: string) {
	try {
		await invoke('cancel_op', { opId });
	} catch {
		// Operation möglicherweise schon beendet
	}
}

// ---- Panels ----

export function panelFor(i: 0 | 1): PanelState {
	return i === 0 ? panelA : panelB;
}

export async function refreshPanel(p: PanelState) {
	p.loading = true;
	p.error = null;
	try {
		const entries = (await invoke<FileEntry[]>('list_dir', { path: p.cwd })) as FileEntry[];
		if (settings.showHidden) {
			p.entries = entries;
		} else {
			p.entries = entries.filter((e) => !e.isHidden);
		}
		p.activeIndex = 0;
	} catch (e) {
		p.entries = [];
		p.error = errMsg(e);
	} finally {
		p.loading = false;
	}
}

export function navigate(p: PanelState, dir: string) {
	if (p.cwd === dir && p.entries.length > 0) return;
	p.cwd = dir;
	p.selected = [];
	p.scrollTop = 0;
	if (p.historyIndex < p.history.length - 1) {
		p.history = p.history.slice(0, p.historyIndex + 1);
	}
	p.history.push(dir);
	p.historyIndex = p.history.length - 1;
	refreshPanel(p);
}

export function navBack(p: PanelState) {
	if (p.historyIndex > 0) {
		p.historyIndex--;
		p.cwd = p.history[p.historyIndex];
		p.selected = [];
		refreshPanel(p);
	}
}

export function navFwd(p: PanelState) {
	if (p.historyIndex < p.history.length - 1) {
		p.historyIndex++;
		p.cwd = p.history[p.historyIndex];
		p.selected = [];
		refreshPanel(p);
	}
}

export function navUp(p: PanelState) {
	if (p.cwd.startsWith('trash:')) {
		navigate(p, '/home/sam');
		return;
	}
	const dir = p.cwd.replace(/\/+$/, '');
	const slash = dir.lastIndexOf('/');
	if (slash <= 0) return;
	navigate(p, dir.slice(0, slash) || '/');
}

export function selectedEntries(p: PanelState): FileEntry[] {
	const sel = new Set(p.selected);
	const map = new Map(p.entries.map((e) => [e.path, e]));
	const out: FileEntry[] = [];
	for (const s of sel) {
		const e = map.get(s);
		if (e) out.push(e);
	}
	return out;
}