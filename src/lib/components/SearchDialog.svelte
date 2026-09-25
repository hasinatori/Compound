<script lang="ts">
	import Icon from './Icon.svelte';
	import {
		uiDialog,
		closeDialog,
		notify,
		settings,
		panelA,
		panelB,
		focusedPanel,
		lang,
		errMsg,
	} from '../stores.svelte';
	import { t } from '../i18n';
	import { searchIndex, searchNow, indexFolder, indexedRoots, clearIndex } from '../api';
	import type { SearchResult, SearchOpts, IndexedRoot } from '../types';
	import { formatBytes, formatDate } from '../format';

	const d = $derived(uiDialog.d?.kind === 'search' ? true : false);
	let query = $state('');
	let content = $state(true);
	let includeHidden = $state(true);
	let nameOnly = $state(false);
	let maxSize = $state('');
	let running = $state(false);
	let results = $state<SearchResult[]>([]);
	let usedIndex = $state(false);
	let roots = $state<IndexedRoot[]>([]);

	$effect(() => {
		void (async () => {
			try {
				roots = await indexedRoots();
			} catch {
				roots = [];
			}
		})();
	});

	const baseDir = $derived((focusedPanel.value === 0 ? panelA : panelB).cwd);

	function makeOpts(): SearchOpts {
		const ms = maxSize.trim() ? Number(maxSize.trim()) * 1024 * 1024 : null;
		return {
			includeHidden,
			nameOnly,
			maxSize: ms && Number.isFinite(ms) ? ms : null,
			kinds: [],
		};
	}

	async function doIndex() {
		notify('info', t('search.indexing', lang()));
		try {
			await indexFolder(baseDir, false, () => {});
			roots = await indexedRoots();
			notify('ok', t('search.indexUpdated', lang()));
		} catch (e) {
			notify('error', errMsg(e));
		}
	}

	async function runSearch() {
		const q = query.trim();
		if (!q) return;
		running = true;
		results = [];
		try {
			// Erst Index-Suche, bei Treffern fertig; sonst Live-Suche.
			const idx = await searchIndex(q, baseDir, makeOpts(), () => {});
			if (idx.results.length > 0 || idx.usedIndex) {
				results = idx.results;
				usedIndex = idx.usedIndex;
			} else {
				const live = await searchNow(q, baseDir, content, makeOpts(), () => {});
				results = live.results;
				usedIndex = live.usedIndex;
			}
		} catch (e) {
			notify('error', errMsg(e));
			// Fallback: Live-Suche
			try {
				const live = await searchNow(q, baseDir, content, makeOpts(), () => {});
				results = live.results;
				usedIndex = false;
			} catch (e2) {
				notify('error', String(e2));
			}
		} finally {
			running = false;
		}
	}

	async function openResult(r: SearchResult) {
		const { openDefault } = await import('../api');
		if (r.isDir) {
			const p = focusedPanel.value === 0 ? panelA : panelB;
			p.cwd = r.path;
			closeDialog();
		} else {
			await openDefault(r.path);
		}
	}

	async function doClearIndex() {
		try {
			await clearIndex(baseDir);
			roots = await indexedRoots();
			notify('ok', t('search.indexCleared', lang()));
		} catch (e) {
			notify('error', errMsg(e));
		}
	}
</script>

{#if d}
	<div class="overlay">
		<div class="dialog" style="min-width: 640px; max-width: 90vw; height: 80vh;">
			<div class="dialog-header">
				<Icon name="search" size={16} />&nbsp;{t('search.title', lang())}
				<button class="toolbar-btn" onclick={closeDialog}><Icon name="close" size={15} /></button>
			</div>
			<div class="dialog-body" style="display:flex; flex-direction:column; gap:12px;">
				<div class="form-row">
					<label for="search-query">{t('search.query', lang())}</label>
					<input
						id="search-query"
						bind:value={query}
						onkeydown={(e) => e.key === 'Enter' && !running && void runSearch()}
					/>
				</div>
				<div style="display:flex; gap:16px; flex-wrap:wrap; align-items:center;">
					<label style="display:flex; gap:6px; align-items:center; font-size:13px; cursor:pointer;">
						<input type="checkbox" bind:checked={content} /> In Inhalten suchen
					</label>
					<label style="display:flex; gap:6px; align-items:center; font-size:13px; cursor:pointer;">
						<input type="checkbox" bind:checked={includeHidden} /> Versteckte
					</label>
					<label style="display:flex; gap:6px; align-items:center; font-size:13px; cursor:pointer;">
						<input type="checkbox" bind:checked={nameOnly} /> Nur Namen
					</label>
					<label style="display:flex; gap:6px; align-items:center; font-size:13px;">
						Max MB:
						<input type="number" bind:value={maxSize} style="width:80px;" />
					</label>
				</div>
				<div style="display:flex; gap:8px; align-items:center;">
					<button class="btn primary" onclick={() => void runSearch()} disabled={running || !query.trim()}>
						<Icon name="search" size={14} /> Suchen
					</button>
					<button class="btn" onclick={() => void doIndex()} disabled={running}>
						<Icon name="refresh" size={14} /> Index aufbauen
					</button>
					<button class="btn" onclick={() => void doClearIndex()}>
						<Icon name="trash" size={14} /> {t('search.clearIndex', lang())}
					</button>
					<span style="font-size:12px; color:var(--text-faint); flex:1; text-align:right;">
						{t('search.roots', lang(), { n: roots.length, dir: baseDir })}
					</span>
				</div>
				<hr style="border:none; border-top:1px solid var(--border); margin:2px 0;" />
				<div style="flex:1; overflow:auto; min-height: 120px;">
					{#if running}
						<div style="text-align:center; color:var(--text-dim); padding:24px;">{t('search.searching', lang())}</div>
					{:else if results.length === 0 && query.trim()}
						<div style="text-align:center; color:var(--text-faint); padding:24px;">
							{t('search.noResults', lang())}
						</div>
					{:else if results.length > 0}
						<table class="tbl">
							<thead>
								<tr>
									<th>{t('prop.name', lang())}</th>
									<th>{t('prop.path', lang())}</th>
									<th style="text-align:right;">{t('prop.size', lang())}</th>
									<th>{t('prop.modified', lang())}</th>
								</tr>
							</thead>
							<tbody>
								{#each results as r (r.path)}
									<tr style="cursor:pointer;" onclick={() => void openResult(r)}>
										<td>
											<span style="display:inline-flex; gap:6px; align-items:center;">
												<Icon name={r.isDir ? 'folder' : 'file'} size={14} />
												{r.name}
											</span>
										</td>
										<td style="color:var(--text-dim); word-break:break-all;">
											{r.path}
										</td>
										<td style="text-align:right;">{formatBytes(r.size)}</td>
										<td style="color:var(--text-dim);">{formatDate(r.mtimeMs)}</td>
									</tr>
								{/each}
							</tbody>
						</table>
					{/if}
				</div>
			</div>
		</div>
	</div>
{/if}