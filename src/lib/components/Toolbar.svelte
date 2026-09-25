<script lang="ts">
	// Верхняя панель: кнопки действий + переключатель вида. Только UI, логика в App.
	// Top toolbar: action buttons + view switcher. UI only, logic lives in App.
	import Icon from './Icon.svelte';
import {
	view,
	settings,
	openDialog,
	panelA,
	panelB,
	focusedPanel,
	setEditorPath,
	setView,
	navigate,
	setTerminalCwd,
} from '../stores.svelte';
	import { t } from '../i18n';

	function togglePanels() {
		settings.singlePanel = !settings.singlePanel;
	}

	function openEditor() {
		const p = focusedPanel.value === 0 ? panelA : panelB;
		const fileSel = p.selected.filter((s) => !p.entries.find((e) => e.path === s)?.isDir);
		if (fileSel.length === 1 && fileSel[0]) {
			setEditorPath(fileSel[0]);
			setView('editor');
		} else {
			setEditorPath(null);
			setView('editor');
		}
	}

	function openTerminal() {
		const p = focusedPanel.value === 0 ? panelA : panelB;
		setTerminalCwd(p.cwd);
		setView('terminal');
	}

	function goTrash() {
		const p = focusedPanel.value === 0 ? panelA : panelB;
		navigate(p, 'trash://');
		setView('browser');
	}

	const L = () => settings.language;
</script>

<header
	style="display:flex; align-items:center; gap:6px; padding:0 12px; height:44px; background:var(--bg-elev); border-bottom:1px solid var(--border); flex:none;"
>
	<span style="display:inline-flex; align-items:center; gap:8px; font-weight:700; font-size:15px; margin-right:8px;">
		<Icon name="folderOpen" size={18} color="var(--accent)" />
		Compound
		<span style="color:var(--text-faint); font-size:11px; font-weight:400;">1.0.0</span>
	</span>

	<button
		class="toolbar-btn"
		class:active={view.value === 'browser'}
		onclick={() => setView('browser')}
	>
		<Icon name="folder" size={15} /> {t('toolbar.browser', L())}
	</button>

	<div class="sep-v"></div>

	<button
		class="toolbar-btn"
		onclick={() => openDialog({ kind: 'search' })}
		title="Strg+F"
	>
		<Icon name="search" size={15} /> {t('toolbar.search', L())}
	</button>
	<button
		class="toolbar-btn"
		class:active={view.value === 'editor'}
		onclick={openEditor}
		title="Aktive Auswahl öffnen"
	>
		<Icon name="edit" size={15} /> {t('toolbar.editor', L())}
	</button>
	<button
		class="toolbar-btn"
		class:active={view.value === 'terminal'}
		onclick={openTerminal}
	>
		<Icon name="terminal" size={15} /> {t('toolbar.terminal', L())}
	</button>
	<button
		class="toolbar-btn"
		onclick={() => openDialog({ kind: 'tools' })}
	>
		<Icon name="tools" size={15} /> {t('toolbar.tools', L())}
	</button>

	<div class="sep-v"></div>

	<button class="toolbar-btn" class:active={!settings.singlePanel} onclick={togglePanels} title={t('toolbar.dual', L())}>
		<Icon name="dual" size={15} /> {t('toolbar.dual', L())}
	</button>
	<button class="toolbar-btn" class:active={settings.singlePanel} onclick={togglePanels} title={t('toolbar.single', L())}>
		<Icon name="single" size={15} /> {t('toolbar.single', L())}
	</button>

	<div class="sep-v"></div>

	<button class="toolbar-btn" onclick={goTrash} title={t('sidebar.trash', L())}>
		<Icon name="trash" size={15} />
	</button>
	<button class="toolbar-btn" onclick={() => openDialog({ kind: 'settings' })} title={t('toolbar.settings', L())}>
		<Icon name="settings" size={15} />
	</button>
	<button class="toolbar-btn" onclick={() => openDialog({ kind: 'shortcuts' })} title={t('shortcuts.title', L())}>
		<Icon name="key" size={15} />
	</button>

	<div style="flex:1;"></div>

	<button
		class="toolbar-btn"
		onclick={() => {
			const p = focusedPanel.value === 0 ? panelA : panelB;
			setTimeout(() => navigate(p, p.cwd), 0);
		}}
		title={t('action.refresh', L())}
	>
		<Icon name="refresh" size={15} />
	</button>
</header>