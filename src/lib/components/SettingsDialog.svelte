<script lang="ts">
	// Настройки: язык, панели, сортировка, подтверждения. Пишет в localStorage.
	// Settings: language, panels, sorting, confirmations. Stored in localStorage.
	import Icon from './Icon.svelte';
	import { uiDialog, closeDialog, settings, saveSettings, lang } from '../stores.svelte';
	import { t } from '../i18n';
	import { getAbout } from '../api';
	import type { AboutInfo } from '../types';

	const d = $derived(uiDialog.d?.kind === 'settings' ? true : false);
	let about = $state<AboutInfo | null>(null);
	let language = $state(settings.language);
	let singlePanel = $state(settings.singlePanel);
	let confirmDelete = $state(settings.confirmDelete);
	let showHidden = $state(settings.showHidden);
	let sortBy = $state(settings.sortBy);

	$effect(() => {
		void (async () => {
			try {
				about = await getAbout();
			} catch {
				about = null;
			}
		})();
	});

	function apply() {
		settings.language = language;
		settings.singlePanel = singlePanel;
		settings.confirmDelete = confirmDelete;
		settings.showHidden = showHidden;
		settings.sortBy = sortBy as typeof settings.sortBy;
		saveSettings(settings);
		void (async () => {
			const { refreshPanel, panelA, panelB } = await import('../stores.svelte');
			refreshPanel(panelA);
			refreshPanel(panelB);
		})();
		closeDialog();
	}
</script>

{#if d}
	<div class="overlay">
		<div class="dialog" style="width: 560px;">
			<div class="dialog-header">
				<Icon name="settings" size={16} />&nbsp;{t('settings.title', lang())}
				<button class="toolbar-btn" onclick={closeDialog}><Icon name="close" size={15} /></button>
			</div>
			<div class="dialog-body">
				<div class="form-row">
					<label for="settings-language">{t('settings.language', lang())}</label>
					<select id="settings-language" bind:value={language}>
						<option value="de">Deutsch</option>
						<option value="en">English</option>
					</select>
				</div>
				<div class="form-row">
					<label for="settings-sort">{t('settings.sort', lang())}</label>
					<select id="settings-sort" bind:value={sortBy}>
						<option value="name">{t('prop.name', lang())}</option>
						<option value="ext">{t('sort.ext', lang())}</option>
						<option value="size">{t('prop.size', lang())}</option>
						<option value="mtime">{t('sort.date', lang())}</option>
					</select>
				</div>
				<label style="display:flex; gap:8px; align-items:center; font-size:13px; margin-bottom:10px; cursor:pointer;">
					<input type="checkbox" bind:checked={singlePanel} /> {t('settings.singlePanel', lang())}
				</label>
				<label style="display:flex; gap:8px; align-items:center; font-size:13px; margin-bottom:10px; cursor:pointer;">
					<input type="checkbox" bind:checked={confirmDelete} /> {t('settings.confirmDelete', lang())}
				</label>
				<label style="display:flex; gap:8px; align-items:center; font-size:13px; margin-bottom:10px; cursor:pointer;">
					<input type="checkbox" bind:checked={showHidden} /> {t('settings.showHidden', lang())}
				</label>
				<hr style="border:none; border-top:1px solid var(--border); margin:14px 0;" />
				{#if about}
					<div style="font-size:12px; color:var(--text-dim); display:flex; flex-direction:column; gap:3px;">
						<span>{about.name} v{about.version}</span>
						<span>{about.identifier}</span>
					</div>
				{/if}
			</div>
			<div class="dialog-footer">
				<button class="btn" onclick={closeDialog}>{t('action.cancel', lang())}</button>
				<button class="btn primary" onclick={apply}>{t('action.apply', lang())}</button>
			</div>
		</div>
	</div>
{/if}