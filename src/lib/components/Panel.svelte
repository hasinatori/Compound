<script lang="ts">
	// Одна файловая панель: список, выделение, контекст-меню, инлайн-переименование.
	// One file panel: list, selection, context menu, inline rename.
	// Левый клик меняет фокус панели; dbl-click открывает папку/файл.
	// Left click sets panel focus; dbl-click opens a folder/file.
	// Выделение = p.selected, p.activeIndex — двигают хоткеи и Enter.
	// Selection lives in p.selected / p.activeIndex — keys and Enter read those.
	import Icon from './Icon.svelte';
	import Breadcrumb from './Breadcrumb.svelte';
	import type { FileEntry, PanelState } from '../types';
	import { formatBytes, formatDate, fileIconName } from '../format';
	import { settings } from '../stores.svelte';
	import {
		navigate,
		navBack,
		navFwd,
		navUp,
		refreshPanel,
		bookmarks,
		addBookmark,
		removeBookmark,
		openDialog,
		clipboard,
		notify,
		errMsg,
		renderOpLabel,
	} from '../stores.svelte';
	import {
		openDefault,
		openExternalTerminal,
		duplicateItem,
		renameItem,
		trashItems,
		deletePermanent,
		copyItems,
		moveItems,
	} from '../api';
	import { t } from '../i18n';
	import { setView, setTerminalCwd, setClipboard } from '../stores.svelte';
	import { sortEntries } from '../sort';

	let { p, index }: { p: PanelState; index: 0 | 1 } = $props();

	const lang = () => settings.language;
	const ROW_H = 28;
	const VIS_OVER = 12;

	// ---- Sortierung ----
	const sorted = $derived(sortEntries(p.entries, p.sortBy, p.sortDesc));

	function toggleSort(key: PanelState['sortBy']) {
		if (p.sortBy === key) {
			p.sortDesc = !p.sortDesc;
		} else {
			p.sortBy = key;
			p.sortDesc = key === 'name' ? false : true;
		}
	}

	// ---- Виртуализация (только вид списком) ----
	// ---- Virtualization (list view only) ----
	let scrollEl = $state<HTMLDivElement | null>(null);
	let viewportEl = $state<HTMLDivElement | null>(null);
	let scrollTop = $state(0);
	let viewportH = $state(600);

	$effect(() => {
		const el = viewportEl;
		if (!el) return;
		const ro = new ResizeObserver(() => {
			viewportH = el.clientHeight;
		});
		ro.observe(el);
		return () => ro.disconnect();
	});

	$effect(() => {
		if (renameTarget && renameInput) {
			renameInput.focus();
			renameInput.select();
		}
	});

	const listH = $derived(sorted.length * ROW_H);
	const firstVisible = $derived(Math.max(0, Math.floor(scrollTop / ROW_H) - VIS_OVER));
	const lastVisible = $derived(
		Math.min(sorted.length, Math.ceil((scrollTop + viewportH) / ROW_H) + VIS_OVER),
	);
	const visibleRows = $derived(sorted.slice(firstVisible, lastVisible));

	function onScroll() {
		if (scrollEl) {
			scrollTop = scrollEl.scrollTop;
			p.scrollTop = scrollTop;
		}
	}

	// ---- Auswahl ----
	function onRowClick(e: MouseEvent, row: FileEntry, idx: number) {
		if (renameTarget) return;
		if (e.ctrlKey || e.metaKey) {
			const i = p.selected.indexOf(row.path);
			if (i >= 0) p.selected.splice(i, 1);
			else p.selected.push(row.path);
		} else if (e.shiftKey) {
			// Диапазон от активной строки до сюда.
		// Range from the active row down to here
			const anchor = p.activeIndex;
			const lo = Math.min(anchor, idx);
			const hi = Math.max(anchor, idx);
			p.selected = sorted.slice(lo, hi + 1).map((x) => x.path);
		} else {
			p.selected = [row.path];
			p.activeIndex = idx;
			focusedIdx = idx;
		}
		(e.currentTarget as HTMLElement | null)?.focus();
	}

	/// Tastaturaktivierung einer Zeile/Kachel (Enter öffnet, Leertaste wählt).
	function onRowKeydown(e: KeyboardEvent, row: FileEntry, idx: number) {
		if (e.key !== 'Enter' && e.key !== ' ') return;
		// Не даём глобальному хендлеру сработать дважды.
		// Don't let the global window handler fire twice
		e.stopPropagation();
		e.preventDefault();
		if (e.key === 'Enter') {
			onDblClick(row);
		} else {
			onRowClick({ ctrlKey: false, shiftKey: false } as unknown as MouseEvent, row, idx);
		}
	}

	function onDblClick(row: FileEntry) {
		if (row.isDir) {
			navigate(p, row.path);
		} else {
			void openDefault(row.path);
		}
	}

	// Клавиатура по строкам (Enter, стрелки и т.д.).
	// Keyboard handling on the rows (Enter, arrows, ...)
	let focusedIdx = $state(0);
	function scrollToIdx(idx: number) {
		if (!scrollEl) return;
		const target = idx * ROW_H;
		if (target < scrollEl.scrollTop) scrollEl.scrollTop = target;
		else if (target + ROW_H > scrollEl.scrollTop + viewportH)
			scrollEl.scrollTop = target - viewportH + ROW_H;
	}

	// ---- Kontextmenü ----
	let ctx = $state<{ x: number; y: number; entry: FileEntry | null } | null>(null);

	function onCtx(e: MouseEvent, entry: FileEntry | null) {
		e.preventDefault();
		// ПКМ по невыделенной строке: сначала выделяем её.
			// Right-click on an unselected row: select it first
		if (entry && !p.selected.includes(entry.path)) {
			p.selected = [entry.path];
		}
		const x = Math.min(Math.max(e.clientX, 8), window.innerWidth - 250);
		const y = Math.min(Math.max(e.clientY, 8), window.innerHeight - 340);
		ctx = { x, y, entry };
	}

	function onPaneCtx(e: MouseEvent) {
		// Hintergrund-Kontextmenü (leere Fläche)
		if ((e.target as HTMLElement).closest('.file-row, .grid-item')) return;
		onCtx(e, null);
	}

	function selPaths(): string[] {
		return p.selected.length > 0 ? p.selected : (ctx?.entry ? [ctx.entry.path] : []);
	}

	async function doDegree(degree: 'copy' | 'move', srcs: string[]) {
		try {
			const started =
				degree === 'copy'
					? await copyItems(srcs, p.cwd, 'ask')
					: await moveItems(srcs, p.cwd, 'ask');
			notify('info', `${renderOpLabel(started.kind)} — ${srcs.length} ${t(srcs.length === 1 ? 'misc.element' : 'misc.elements', lang())}`);
		} catch (e) {
			notify('error', errMsg(e));
		}
	}

	async function ctxNew(destDir: string) {
		openDialog({ kind: 'newitem', payload: { destDir } });
	}

	async function actionTrash(paths: string[]) {
		if (paths.length === 0) return;
		openDialog({ kind: 'confirmTrash', payload: { paths } });
	}

	async function actionDelete(paths: string[]) {
		if (paths.length === 0) return;
		openDialog({ kind: 'confirmDelete', payload: { paths } });
	}

	async function actionPaste() {
		if (clipboard.paths.length === 0) {
			notify('warn', t('notice.nothingInClipboard', lang()));
			return;
		}
		await doDegree(clipboard.cut ? 'move' : 'copy', [...clipboard.paths]);
	}

	function openTerminalHere() {
		setTerminalCwd(p.cwd);
		setView('terminal');
	}

	async function actionDuplicate(path: string) {
		try {
			const name = await duplicateItem(path);
			notify('ok', t('notice.duplicated', lang(), { name }));
			void refreshPanel(p);
		} catch (e) {
			notify('error', errMsg(e));
		}
	}

	// ---- Inline-Umbenennen ----
	let renameTarget = $state<FileEntry | null>(null);
	let renameValue = $state('');
	let renameInput = $state<HTMLInputElement | null>(null);

	function startRename(entry: FileEntry) {
		renameTarget = entry;
		renameValue = entry.name;
	}

	async function commitRename() {
		if (!renameTarget) return;
		const target = renameTarget;
		renameTarget = null;
		const newName = renameValue.trim();
		if (!newName || newName === target.name) return;
		try {
			await renameItem(target.path, newName);
			notify('ok', t('notice.renamed', lang(), { name: newName }));
			void refreshPanel(p);
		} catch (e) {
			notify('error', errMsg(e));
		}
	}

	// ---- Drag & Drop externer Dateien (OS) ----
	let dragOver = $state(false);

	function onDrop(e: DragEvent) {
		e.preventDefault();
		dragOver = false;
		const files = e.dataTransfer?.files;
		if (!files || files.length === 0) return;
		const paths = [...files]
			.map((f) => (f as unknown as { path?: string }).path)
			.filter((x): x is string => Boolean(x));
		if (paths.length === 0) return;
		void doDegree('copy', paths);
	}

	function onCtxAction(fn: () => void) {
		ctx = null;
		fn();
	}

	const getSel = () => (ctx?.entry && !p.selected.length ? [ctx.entry.path] : [...p.selected]);

	// ---- Favoriten (Ordner speichern) ----
	const isBookmarked = $derived(bookmarks.includes(p.cwd));

	function toggleBookmark() {
		if (bookmarks.includes(p.cwd)) removeBookmark(p.cwd);
		else addBookmark(p.cwd);
	}
