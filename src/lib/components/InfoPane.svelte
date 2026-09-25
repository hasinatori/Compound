<script lang="ts">
	import Icon from './Icon.svelte';
	import type { PanelState, FileEntry } from '../types';
	import { settings, lang } from '../stores.svelte';
	import { t } from '../i18n';
	import { formatBytes, formatDate, formatPerms, fileIconName } from '../format';

	let { p, index }: { p: PanelState; index: number } = $props();

	// ---- Angezeigter Eintrag: fokussierte Zeile > erste Auswahl > Eintrag 0 > nichts ----
	const shown = $derived.by((): FileEntry | null => {
		if (p.entries.length === 0) return null;
		const active = p.entries[p.activeIndex];
		if (active) return active;
		if (p.selected.length > 0) {
			return p.entries.find((e) => e.path === p.selected[0]) ?? p.entries[0];
		}
		return p.entries[0];
	});

	function kindLabel(e: FileEntry | null): string {
		if (!e) return '';
		if (e.isDir) return t('prop.typeFolder', lang());
		switch (e.kind) {
			case 'image':
				return t('prop.typeImage', lang());
			case 'video':
				return t('prop.typeVideo', lang());
			case 'audio':
				return t('prop.typeAudio', lang());
			case 'archive':
				return t('prop.typeArchive', lang());
			case 'code':
				return t('prop.typeCode', lang());
			case 'text':
				return t('prop.typeText', lang());
			default:
				return t('prop.typeFile', lang());
		}
	}

	function typeRowLabel(e: FileEntry | null): string {
		if (!e) return '–';
		if (e.isDir) return t('panes.entries', lang());
		return formatBytes(e.size);
	}
</script>

<aside class="info-pane" aria-label={t('panes.info', lang())}>
	<div class="info-head">
		<Icon name="info" size={14} />
		{t('panes.info', lang())}
	</div>

	{#if shown}
		<div class="info-main">
			<span class="info-icon {shown.isDir ? 'dir' : shown.kind}">
				<Icon name={fileIconName(shown)} size={34} />
			</span>
			<div class="info-title">{shown.name}</div>
		</div>

		<div class="info-row">
			<span>{t('prop.type', lang())}</span>
			<span>{kindLabel(shown)}</span>
		</div>

		<div class="info-row">
			<span>{t('prop.size', lang())}</span>
			<span>{typeRowLabel(shown)}</span>
		</div>

		<div class="info-row">
			<span>{t('prop.modified', lang())}</span>
			<span>{formatDate(shown.mtimeMs, lang())}</span>
		</div>

		<div class="info-row">
			<span>{t('prop.permissions', lang())}</span>
			<span style="font-family:monospace;">{formatPerms(shown.perms)}</span>
		</div>

		<div class="info-row info-path">
			<span>{t('prop.path', lang())}</span>
			<span>{shown.path}</span>
		</div>

		{#if !shown.isDir && shown.linkTarget}
			<div class="info-row info-path">
				<span>{t('prop.link', lang())}</span>
				<span>{shown.linkTarget}</span>
			</div>
		{/if}
	{:else}
		<div class="info-empty">{t('panel.empty', lang())}</div>
	{/if}
</aside>

<style>
	.info-pane {
		width: 240px;
		flex: none;
		height: 100%;
		overflow-y: auto;
		padding: 12px 14px;
		background: var(--bg-panel);
		border-left: 1px solid var(--border-strong);
		user-select: none;
	}
	.info-head {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 11px;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--text-dim);
		padding-bottom: 10px;
		margin-bottom: 6px;
		border-bottom: 1px solid var(--border-strong);
	}
	.info-main {
		display: flex;
		align-items: center;
		gap: 10px;
		margin-bottom: 12px;
	}
	.info-icon {
		flex: none;
		color: var(--accent);
	}
	.info-icon.dir {
		color: var(--info);
	}
	.info-title {
		font-size: 14px;
		font-weight: 600;
		word-break: break-all;
		line-height: 1.3;
		min-width: 0;
	}
	.info-row {
		display: flex;
		justify-content: space-between;
		gap: 10px;
		padding: 3px 0;
		font-size: 12px;
	}
	.info-row > span:first-child {
		color: var(--text-dim);
		flex: none;
	}
	.info-row > span:last-child {
		text-align: right;
		word-break: break-all;
	}
	.info-path > span:last-child {
		color: var(--text-dim);
	}
	.info-empty {
		color: var(--text-dim);
		font-size: 12px;
	}
</style>
