<script lang="ts">
	import Icon from './Icon.svelte';
	import { uiDialog, closeDialog, notify, lang, errMsg } from '../stores.svelte';
	import { t } from '../i18n';
	import { createItem } from '../api';
	import type { PanelState } from '../types';

	interface Payload {
		destDir: string;
		isDir?: boolean;
	}

	const d = $derived(uiDialog.d?.kind === 'newitem' ? (uiDialog.d.payload as Payload) : null);
	let name = $state('');
	let isDir = $state(false);
	let nameInput = $state<HTMLInputElement | null>(null);
	$effect(() => {
		if (d) isDir = d.isDir ?? false;
	});
	$effect(() => {
		if (d) nameInput?.focus();
	});

	async function submit() {
		const target = d;
		if (!target) return;
		if (!name.trim()) return;
		try {
			await createItem(target.destDir, name.trim(), isDir);
			notify('ok', t('notice.created', lang(), { name: name.trim() }));
			closeDialog();
		} catch (e) {
			notify('error', errMsg(e));
		}
	}
</script>

{#if d}
	<div class="overlay">
		<div class="dialog">
			<div class="dialog-header">
				{isDir ? t('action.newFolder', lang()) : t('action.newFile', lang())}
				<button class="toolbar-btn" onclick={closeDialog}><Icon name="close" size={15} /></button>
			</div>
			<div class="dialog-body">
				<div class="form-row">
					<label for="newitem-name">{t('prop.name', lang())}</label>
					<input id="newitem-name" bind:this={nameInput} bind:value={name} onkeydown={(e) => e.key === 'Enter' && void submit()} />
				</div>
				<label style="display:flex; align-items:center; gap:8px; font-size:13px; cursor:pointer;">
					<input type="checkbox" bind:checked={isDir} />
					{t('action.newFolder', lang())}
				</label>
				<div class="form-hint" style="margin-top:10px;">{t('prop.path', lang())}: {d.destDir}</div>
			</div>
			<div class="dialog-footer">
				<button class="btn" onclick={closeDialog}>{t('action.cancel', lang())}</button>
				<button class="btn primary" onclick={() => void submit()}>{t('action.create', lang())}</button>
			</div>
		</div>
	</div>
{/if}