<script lang="ts">
	// Просмотр/правка текста (CodeMirror 6) с подсветкой и защитой от гонок.
	// Text view/edit (CodeMirror 6) with highlighting and race protection.
	// save_text отдаёт expected_mtime — если файл меняли, получаем ошибку.
	// save_text takes expected_mtime — external edits make the save fail.
	import Icon from './Icon.svelte';
	import {
		editorPath,
		panelA,
		panelB,
		focusedPanel,
		settings,
		notify,
		setView,
		setEditorPath,
		lang,
		errMsg,
	} from '../stores.svelte';
	import { t } from '../i18n';
	import { readText, saveText } from '../api';
	import type { EditorRead, SaveResult } from '../types';
	import type { Extension } from '@codemirror/state';

	let text = $state('');
	let mtime = $state(0);
	let status = $state<{ icon: string; msg: string; color?: string } | null>(null);
	let binary = $state(false);
	let readonly = $state(false);
	let truncated = $state(false);
	let dirty = $state(false);

	let host = $state<HTMLDivElement | null>(null);
	let view = $state<import('@codemirror/view').EditorView | null>(null);

	type CMState = typeof import('@codemirror/state').EditorState;

	interface Mounted {
		state: CMState;
		view: import('@codemirror/view').EditorView;
	}

	async function langFor(path: string, firstLine: string): Promise<Extension[]> {
		// Legacy-режимы (StreamLanguage) для языков без официального пакета CM6.
		// Legacy modes (StreamLanguage) for languages without an official CM6 package
		const shellLang = async (): Promise<Extension> => {
			const { StreamLanguage } = await import('@codemirror/language');
			const { shell } = await import('@codemirror/legacy-modes/mode/shell');
			return StreamLanguage.define(shell);
		};
		const yamlLang = async (): Promise<Extension> => {
			const { StreamLanguage } = await import('@codemirror/language');
			const { yaml } = await import('@codemirror/legacy-modes/mode/yaml');
			return StreamLanguage.define(yaml);
		};
		const tomlLang = async (): Promise<Extension> => {
			const { StreamLanguage } = await import('@codemirror/language');
			const { toml } = await import('@codemirror/legacy-modes/mode/toml');
			return StreamLanguage.define(toml);
		};
		const propsLang = async (): Promise<Extension> => {
			const { StreamLanguage } = await import('@codemirror/language');
			const { properties } = await import('@codemirror/legacy-modes/mode/properties');
			return StreamLanguage.define(properties);
		};
		const dockerLang = async (): Promise<Extension> => {
			const { StreamLanguage } = await import('@codemirror/language');
			const { dockerFile } = await import('@codemirror/legacy-modes/mode/dockerfile');
			return StreamLanguage.define(dockerFile);
		};
		const cmakeLang = async (): Promise<Extension> => {
			const { StreamLanguage } = await import('@codemirror/language');
			const { cmake } = await import('@codemirror/legacy-modes/mode/cmake');
			return StreamLanguage.define(cmake);
		};

		const map: Array<[RegExp, () => Promise<Extension>]> = [
			[/\.(rs)$/, async () => (await import('@codemirror/lang-rust')).rust()],
			[/\.(py|pyw)$/, async () => (await import('@codemirror/lang-python')).python()],
			[/\.(tsx|jsx)$/, async () => (await import('@codemirror/lang-javascript')).javascript({ typescript: true, jsx: true })],
			[/\.(ts|mts|cts)$/, async () => (await import('@codemirror/lang-javascript')).javascript({ typescript: true })],
			[/\.(m?js|cjs)$/, async () => (await import('@codemirror/lang-javascript')).javascript()],
			[/\.(json|jsonc)$/, async () => (await import('@codemirror/lang-json')).json()],
			[/\.(md|markdown)$/, async () => (await import('@codemirror/lang-markdown')).markdown()],
			[/\.(css|scss|sass|less)$/, async () => (await import('@codemirror/lang-css')).css()],
			[/\.(html?|xml|svg|xsd)$/, async () => (await import('@codemirror/lang-html')).html()],
			[/\.(sql)$/, async () => (await import('@codemirror/lang-sql')).sql()],
			[/\.(c|h|cc|cpp|hpp|cxx|hxx)$/, async () => (await import('@codemirror/lang-cpp')).cpp()],
			[/\.go$/, async () => (await import('@codemirror/lang-go')).go()],
			[/\.java$/, async () => (await import('@codemirror/lang-java')).java()],
			[/\.(php|phtml)$/, async () => (await import('@codemirror/lang-php')).php()],
			[/\.(sh|bash|zsh|ksh)$/, shellLang],
			[/\.ya?ml$/, yamlLang],
			[/\.toml$/, tomlLang],
			[/\.(ini|cfg|conf|properties)$/, propsLang],
			[/(^|\/)(Dockerfile|Containerfile)(\.|$)/, dockerLang],
			[/(^|\/)CMakeLists\.txt$/, cmakeLang],
		];
		for (const [re, fn] of map) {
			if (re.test(path)) {
				try {
					return [await fn()];
				} catch {
					return [];
				}
			}
		}
		// Определяем shebang у скриптов без подходящего расширения.
		// Shebang detection for scripts without a matching extension
		try {
			if (/^#!.*\b(bash|zsh|ksh|sh)\b/.test(firstLine)) return [await shellLang()];
			if (/^#!.*python/.test(firstLine)) return [(await import('@codemirror/lang-python')).python()];
		} catch {
			return [];
		}
		return [];
	}

	async function mountEditor(doc: string, path: string) {
		if (!host) return;
		const { EditorState } = await import('@codemirror/state');
		const {
			EditorView,
			keymap,
			lineNumbers,
			highlightActiveLine,
			highlightActiveLineGutter,
			drawSelection,
			dropCursor,
			rectangularSelection,
			crosshairCursor,
		} = await import('@codemirror/view');
		const { defaultKeymap, history, historyKeymap, indentWithTab } = await import('@codemirror/commands');
		const { searchKeymap, highlightSelectionMatches } = await import('@codemirror/search');
		const { syntaxHighlighting, defaultHighlightStyle, bracketMatching, indentOnInput, foldGutter, foldKeymap } =
			await import('@codemirror/language');

		const langs = await langFor(path, doc.split('\n')[0] ?? '');
		const state = EditorState.create({
			doc,
			extensions: [
				lineNumbers(),
				highlightActiveLineGutter(),
				highlightActiveLine(),
				drawSelection(),
				dropCursor(),
				rectangularSelection(),
				crosshairCursor(),
				foldGutter(),
				bracketMatching(),
				indentOnInput(),
				syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
				highlightSelectionMatches(),
				history(),
				keymap.of([{ key: 'Mod-s', run: () => { void save(); return true; } }, ...defaultKeymap, ...searchKeymap, ...historyKeymap, ...foldKeymap, indentWithTab]),
				EditorView.updateListener.of((u) => {
					if (u.docChanged) dirty = true;
				}),
				readonly || binary ? EditorState.readOnly.of(true) : [],
				EditorView.theme({
					'&': { color: '#e6e9ef', backgroundColor: '#0f1117', height: '100%' },
					'.cm-content': { fontFamily: "'JetBrains Mono','Fira Code',monospace", fontSize: '13px', lineHeight: '1.55', padding: '8px 0' },
					'.cm-gutters': { backgroundColor: '#0f1117', color: '#4b5563', borderRight: '1px solid #1c1f26' },
					'.cm-activeLine': { backgroundColor: 'rgba(255,255,255,0.04)' },
					'.cm-activeLineGutter': { backgroundColor: 'rgba(255,255,255,0.06)', color: '#9aa4b2' },
					'.cm-cursor': { borderLeft: '2px solid #4f9cf7' },
					'.cm-selectionBackground, &.cm-focused .cm-selectionBackground': { backgroundColor: 'rgba(79,156,247,0.28)' },
					'.cm-matchingBracket': { backgroundColor: 'rgba(87,201,106,0.2)', outline: '1px solid var(--ok)' },
					'.cm-foldPlaceholder': { backgroundColor: 'transparent', border: 'none', color: '#4b5563' },
					'.cm-panels': { backgroundColor: '#16181d', color: '#e6e9ef' },
					'.cm-searchMatch': { backgroundColor: 'rgba(229,181,103,0.25)' },
					'.cm-searchMatch.selected': { backgroundColor: 'rgba(229,181,103,0.5)' },
				}),
				...langs,
			],
		});
		view = new EditorView({ state, parent: host });
	}

	async function load(p: string) {
		status = null;
		binary = false;
		dirty = false;
		view?.destroy();
		view = null;
		try {
			const r: EditorRead = await readText(p);
			mtime = r.mtimeMs;
			binary = r.binary;
			readonly = r.readonly || r.binary;
			truncated = r.truncated;
			text = r.text;
			if (!r.binary) {
				await mountEditor(r.text, p);
			}
			status = {
				icon: r.binary ? 'error' : 'check',
				msg: r.binary
					? 'Binärdatei – nicht bearbeitbar.'
					: `UTF-8${r.converted ? ' → Latin-1' : ''} · ${Math.round(r.text.length / 1024)} KB${r.truncated ? ' · abgeschnitten' : ''}`,
				color: r.binary ? 'var(--err)' : 'var(--text-dim)',
			};
		} catch (e) {
			status = { icon: 'error', msg: errMsg(e), color: 'var(--err)' };
		}
	}

	async function save() {
		if (!editorPath.value || !view || readonly || binary) return;
		try {
			const res: SaveResult = await saveText(editorPath.value, view.state.doc.toString(), mtime);
			if (res.saved) {
				mtime = res.mtimeMs;
				dirty = false;
				status = { icon: 'check', msg: 'Gespeichert.', color: 'var(--ok)' };
			} else if (res.changed) {
				const { confirm } = await import('@tauri-apps/plugin-dialog');
				const ok = await confirm('Datei wurde extern geändert.', {
					title: 'Konflikt',
					okLabel: 'Überschreiben',
					cancelLabel: 'Abbrechen',
					kind: 'warning',
				});
				if (ok) {
					const retry: SaveResult = await saveText(editorPath.value, view.state.doc.toString(), mtime);
					if (retry.saved) {
						mtime = retry.mtimeMs;
						dirty = false;
						status = { icon: 'check', msg: 'Gespeichert.', color: 'var(--ok)' };
					} else {
						status = { icon: 'error', msg: retry.error ?? 'Speichern fehlgeschlagen.', color: 'var(--err)' };
					}
				}
			} else {
				status = { icon: 'error', msg: res.error ?? 'Speichern fehlgeschlagen.', color: 'var(--err)' };
			}
		} catch (e) {
			status = { icon: 'error', msg: errMsg(e), color: 'var(--err)' };
		}
	}

	function back() {
		setView('browser');
		setEditorPath(null);
		view?.destroy();
		view = null;
	}

	// Путь: активная выборка в панели в фокусе (или один путь).
	// Path: the active selection in the focused panel (or a single path)
	function pickFromPanel() {
		const p = focusedPanel.value === 0 ? panelA : panelB;
		const sel = p.selected.filter((s) => !p.entries.find((e) => e.path === s)?.isDir);
		if (sel.length === 1) {
			setEditorPath(sel[0]);
			void load(sel[0]);
		} else {
			notify('info', t('notice.selectOneFile', lang()));
		}
	}

	// При открытии берём текущую выборку.
	// On open: use the current selection
	if (!editorPath.value) pickFromPanel();

	// Монтируем редактор, когда появится host-DOM (может быть позже load()).
	// Mount the editor once the host DOM exists (may happen after load())
	$effect(() => {
		if (host && editorPath.value && !view && !binary && status) {
			void mountEditor(text, editorPath.value);
		}
	});
</script>

<div class="editor-host" style="flex:1; min-height:0;">
	<div
		style="display:flex; align-items:center; gap:8px; padding:8px 12px; background:var(--bg-elev); border-bottom:1px solid var(--border); flex:none;"
	>
		<button class="toolbar-btn" onclick={back}><Icon name="back" size={15} /> {t('toolbar.browser', lang())}</button>
		<button class="toolbar-btn" onclick={pickFromPanel} title={t('editor.pickFromSelection', lang())}>
			<Icon name="file" size={14} />
		</button>
		<span style="flex:1; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; font-size:13px;" title={editorPath.value ?? ''}>
			{editorPath.value}
		</span>
		{#if status}
			<span style="display:inline-flex; gap:6px; align-items:center; font-size:12px; color:{status.color ?? 'var(--text-dim)'};">
				<Icon name={status.icon} size={13} /> {status.msg}
			</span>
		{/if}
		{#if dirty}
			<span style="font-size:12px; color:var(--warn);">{t('editor.changed', lang())}</span>
		{/if}
		<button
			class="btn primary"
			onclick={() => void save()}
			disabled={readonly || binary || !view}
			title="Ctrl+S"
		>
			<Icon name="check" size={14} /> {t('editor.save', lang())}
		</button>
	</div>
	<div bind:this={host} style="flex:1; min-height:0; overflow:auto; background:#0f1117;"></div>
</div>