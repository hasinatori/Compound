<script lang="ts">
	import Icon from './Icon.svelte';
	import { t } from '../i18n';
	import type { PanelState } from '../types';
	import { pathParts } from '../api';
	import { navigate, settings } from '../stores.svelte';

	let { p }: { p: PanelState } = $props();

	let parts = $state<{ name: string; path: string }[]>([]);

	$effect(() => {
		void (async () => {
			try {
				parts = await pathParts(p.cwd);
			} catch {
				parts = [];
			}
		})();
	});

	const L = () => settings.language;

	function go(part: { name: string; path: string }) {
		navigate(p, part.path);
	}
</script>

{#if p.cwd.startsWith('trash:')}
	<div class="breadcrumb">
		<Icon name="trash" size={14} />
		<span class="bc-item current">{t('sidebar.trash', L())}</span>
	</div>
{:else}
	<div class="breadcrumb">
		{#each parts as part, i (part.path)}
			{#if i > 0}
				<span style="color:var(--text-faint)">/</span>
			{/if}
			<button
				class="bc-item"
				class:current={i === parts.length - 1}
				onclick={() => go(part)}
				title={part.path}
			>
				{part.name}
			</button>
		{/each}
	</div>
{/if}