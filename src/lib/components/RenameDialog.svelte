<script lang="ts">
	import Icon from './Icon.svelte';
	import { uiDialog, closeDialog, notify, lang, errMsg } from '../stores.svelte';
	import { t } from '../i18n';
	import { renameItem } from '../api';

	interface Payload {
		path: string;
		canonical: boolean;
	}

	const d = $derived(uiDialog.d?.kind === 'rename' ? (uiDialog.d.payload as Payload) : null);
	let name = $state('');
	let nameInput = $state<HTMLInputElement | null>(null);
	$effect(() => {
		if (d) name = d.path.split('/').pop() ?? '';
	});
	$effect(() => {
		if (d) nameInput?.focus();
	});

	async function submit() {
		const target = d;
		if (!target) return;
		if (!name.trim()) return;
		if (name.trim() === (target.path.split('/').pop() ?? '')) {
			closeDialog();
			return;
		}
		try {
			await renameItem(target.path, name.trim());
			notify('ok', t('notice.renamed', lang(), { name: name.trim() }));
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
				{t('action.rename', lang())}
				<button class="toolbar-btn" onclick={closeDialog}><Icon name="close" size={15} /></button>
			</div>
			<div class="dialog-body">
				<div class="form-row">
					<label for="rename-name">{t('rename.newname', lang())}</label>
					<input id="rename-name" bind:this={nameInput} bind:value={name} onkeydown={(e) => e.key === 'Enter' && void submit()} />
				</div>
				<div class="form-hint" style="margin-top:10px;">{t('prop.path', lang())}: {d.path}</div>
			</div>
			<div class="dialog-footer">
				<button class="btn" onclick={closeDialog}>{t('action.cancel', lang())}</button>
				<button class="btn primary" onclick={() => void submit()}>{t('action.rename', lang())}</button>
			</div>
		</div>
	</div>
{/if}