</script>

<div class="panel" id="panel-{index}" role="group" aria-label="Panel {index + 1}">
	<!-- Toolbar -->
	<div class="panel-toolbar">
		<button class="toolbar-btn" title={t('action.back', lang())} onclick={() => navBack(p)}>
			<Icon name="back" size={15} />
		</button>
		<button class="toolbar-btn" title={t('action.forward', lang())} onclick={() => navFwd(p)}>
			<Icon name="forward" size={15} />
		</button>
		<button class="toolbar-btn" title={t('action.up', lang())} onclick={() => navUp(p)}>
			<Icon name="up" size={15} />
		</button>
		<button class="toolbar-btn" title={t('action.home', lang())} onclick={() => navigate(p, '/home/sam')}>
			<Icon name="home" size={15} />
		</button>
		<button class="toolbar-btn" title={t('action.refresh', lang())} onclick={() => refreshPanel(p)}>
			<Icon name="refresh" size={15} />
		</button>
		<button
			class="toolbar-btn"
			class:active={isBookmarked}
			title={isBookmarked ? t('action.unbookmark', lang()) : t('action.bookmark', lang())}
			onclick={toggleBookmark}
		>
			<Icon name="bookmark" size={15} />
		</button>
		<div class="sep-v"></div>
		<select
			value={p.sortBy}
			onchange={(e) => toggleSort((e.currentTarget as HTMLSelectElement).value as PanelState['sortBy'])}
			title={t('settings.sort', lang())}
		>
			<option value="name">{t('prop.name', lang())}</option>
			<option value="ext">{t('sort.ext', lang())}</option>
			<option value="size">{t('prop.size', lang())}</option>
			<option value="mtime">{t('sort.date', lang())}</option>
		</select>
		<button
			class="toolbar-btn"
			title={t('panel.sortDir', lang())}
			onclick={() => (p.sortDesc = !p.sortDesc)}
		>
			<Icon name={p.sortDesc ? 'sortDesc' : 'sort'} size={15} />
		</button>
		<div class="sep-v"></div>
		<button class="toolbar-btn" title="Listenansicht" onclick={() => (p.viewMode = 'list')}>
			<Icon name="list" size={15} />
		</button>
		<button class="toolbar-btn" title="Kachelansicht" onclick={() => (p.viewMode = 'grid')}>
			<Icon name="grid" size={15} />
		</button>
		<div style="flex:1"></div>
		{#if p.selected.length > 0}
			<span style="font-size:12px;color:var(--text-dim)">
				{t('panel.selected', lang(), { n: p.selected.length })}
			</span>
		{/if}
	</div>

	<Breadcrumb {p} />

	<!-- Dateiliste -->
	<div
		class="panel-body"
		role="group"
		bind:this={viewportEl}
		oncontextmenu={onPaneCtx}
	>
		{#if p.viewMode === 'list'}
			<div
				class="list-scroll"
				role="group"
				bind:this={scrollEl}
				onscroll={onScroll}
				ondragenter={(e) => {
					e.preventDefault();
					dragOver = true;
				}}
				ondragleave={(e) => {
					if (!e.relatedTarget) dragOver = false;
				}}
				ondragover={(e) => e.preventDefault()}
				ondrop={onDrop}
				style="height:100%; overflow-y:auto; position:relative;"
			>
				<div class="virtual-spacer" style="height: {listH}px; position:relative;">
					{#each visibleRows as row, i (row.path)}
						{@const idx = i + firstVisible}
						<div
							class="file-row"
							role="button"
							tabindex="-1"
							aria-label={row.name}
							class:selected={p.selected.includes(row.path)}
							class:focused={idx === p.activeIndex}
							class:drop-target={dragOver}
							style="position:absolute; top:{idx * ROW_H}px; left:0; right:0;"
							onclick={(e) => onRowClick(e, row, idx)}
							ondblclick={() => onDblClick(row)}
							oncontextmenu={(e) => onCtx(e, row)}
							onkeydown={(e) => onRowKeydown(e, row, idx)}
						>
							<span class="kind-icon {row.isDir ? 'dir' : row.kind}">
								<Icon name={fileIconName(row)} size={16} />
							</span>
							{#if renameTarget?.path === row.path}
								<span class="name-edit">
									<input
										bind:this={renameInput}
										bind:value={renameValue}
										onkeydown={(e) => {
											if (e.key === 'Enter') void commitRename();
											if (e.key === 'Escape') renameTarget = null;
										}}
										onblur={() => void commitRename()}
									/>
								</span>
							{:else}
								<span class="name" title={row.name}>{row.name}</span>
							{/if}
							<span class="meta" style="width:56px;text-align:right;">{#if !row.isDir}{formatBytes(row.size)}{/if}</span>
							<span class="meta" style="width:90px;">{formatDate(row.mtimeMs, lang())}</span>
							<span class="meta" style="width:60px;">{row.perms}</span>
						</div>
					{/each}
				</div>
				{#if p.entries.length === 0 && !p.loading && !p.error}
					<div class="empty-hint">{t('panel.empty', lang())}</div>
				{/if}
			</div>
		{:else}
			<div
				class="grid-view"
				role="group"
				oncontextmenu={onPaneCtx}
				ondragenter={(e) => {
					e.preventDefault();
					dragOver = true;
				}}
				ondragover={(e) => e.preventDefault()}
				ondrop={onDrop}
			>
				{#each sorted as row}
					<div
						class="grid-item"
						role="button"
						tabindex="-1"
						aria-label={row.name}
						class:selected={p.selected.includes(row.path)}
						onclick={(e) => {
							onRowClick(e, row, sorted.indexOf(row));
						}}
						ondblclick={() => onDblClick(row)}
						oncontextmenu={(e) => onCtx(e, row)}
						onkeydown={(e) => onRowKeydown(e, row, sorted.indexOf(row))}
					>
						<Icon name={fileIconName(row)} size={30} />
						<span class="gname">{row.name}</span>
					</div>
				{/each}
				{#if p.entries.length === 0 && !p.loading && !p.error}
					<span class="empty-hint">{t('panel.empty', lang())}</span>
				{/if}
			</div>
		{/if}

		{#if p.loading}
			<div class="empty-hint">{t('panel.loading', lang())}</div>
		{/if}
		{#if p.error}
			<div class="error-hint">{p.error}</div>
		{/if}
	</div>
</div>

<!-- Escape schließt Kontextmenü und Inline-Umbenennen (Dialoge: zentral in App.svelte). -->
<svelte:window onkeydown={(e) => { if (e.key === 'Escape') { ctx = null; renameTarget = null; } }} />

{#if ctx}
	<div
		class="ctx-menu"
		role="menu"
		tabindex="-1"
		style="left:{ctx.x}px; top:{ctx.y}px;"
		oncontextmenu={(e) => e.preventDefault()}
		onmouseleave={() => (ctx = null)}
	>
		{#if ctx.entry}
			<button
				class="ctx-item"
				onclick={() => onCtxAction(() => (ctx!.entry!.isDir ? navigate(p, ctx!.entry!.path) : void openDefault(ctx!.entry!.path)))}
			>
				<Icon name={ctx.entry.isDir ? 'folderOpen' : 'file'} size={14} />
				{t('ctx.open', lang())}
			</button>
			{#if !ctx.entry.isDir}
				<button class="ctx-item" onclick={() => onCtxAction(() => openDefault(ctx!.entry!.path))}>
					<Icon name="openEx" size={14} />
					{t('ctx.openDefault', lang())}
				</button>
			{/if}
			<button class="ctx-item" onclick={() => onCtxAction(() => openDialog({ kind: 'properties', payload: { entry: ctx!.entry } }))}>
				<Icon name="info" size={14} />
				{t('ctx.properties', lang())}
			</button>
			<div class="ctx-sep"></div>
			<button class="ctx-item" onclick={() => onCtxAction(() => { setClipboard(getSel(), false); notify('ok', t('notice.copied', lang())); })}>
				<Icon name="copy" size={14} />
				{t('ctx.copy', lang())}
			</button>
			<button class="ctx-item" onclick={() => onCtxAction(() => { setClipboard(getSel(), true); notify('info', t('notice.cut', lang())); })}>
				<Icon name="cut" size={14} />
				{t('ctx.cut', lang())}
			</button>
			<div class="ctx-sep"></div>
			<button class="ctx-item" onclick={() => onCtxAction(() => startRename(ctx!.entry!))}>
				<Icon name="rename" size={14} />
				{t('ctx.rename', lang())}
			</button>
			<button class="ctx-item" onclick={() => onCtxAction(() => actionDuplicate(ctx!.entry!.path))}>
				<Icon name="duplicate" size={14} />
				{t('ctx.duplicate', lang())}
			</button>
			<button class="ctx-item" onclick={() => onCtxAction(() => openDialog({ kind: 'archive', payload: { mode: 'create', srcs: getSel() } }))}>
				<Icon name="archive" size={14} />
				{t('ctx.archive', lang())}
			</button>
			<div class="ctx-sep"></div>
			<button class="ctx-item" onclick={() => onCtxAction(openTerminalHere)}>
				<Icon name="terminal" size={14} />
				{t('ctx.terminalHere', lang())}
			</button>
			<button class="ctx-item" onclick={() => onCtxAction(() => openExternalTerminal(p.cwd))}>
				<Icon name="external" size={14} />
				{t('ctx.external', lang())}
			</button>
			<div class="ctx-sep"></div>
			<button class="ctx-item danger" onclick={() => onCtxAction(() => actionTrash(getSel()))}>
				<Icon name="trash" size={14} />
				{t('ctx.trash', lang())}
			</button>
			<button class="ctx-item danger" onclick={() => onCtxAction(() => actionDelete(getSel()))}>
				<Icon name="error" size={14} />
				{t('ctx.delete', lang())}
			</button>
		{:else}
			<button class="ctx-item" onclick={() => onCtxAction(() => ctxNew(p.cwd))}>
				<Icon name="newFile" size={14} />
				{t('ctx.newFile', lang())}
			</button>
			<button class="ctx-item" onclick={() => onCtxAction(() => ctxNew(p.cwd))}>
				<Icon name="newFolder" size={14} />
				{t('ctx.newFolder', lang())}
			</button>
			<div class="ctx-sep"></div>
			<button class="ctx-item" onclick={() => onCtxAction(() => actionPaste())}>
				<Icon name="paste" size={14} />
				{t('ctx.paste', lang())}
			</button>
			{#if clipboard.paths.length > 0}
				<button class="ctx-item" onclick={() => onCtxAction(() => (clipboard.paths.splice(0, clipboard.paths.length)))}>
					<Icon name="close" size={14} />
					Zwischenablage leeren
				</button>
			{/if}
			<div class="ctx-sep"></div>
			<button class="ctx-item" onclick={() => onCtxAction(() => refreshPanel(p))}>
				<Icon name="refresh" size={14} />
				{t('ctx.refresh', lang())}
			</button>
			<button class="ctx-item" onclick={() => onCtxAction(() => addBookmark(p.cwd))}>
				<Icon name="bookmark" size={14} />
				Lesezeichen hinzufügen
			</button>
		{/if}
	</div>
{/if}