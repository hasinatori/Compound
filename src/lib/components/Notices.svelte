<script lang="ts">
	// Тосты уведомлений поверх всего. Живут в notices-сторе, гаснут по таймеру.
	// Notice toasts on top. Live in the notices store, auto-expire on a timer.
	import Icon from './Icon.svelte';
	import { notices } from '../stores.svelte';

	const icons: Record<string, string> = { info: 'info', ok: 'check', warn: 'warn', error: 'error' };
</script>

<div class="notices">
	{#each notices as n (n.id)}
		<div class="notice {n.kind}">
			<Icon name={icons[n.kind]} size={15} />
			<span class="nl">{n.text}</span>
			<button
				class="toolbar-btn"
				onclick={() => {
					const i = notices.findIndex((x) => x.id === n.id);
					if (i >= 0) notices.splice(i, 1);
				}}
			>
				<Icon name="close" size={13} />
			</button>
		</div>
	{/each}
</div>