<script lang="ts">
	import Icon from './Icon.svelte';
	import { t } from '../i18n';
	import type { Place } from '../types';
	import {
		panelA,
		panelB,
		focusedPanel,
		bookmarks,
		removeBookmark,
		navigate,
		settings,
	} from '../stores.svelte';

	let places = $state<Place[]>([]);

	$effect(() => {
		void (async () => {
			try {
				const { getPlaces } = await import('../api');
				places = await getPlaces();
			} catch {
				places = [];
			}
		})();
	});

	const L = () => settings.language;
	const activePanel = () => (focusedPanel.value === 0 ? panelA : panelB);
	const active = () => activePanel().cwd;

	function go(path: string) {
		navigate(activePanel(), path);
	}
</script>

<aside class="sidebar">
	<div class="sb-section" style="flex:1; overflow-y:auto; border-bottom:none;">
		<div class="sb-heading">{t('sidebar.places', L())}</div>
		{#each places as pl (pl.path)}
			<button
				class="sb-item"
				class:active={active() === pl.path}
				title={pl.path}
				onclick={() => go(pl.path)}
			>
				{#if pl.kind === 'trash'}
					<Icon name="trash" size={15} />
				{:else if pl.kind === 'dir'}
					<Icon name="folderOpen" size={15} />
				{:else if pl.kind === 'mount'}
					<Icon name="drive" size={15} />
				{:else}
					<Icon name="home" size={15} />
				{/if}
				<span class="trunc">{pl.name}</span>
			</button>
		{/each}

		{#if bookmarks.length > 0}
			<div class="sb-heading" style="margin-top:10px;">{t('sidebar.favorites', L())}</div>
			{#each [...bookmarks].sort((a, b) => a.localeCompare(b)) as bm (bm)}
				<button class="sb-item" class:active={active() === bm} title={bm} onclick={() => go(bm)}>
					<Icon name="star" size={15} />
					<span class="sb-fav">
						<span class="sb-fav-name trunc">{bm.split('/').filter(Boolean).pop() ?? bm}</span>
						<span class="sb-fav-path trunc">{bm}</span>
					</span>
					<span
						style="margin-left:auto; opacity:0.6; display:inline-flex; flex:none;"
						role="button"
						tabindex="0"
						aria-label={`Entfernen: ${bm}`}
						onclick={(e) => {
							e.stopPropagation();
							removeBookmark(bm);
						}}
						onkeydown={(e) => {
							if (e.key === 'Enter' || e.key === ' ') {
								e.stopPropagation();
								e.preventDefault();
								removeBookmark(bm);
							}
						}}
					>
						<Icon name="close" size={12} />
					</span>
				</button>
			{/each}
		{/if}
	</div>
</aside>