<script lang="ts">
	import Toolbar from './lib/components/Toolbar.svelte';
	import Sidebar from './lib/components/Sidebar.svelte';
	import Panel from './lib/components/Panel.svelte';
	import StatusBar from './lib/components/StatusBar.svelte';
	import Notices from './lib/components/Notices.svelte';
	import InfoPane from './lib/components/InfoPane.svelte';
	import ProgressDialog from './lib/components/ProgressDialog.svelte';
	import ConflictDialog from './lib/components/ConflictDialog.svelte';
	import NewItemDialog from './lib/components/NewItemDialog.svelte';
	import RenameDialog from './lib/components/RenameDialog.svelte';
	import SearchDialog from './lib/components/SearchDialog.svelte';
	import ToolsDialog from './lib/components/ToolsDialog.svelte';
	import ArchiveDialog from './lib/components/ArchiveDialog.svelte';
	import SettingsDialog from './lib/components/SettingsDialog.svelte';
	import PropertiesDialog from './lib/components/PropertiesDialog.svelte';
	import ConfirmDialogs from './lib/components/ConfirmDialogs.svelte';
	import ShortcutsDialog from './lib/components/ShortcutsDialog.svelte';
	import EditorPane from './lib/components/EditorPane.svelte';
	import TerminalPane from './lib/components/TerminalPane.svelte';
	import { onMount } from 'svelte';
	import {
		view,
		settings,
		panelA,
		panelB,
		focusedPanel,
		uiDialog,
		openDialog,
		closeDialog,
		clipboard,
		ops,
		opNew,
		opProgress,
		renderOpLabel,
		errMsg,
		notify,
		refreshPanel,
		navigate,
		sweepOps,
		setFocusedPanel,
		setClipboard,
		bindOpFinished,
		clearNotices,
	} from './lib/stores.svelte';
	import { setConflictHandler, setProgressHandler } from './lib/api';
	import { sortEntries } from './lib/sort';
	import { copyItems, moveItems, resolveConflict } from './lib/api';
	import type { ConflictRequest } from './lib/types';

	onMount(async () => {
		// Handler: Konflikte sichtbar machen
		setConflictHandler((req: ConflictRequest) => {
			setPending(req);
		});

		// Handler: Fortschritt an Store (lokalisierte Labels aus Kinds+Params)
		setProgressHandler((opId, started, events) => {
			if (!ops[opId]) opNew(opId, renderOpLabel(started.kind), started.total);
			const last = events[events.length - 1];
			if (last) {
				opProgress(opId, last.current, last.total, renderOpLabel(last.kind, last.params ?? {}));
			}
		});

		// op-finished Events
		const un = await bindOpFinished();
		unsubs.push(un);
		sweepOps();

		// Initiale Panels laden
		refreshPanel(panelA);
		refreshPanel(panelB);
	});

	let pending = $state<ConflictRequest | null>(null);
	function setPending(req: ConflictRequest) {
		pending = req;
	}
	function clearPending() {
		pending = null;
	}

	let unsubs: Array<() => void> = [];

	const p = $derived(focusedPanel.value === 0 ? panelA : panelB);

	function selSorted(): string[] {
		const entries = sortEntries(p.entries, p.sortBy, p.sortDesc);
		const e = entries[p.activeIndex];
		return e ? [e.path] : p.selected;
	}

	function onKey(e: KeyboardEvent) {
		// Escape schließt immer das oberste Popup:
		// 1. laufender Konflikt -> Operation abbrechen (liest Antwort vom Backend ab)
		// 2. offener Dialog -> schließen
		// 3. sonst Toasts/Notices ausblenden (bewusst AUCH bei Fokus in Eingabefeldern,
		//    damit ESC in Dialogen immer funktioniert).
		if (e.key === 'Escape') {
			e.preventDefault();
			if (pending) {
				void resolveConflict(pending.opId, pending.key, { action: 'abort', applyToAll: false });
				clearPending();
				return;
			}
			if (uiDialog.d) {
				closeDialog();
				return;
			}
			clearNotices();
			return;
		}
		const target = e.target as HTMLElement | null;
		const inInput =
			target &&
			(target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.tagName === 'SELECT' || target.isContentEditable);
		if (inInput) return;
		const ctrl = e.ctrlKey || e.metaKey;
		const k = e.key.toLowerCase();

		if (k === 'f5' || (ctrl && k === 'r')) {
			e.preventDefault();
			refreshPanel(p);
			return;
		}
		// Suche öffnen
		if (ctrl && k === 'f') {
			e.preventDefault();
			openDialog({ kind: 'search' });
			return;
		}
		if (ctrl && k === 'n') {
			e.preventDefault();
			openDialog({ kind: 'newitem', payload: { destDir: p.cwd } });
			return;
		}
		if (ctrl && k === 'v') {
			e.preventDefault();
			if (clipboard.paths.length === 0) {
				notify('warn', 'Zwischenablage leer.');
				return;
			}
			void copyOrMove(clipboard.cut ? 'move' : 'copy', [...clipboard.paths], p.cwd.replace(/\/$/, '') + '/');
			return;
		}
		if (ctrl && k === 'c') {
			if (p.selected.length) {
				setClipboard([...p.selected], false);
				notify('ok', `Kopiert: ${p.selected.length}`);
			}
			return;
		}
		if (ctrl && k === 'x') {
			if (p.selected.length) {
				setClipboard([...p.selected], true);
				notify('info', `Ausgeschnitten: ${p.selected.length}`);
			}
			return;
		}
		if (k === 'f2') {
			e.preventDefault();
			const sel = [...selSorted()];
			if (sel.length === 1 && sel[0]) {
				openDialog({ kind: 'rename', payload: { path: sel[0], canonical: false } });
			}
			return;
		}
		if (k === 'backspace') {
			// Backspace = zurück zum übergeordneten Verzeichnis (Dateimanager-Standard).
			// Der Papierkorb läuft jetzt nur noch über Entf/Delete bzw. Strg+D.
			e.preventDefault();
			navDir('up');
			return;
		}
		if (k === 'delete') {
			e.preventDefault();
			if (p.selected.length === 0) return;
			openDialog({ kind: 'confirmTrash', payload: { paths: [...p.selected] } });
			return;
		}
		// Alt+Strg+Pfeil: Auswahl im Doppelpanel in das angrenzende Panel verschieben.
		// Pfeil rechts → rechtes Panel (B), Pfeil links → linkes Panel (A).
		// Muss VOR dem reinen Alt-Pfeil-Navigationsblock stehen (der fängt pfeiltasten sonst ab).
		if (e.altKey && e.ctrlKey && (k === 'arrowleft' || k === 'arrowright')) {
			e.preventDefault();
			if (settings.singlePanel) {
				notify('warn', 'Zum Verschieben werden zwei Panels benötigt.');
				return;
			}
			if (p.selected.length === 0) {
				notify('info', 'Nichts zum Verschieben ausgewählt.');
				return;
			}
			const dest = k === 'arrowright' ? panelB : panelA;
			const destIsActive = k === 'arrowright' ? focusedPanel.value === 1 : focusedPanel.value === 0;
			if (destIsActive) {
				notify('warn', 'Ziel-Panel ist bereits das aktive Panel.');
				return;
			}
			// Nach dem Verschieben beide Panels neu laden: Quelle (Selektion weg)
			// und Ziel (neu angekommene Dateien), damit kein Panel veraltet bleibt.
			void copyOrMove('move', [...p.selected], dest.cwd).then(() => {
				refreshPanel(panelA);
				refreshPanel(panelB);
			});
			return;
		}
		if (ctrl && k === 'd') {
			e.preventDefault();
			if (p.selected.length === 0) return;
			openDialog({ kind: 'confirmDelete', payload: { paths: [...p.selected] } });
			return;
		}
		if (k === 'enter') {
			e.preventDefault();
			const sel = [...selSorted()];
			// Muster B (Dual-Panel): eigene Auswahl leer, aber das ANDERE Panel
			// hat eine Markierung → Enter = „diese Markierung ins aktive
			// Verzeichnis HIER verschieben" (rechts markieren → links Enter).
			if (sel.length === 0) {
				const otherP = focusedPanel.value === 0 ? panelB : panelA;
				if (otherP.selected.length > 0) {
					void copyOrMove('move', [...otherP.selected], p.cwd).then(() => {
						otherP.selected = [];
						refreshPanel(panelA);
						refreshPanel(panelB);
					});
					return;
				}
			}
			const entry = p.entries.find((x) => x.path === sel[0]);
			if (!entry) return;
			if (entry.isDir) {
				navigate(p, entry.path);
			} else {
				void import('./lib/api').then(({ openDefault }) => openDefault(entry.path));
			}
			return;
		}
		if (k === ' ' || k === 'insert') {
			// Space/Insert = aktive Zeile zur Markierung hinzufügen/entfernen.
			e.preventDefault();
			const entries = sortEntries(p.entries, p.sortBy, p.sortDesc);
			if (entries.length === 0) return;
			const cur = entries[p.activeIndex];
			if (!cur) return;
			p.selected = p.selected.includes(cur.path)
				? p.selected.filter((x) => x !== cur.path)
				: [...p.selected, cur.path];
			return;
		}
		if (k === 'arrowdown' || k === 'arrowup' || k === 'home' || k === 'end') {
			const entries = sortEntries(p.entries, p.sortBy, p.sortDesc);
			if (entries.length === 0) return;
			e.preventDefault();
			let idx = p.activeIndex;
			if (k === 'arrowdown') idx = Math.min(entries.length - 1, idx + 1);
			else if (k === 'arrowup') idx = Math.max(0, idx - 1);
			else if (k === 'home') idx = 0;
			else idx = entries.length - 1;
			p.activeIndex = idx;
			p.selected = [entries[idx].path];
			return;
		}
		if (e.altKey && (k === 'arrowleft' || k === 'arrowright' || k === 'arrowup')) {
			e.preventDefault();
			if (k === 'arrowleft') navDir('back');
			else if (k === 'arrowright') navDir('fwd');
			else navDir('up');
			return;
		}
		if (k === 'tab') {
			e.preventDefault();
			if (settings.singlePanel) return;
			setFocusedPanel(focusedPanel.value === 0 ? 1 : 0);
			return;
		}
	}

	function navDir(dir: 'back' | 'fwd' | 'up') {
		if (dir === 'back') {
			if (p.historyIndex > 0) {
				p.historyIndex--;
				p.cwd = p.history[p.historyIndex];
				p.selected = [];
				refreshPanel(p);
			}
		} else if (dir === 'fwd') {
			if (p.historyIndex < p.history.length - 1) {
				p.historyIndex++;
				p.cwd = p.history[p.historyIndex];
				p.selected = [];
				refreshPanel(p);
			}
		} else {
			const dir0 = p.cwd.replace(/\/+$/, '');
			const slash = dir0.lastIndexOf('/');
			if (slash <= 0) return;
			navigate(p, dir0.slice(0, slash) || '/');
		}
	}

	function sameDir(paths: string[], dir: string): boolean {
		// Nur blockieren, wenn die Dateien DIREKT im Zielverzeichnis liegen
		// (dann wäre es ein No-Op). Dateien in einem UNTERordner des Ziels
		// sind z.B. ein legitimes "eine Ebene hoch": /a/b/f.txt nach /a.
		const norm = dir.replace(/\/+$/, '') || '/';
		return paths.every((x) => {
			const xp = x.replace(/\/+$/, '');
			const slash = xp.lastIndexOf('/');
			const parent = slash <= 0 ? '/' : xp.slice(0, slash);
			return parent === norm;
		});
	}

	async function copyOrMove(kind: 'copy' | 'move', srcs: string[], dest: string) {
		const clean = dest.replace(/\/+$/, '');
		if (sameDir(srcs, clean)) {
			notify('warn', 'Quelle und Ziel sind gleich.');
			return;
		}
		try {
			if (kind === 'copy') await copyItems(srcs, clean, 'ask');
			else await moveItems(srcs, clean, 'ask');
		} catch (e) {
			notify('error', errMsg(e));
		}
	}

	// Fokus: Klick auf Panel setzt Fokus
	function focusA() {
		setFocusedPanel(0);
	}
	function focusB() {
		setFocusedPanel(1);
	}
