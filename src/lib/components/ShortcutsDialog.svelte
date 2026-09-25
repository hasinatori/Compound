<script lang="ts">
	import Icon from './Icon.svelte';
	import { uiDialog, closeDialog, settings, lang } from '../stores.svelte';
	import { t } from '../i18n';

	const d = $derived(uiDialog.d?.kind === 'shortcuts' ? true : false);
	const L = () => settings.language;

	// Beschreibungen werden aus dem i18n-Wörterbuch geladen,
	// damit der Dialog mit der gewählten Sprache wechselt.
	const sections = $derived([
		{
			title: t('shortcuts.browser', L()),
			rows: [
				{ keys: ['F5', 'Ctrl+R'], label: t('action.refresh', L()) },
				{ keys: ['Ctrl+F'], label: t('toolbar.search', L()) },
				{ keys: ['Ctrl+N'], label: t('ctx.newFile', L()) },
				{ keys: ['Ctrl+C', 'Ctrl+X', 'Ctrl+V'], label: t('shortcuts.clipboard', L()) },
				{ keys: ['F2'], label: t('action.rename', L()) },
				{ keys: ['Entf'], label: t('ctx.trash', L()) },
				{ keys: ['Ctrl+D'], label: t('ctx.delete', L()) },
				{ keys: ['Enter'], label: t('ctx.open', L()) },
				{ keys: ['↑', '↓', 'Pos1', 'Ende'], label: t('shortcuts.navigate', L()) },
				{ keys: ['Alt+←', 'Alt+→', 'Alt+↑'], label: t('shortcuts.navdir', L()) },
				{ keys: ['Tab'], label: t('shortcuts.switchPanel', L()) },
				{ keys: ['Esc'], label: t('shortcuts.close', L()) },
			],
		},
		{
			title: t('shortcuts.editor', L()),
			rows: [
				{ keys: ['Ctrl+S'], label: t('editor.save', L()) },
				{ keys: ['Ctrl+F'], label: t('toolbar.search', L()) },
				{ keys: ['Ctrl+Z', 'Ctrl+Y'], label: t('shortcuts.undoRedo', L()) },
				{ keys: ['Tab', 'Shift+Tab'], label: t('shortcuts.indent', L()) },
				{ keys: ['Esc'], label: t('shortcuts.close', L()) },
			],
		},
		{
			title: t('shortcuts.dialogs', L()),
			rows: [
				{ keys: ['Enter'], label: t('shortcuts.confirm', L()) },
				{ keys: ['Esc'], label: t('shortcuts.close', L()) },
			],
		},
	]);
</script>

{#if d}
	<div class="overlay">
		<div class="dialog" style="width: 620px; max-height: 80vh; display:flex; flex-direction:column;">
			<div class="dialog-header">
				<Icon name="key" size={16} />&nbsp;{t('shortcuts.title', lang())}
				<button class="toolbar-btn" onclick={closeDialog}><Icon name="close" size={15} /></button>
			</div>
			<div class="dialog-body" style="overflow-y:auto;">
				{#each sections as sec}
					<div class="sc-head">{sec.title}</div>
					<table class="sc-table">
						<tbody>
							{#each sec.rows as row}
								<tr>
									<td class="sc-keys">
										{#each row.keys as k}
											<kbd>{k}</kbd>
										{/each}
									</td>
									<td class="sc-label">{row.label}</td>
								</tr>
							{/each}
						</tbody>
					</table>
				{/each}
				<div style="font-size:11px; color:var(--text-faint); margin-top:12px;">
					{settings.language === 'de'
						? 'Hinweis: Ctrl = Strg auf deutschen Tastaturen.'
						: 'Note: Ctrl = Strg on German keyboards.'}
				</div>
			</div>
			<div class="dialog-footer">
				<button class="btn" onclick={closeDialog}>{t('action.close', lang())}</button>
			</div>
		</div>
	</div>
{/if}