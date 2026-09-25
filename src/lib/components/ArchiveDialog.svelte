<script lang="ts">
	// Архивы: создать/распаковать zip|tar|gz|bz2|xz, список содержимого.
	// Archives: create/extract zip|tar|gz|bz2|xz, list contents.
	import Icon from './Icon.svelte';
	import {
		uiDialog,
		closeDialog,
		notify,
		panelA,
		panelB,
		focusedPanel,
		lang,
		errMsg,
	} from '../stores.svelte';
	import { t } from '../i18n';
	import { archiveCreate, archiveExtract, archiveList } from '../api';
	import type { ArchiveEntry, ArchiveSummary } from '../types';
	import { formatBytes } from '../format';

	interface Payload {
		mode: 'create' | 'extract';
		srcs?: string[]; // create: Quelldateien
		archive?: string; // extract: Archiv
	}

	const d = $derived<Payload | null>(
		uiDialog.d?.kind === 'archive' ? (uiDialog.d.payload as Payload) : null,
	);

	const activeDir = $derived((focusedPanel.value === 0 ? panelA : panelB).cwd);

	// Create
	let dest = $state('');
	let createMsg = $state<string | null>(null);
	// Extract
	let destDir = $state('');
	let entries = $state<ArchiveEntry[]>([]);
	let listMsg = $state<string | null>(null);
	let busy = $state(false);

	$effect(() => {
		if (d?.mode === 'create') {
			dest = `${activeDir}/${d.srcs?.[0]?.split('/').pop() ?? 'archiv'}.zip`;
			createMsg = null;
		} else if (d?.mode === 'extract') {
			destDir = `${activeDir}/${(d.archive ?? 'archiv').split('/').pop()?.replace(/\.[^.]+$/, '') ?? 'extract'}`;
			listMsg = null;
			entries = [];
			void (async () => {
				try {
					entries = await archiveList(d.archive ?? '');
				} catch (e) {
					listMsg = errMsg(e);
				}
			})();
		}
	});

	async function submitCreate() {
		if (!d?.srcs || !dest.trim()) return;
		busy = true;
		try {
			const sum: ArchiveSummary = await archiveCreate(d.srcs, dest.trim(), () => {});
			notify('ok', t('notice.archiveCreated', lang(), { n: sum.entries, size: formatBytes(sum.bytes) }));
			closeDialog();
		} catch (e) {
			notify('error', errMsg(e));
		} finally {
			busy = false;
		}
	}

	async function submitExtract() {
		if (!d?.archive || !destDir.trim()) return;
		busy = true;
		try {
			const sum: ArchiveSummary = await archiveExtract(d.archive, destDir.trim(), () => {});
			notify('ok', t('notice.extracted', lang(), { n: sum.entries }));
			closeDialog();
		} catch (e) {
			notify('error', errMsg(e));
		} finally {
			busy = false;
		}
	}
</script>

{#if d}
	<div class="overlay">
		<div class="dialog" style="width: 720px; max-width: 92vw;">
			<div class="dialog-header">
				<Icon name="archive" size={16} />&nbsp;
				{d.mode === 'create' ? t('archive.create', lang()) : t('archive.extract', lang())}
				<button class="toolbar-btn" onclick={closeDialog}><Icon name="close" size={15} /></button>
			</div>
			<div class="dialog-body">
				{#if d.mode === 'create'}
					<div class="form-row">
						<label for="archive-dest">{t('archive.destFile', lang())}</label>
						<input id="archive-dest" bind:value={dest} />
					</div>
					<div style="max-height:140px; overflow:auto; background:var(--bg-active); border-radius:6px; padding:8px; font-size:12px; color:var(--text-dim);">
						{#each d.srcs ?? [] as s (s)}
							<div style="word-break:break-all;">· {s}</div>
						{/each}
					</div>
				{:else}
					<div class="form-row">
						<label for="archive-destdir">{t('archive.destDir', lang())}</label>
						<input id="archive-destdir" bind:value={destDir} />
					</div>
					<div class="form-row">
						<div>{t('archive.list', lang())}</div>
						<div style="max-height:180px; overflow:auto; background:var(--bg-active); border-radius:6px; padding:8px; font-size:12px; color:var(--text-dim);">
							{#if listMsg}
								<div style="color:var(--err);">{listMsg}</div>
							{:else if entries.length === 0}
								<div>{t('misc.loading', lang())}</div>
							{:else}
								{#each entries as e (e.path)}
									<div style="display:flex; gap:6px; align-items:center;">
										<Icon name={e.isDir ? 'folder' : 'file'} size={12} />
										<span style="flex:1; word-break:break-all;">{e.path}</span>
										<span>{e.isDir ? '' : formatBytes(e.size)}</span>
									</div>
								{/each}
							{/if}
						</div>
					</div>
				{/if}
			</div>
			<div class="dialog-footer">
				<button class="btn" onclick={closeDialog}>{t('action.cancel', lang())}</button>
				<button
					class="btn primary"
					onclick={() => void (d.mode === 'create' ? submitCreate() : submitExtract())}
					disabled={busy}
				>
					{d.mode === 'create' ? 'Erstellen' : 'Extrahieren'}
				</button>
			</div>
		</div>
	</div>
{/if}