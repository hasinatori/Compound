<script lang="ts">
	import Icon from './Icon.svelte';
	import { ops, runCancelOp, lang } from '../stores.svelte';
	import { t } from '../i18n';

	const list = $derived(Object.values(ops).filter((o) => !o.done).sort((a, b) => a.opId.localeCompare(b.opId)));
	const pct = (o: (typeof list)[number]) =>
		o.total > 0 ? Math.round((o.current / o.total) * 100) : 0;
</script>

{#if list.length > 0}
	<div class="overlay">
		<div class="dialog" style="min-width: 460px;">
			<div class="dialog-header">
				<span>{t('tools.operations', lang())}</span>
				<span style="color:var(--text-dim); font-weight:400; font-size:12px;">
					{t('progress.running', lang(), { n: list.length })}
				</span>
			</div>
			<div class="dialog-body">
				{#each list as o (o.opId)}
					<div style="margin-bottom: 14px;">
						<div style="display:flex; justify-content:space-between; font-size:13px; margin-bottom:5px;">
							<span style="overflow:hidden; text-overflow:ellipsis; white-space:nowrap; max-width: 70%;">
								{o.label}
							</span>
							<span style="color:var(--text-dim);">{pct(o)}% · {o.current}/{o.total}</span>
						</div>
						<div class="progress-track">
							<div class="progress-fill" style="width: {pct(o)}%;"></div>
						</div>
						<div style="display:flex; justify-content:flex-end; margin-top:6px;">
							<button class="btn" onclick={() => void runCancelOp(o.opId)}>
								<Icon name="close" size={13} /> {t('action.cancel', lang())}
							</button>
						</div>
					</div>
				{/each}
			</div>
		</div>
	</div>
{/if}