</script>

<svelte:window onkeydown={onKey} />

<div style="height:100vh; display:flex; flex-direction:column;">
	<Toolbar />

	<div style="flex:1; display:flex; min-height:0;">
		<Sidebar />
		<div class="panes" style="flex:1; min-width:0;">
			{#if view.value === 'browser'}
				<div style="flex:1; display:flex; min-width:0;" role="button" tabindex="-1" onclick={focusA} onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); focusA(); } }}>
					<Panel p={panelA} index={0} />
				</div>
				{#if !settings.singlePanel}
					<div class="pane-split" role="separator" aria-orientation="vertical" aria-label="Trennung zwischen Panels"></div>
					<div style="flex:1; display:flex; min-width:0;" role="button" tabindex="-1" onclick={focusB} onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); focusB(); } }}>
						<Panel p={panelB} index={1} />
					</div>
				{:else}
					<div class="pane-split" role="separator" aria-orientation="vertical" aria-label="Trennung zwischen Panels"></div>
					<InfoPane p={panelA} index={0} />
				{/if}
			{:else if view.value === 'editor'}
				<EditorPane />
			{:else if view.value === 'terminal'}
				<TerminalPane />
			{/if}
		</div>
	</div>

	<StatusBar />
</div>

<Notices />
<ProgressDialog />
{#if pending}
	<ConflictDialog req={pending} onDone={clearPending} />
{/if}

{#if uiDialog.d}
	{#if uiDialog.d.kind === 'newitem'}
		<NewItemDialog />
	{:else if uiDialog.d.kind === 'rename'}
		<RenameDialog />
	{:else if uiDialog.d.kind === 'search'}
		<SearchDialog />
	{:else if uiDialog.d.kind === 'tools'}
		<ToolsDialog />
	{:else if uiDialog.d.kind === 'archive'}
		<ArchiveDialog />
	{:else if uiDialog.d.kind === 'settings'}
		<SettingsDialog />
	{:else if uiDialog.d.kind === 'properties'}
		<PropertiesDialog />
{:else if uiDialog.d.kind === 'confirmDelete' || uiDialog.d.kind === 'confirmTrash' || uiDialog.d.kind === 'emptyTrash'}
			<ConfirmDialogs />
		{:else if uiDialog.d.kind === 'shortcuts'}
			<ShortcutsDialog />
		{/if}
{/if}