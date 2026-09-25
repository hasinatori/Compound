<script lang="ts">
	// Статусбар внизу: выбор, размер, свободно на диске. Только чтение стора.
	// Bottom status bar: selection, size, free space. Read-only from the store.
	import Icon from './Icon.svelte';
	import { t } from '../i18n';
	import { settings, panelA, panelB, focusedPanel, ops } from '../stores.svelte';
	import { formatBytes } from '../format';
	import { diskUsage } from '../api';

	interface DiskState {
		total: number;
		free: number;
		path: string;
	}

	let disk = $state<DiskState | null>(null);

	$effect(() => {
		const p = focusedPanel.value === 0 ? panelA : panelB;
		const cwd = p.cwd;
		if (cwd.startsWith('trash:')) return;
		void (async () => {
			try {
				const d = await diskUsage(cwd);
				disk = { ...d, path: cwd };
			} catch {
				disk = null;
			}
		})();
	});

	const L = () => settings.language;
	const p = $derived(focusedPanel.value === 0 ? panelA : panelB);
	const running = $derived(Object.values(ops).filter((o) => !o.done).length);
	const diskFree = $derived(disk ? formatBytes(disk.free) : '–');
</script>

<footer class="statusbar">
	<span title={t('status.cwd', L())}>
		<Icon name="folder" size={13} />&nbsp;<span style="max-width:380px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; display:inline-block; vertical-align:bottom;">{p.cwd}</span>
	</span>
	<span>
		{t('panel.items', L(), {
			n: p.entries.length,
			size: formatBytes(p.entries.reduce((a, e) => a + (e.isDir ? 0 : e.size), 0)),
		})}
	</span>
	<span class="right">
		{#if disk}
			<span title={disk.path}>
				<Icon name="drive" size={13} />&nbsp;{t('status.disk', L())}: {diskFree}
			</span>
		{/if}
		{#if running > 0}
			<span style="color: var(--accent);">
				<Icon name="count" size={13} />&nbsp;{running}
			</span>
		{/if}
		<span style="color: var(--text-faint);">Compound 1.0.0</span>
	</span>
</footer>