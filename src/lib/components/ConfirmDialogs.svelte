<script lang="ts">
	// Подтверждения: удалить навсегда, очистить корзину.
	// Confirmations: permanent delete, empty trash.
	import Icon from './Icon.svelte';
	import {
		uiDialog,
		closeDialog,
		notify,
		refreshPanel,
		panelA,
		panelB,
		lang,
		errMsg,
	} from '../stores.svelte';
	import { t } from '../i18n';
	import { trashItems, deletePermanent, emptyTrash } from '../api';

	interface Payload {
		paths?: string[];
		kind?: 'delete' | 'trash' | 'emptyTrash';
	}

	function payloadOf(): Payload | null {
		const d = uiDialog.d;
		if (!d) return null;
		if (d.kind !== 'confirmDelete' && d.kind !== 'confirmTrash' && d.kind !== 'emptyTrash') return null;
		return (d.payload as Payload) ?? {};
	}

	const d = $derived<Payload | null>(payloadOf());

	async function confirm() {
		const payload = d;
		if (!payload) return;
		const k = uiDialog.d?.kind ?? 'emptyTrash';
		const kind = payload.kind ?? (k === 'confirmDelete' ? 'delete' : k === 'confirmTrash' ? 'trash' : 'emptyTrash');
		try {
			if (kind === 'delete' && payload.paths) {
				await deletePermanent(payload.paths);
				notify('info', t('notice.trashStarted', lang(), { n: payload.paths.length }));
			} else if (kind === 'trash' && payload.paths) {
				await trashItems(payload.paths);
				notify('ok', t('notice.trashed', lang(), { n: payload.paths.length }));
			} else if (kind === 'emptyTrash') {
				await emptyTrash();
				notify('ok', t('notice.trashEmptied', lang()));
			}
			refreshPanel(panelA);
			refreshPanel(panelB);
			closeDialog();
		} catch (e) {
			notify('error', errMsg(e));
		}
	}

	const title = $derived(
		uiDialog.d?.kind === 'confirmDelete'
			? t('confirm.deleteTitle', lang())
			: uiDialog.d?.kind === 'confirmTrash'
				? t('confirm.trashTitle', lang())
				: t('confirm.emptyTrashTitle', lang()),
	);
	const text = $derived(
		uiDialog.d?.kind === 'confirmDelete'
			? t('confirm.deleteText', lang())
			: uiDialog.d?.kind === 'confirmTrash'
				? t('confirm.trashText', lang())
				: t('confirm.emptyTrashText', lang()),
	);
</script>

{#if d}
	<div class="overlay">
		<div class="dialog" style="min-width: 420px;">
			<div class="dialog-header">
				<Icon name={uiDialog.d?.kind === 'confirmDelete' ? 'error' : 'trash'} size={16} />
				&nbsp;{title}
			</div>
			<div class="dialog-body">
				<div style="font-size:13px;">{text}</div>
				{#if d.paths && d.paths.length > 0}
					<div
						style="margin-top:10px; max-height:130px; overflow:auto; background:var(--bg-active); border-radius:6px; padding:8px; font-size:12px; color:var(--text-dim);"
					>
						{#each d.paths as p2 (p2)}
							<div style="word-break:break-all;">· {p2}</div>
						{/each}
					</div>
				{/if}
			</div>
			<div class="dialog-footer">
				<button class="btn" onclick={closeDialog}>{t('action.cancel', lang())}</button>
				<button class="btn danger" onclick={() => void confirm()}>
					{uiDialog.d?.kind === 'confirmTrash'
						? t('action.trash', lang())
						: uiDialog.d?.kind === 'confirmDelete'
							? t('action.delete', lang())
							: t('action.confirm', lang())}
				</button>
			</div>
		</div>
	</div>
{/if}