<script lang="ts">
	import Icon from './Icon.svelte';
	import { resolveConflict } from '../api';
	import { t } from '../i18n';
	import { lang } from '../stores.svelte';
	import type { ConflictChoice, ConflictRequest } from '../types';

	let {
		req,
		onDone,
	}: { req: ConflictRequest | null; onDone: () => void } = $props();

	let applyToAll = $state(false);

	function srcName(r: ConflictRequest) {
		return r.src.split('/').pop() ?? r.src;
	}

	async function answer(action: ConflictChoice['action']) {
		const r = req;
		if (!r) return;
		try {
			await resolveConflict(r.opId, r.key, { action, applyToAll });
		} catch {
			// Backend-Antwort kann fehlschlagen, wenn die Op schon vorbei ist
		}
		applyToAll = false;
		onDone();
	}
</script>

{#if req}
	<div class="overlay">
		<div class="dialog" style="min-width: 480px;">
			<div class="dialog-header">
				<Icon name="warn" size={16} />&nbsp;{t('conflict.title', lang())}
				<span style="font-weight:400; font-size:12px; color:var(--text-dim);">
					{req.index + 1} / {req.total}
				</span>
			</div>
			<div class="dialog-body">
				<div style="font-size:13px;">
					<div style="padding:8px 10px; background:var(--bg-active); border-radius:6px; margin-bottom:10px;">
						<div style="color:var(--text-dim); font-size:12px;">{t('conflict.source', lang())}</div>
						<div style="word-break:break-all;">{req.src}</div>
					</div>
					<div style="text-align:center; color:var(--text-faint);">↓</div>
					<div style="padding:8px 10px; background:var(--bg-active); border-radius:6px; margin:10px 0;">
						<div style="color:var(--text-dim); font-size:12px;">{t('conflict.title', lang())}</div>
						<div style="word-break:break-all;">{req.dest}</div>
					</div>
				</div>
				<label style="display:flex; align-items:center; gap:8px; font-size:13px; cursor:pointer;">
					<input type="checkbox" bind:checked={applyToAll} />
					{t('conflict.applyToAll', lang())}
				</label>
			</div>
			<div class="dialog-footer">
				<button class="btn" onclick={() => void answer('abort')}>
					<Icon name="close" size={14} /> {t('action.cancel', lang())}
				</button>
				<button class="btn" onclick={() => void answer('skip')}>{t('conflict.skip', lang())}</button>
				<button class="btn" onclick={() => void answer('overwrite')}>{t('conflict.overwrite', lang())}</button>
				<button class="btn primary" onclick={() => void answer('keep_both')}>{t('conflict.keepBoth', lang())}</button>
			</div>
		</div>
	</div>
{/if}