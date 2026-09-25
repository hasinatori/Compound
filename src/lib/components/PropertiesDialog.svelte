<script lang="ts">
	// Свойства файла: размер, даты, права, тип. Только чтение.
	// File properties: size, dates, permissions, type. Read-only.
	import Icon from './Icon.svelte';
	import { uiDialog, closeDialog, settings, lang } from '../stores.svelte';
	import { t } from '../i18n';
	import type { FileEntry } from '../types';
	import { formatBytes, formatDate, formatPerms } from '../format';

	interface Payload {
		entry: FileEntry;
	}

	const d = $derived<Payload | null>(
		uiDialog.d?.kind === 'properties' ? (uiDialog.d.payload as Payload) : null,
	);

	const kindLabel = $derived(
		(e: FileEntry | null) => {
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
		},
	);
</script>

{#if d}
	<div class="overlay">
		<div class="dialog">
			<div class="dialog-header">
				<Icon name="info" size={16} />&nbsp;{t('action.properties', lang())}
				<button class="toolbar-btn" onclick={closeDialog}><Icon name="close" size={15} /></button>
			</div>
			<div class="dialog-body">
				{#if d.entry}
					<div style="display:flex; gap:14px; align-items:flex-start; margin-bottom:14px;">
						<Icon
							name={d.entry.isDir ? 'folder' : d.entry.kind === 'image' ? 'image' : 'file'}
							size={40}
							color={d.entry.isDir ? 'var(--info)' : 'var(--text-dim)'}
						/>
						<div>
							<div style="font-size:15px; font-weight:600; word-break:break-all;">{d.entry.name}</div>
							<div style="font-size:12px; color:var(--text-dim);">{kindLabel(d.entry)}</div>
						</div>
					</div>
					<div class="form-row">
						<div>{t('prop.path', lang())}</div>
						<div style="font-size:13px; word-break:break-all; user-select:text;">{d.entry.path}</div>
					</div>
					<div class="form-row">
						<div>{t('prop.size', lang())}</div>
						<div style="font-size:13px;">{d.entry.isDir ? '–' : formatBytes(d.entry.size)}</div>
					</div>
					<div class="form-row">
						<div>{t('prop.modified', lang())}</div>
						<div style="font-size:13px;">{formatDate(d.entry.mtimeMs, settings.language)}</div>
					</div>
					<div class="form-row">
						<div>{t('prop.permissions', lang())}</div>
						<div style="font-size:13px; font-family: monospace;">{formatPerms(d.entry.perms)}</div>
					</div>
					{#if d.entry.linkTarget}
						<div class="form-row">
							<div>{t('prop.link', lang())}</div>
							<div style="font-size:13px; word-break:break-all;">{d.entry.linkTarget}</div>
						</div>
					{/if}
				{/if}
			</div>
			<div class="dialog-footer">
				<button class="btn" onclick={closeDialog}>{t('action.close', lang())}</button>
			</div>
		</div>
	</div>
{/if}