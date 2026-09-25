<script lang="ts">
	import Icon from './Icon.svelte';
	import { setView, terminalStartCwd } from '../stores.svelte';
	import { terminalOpen, terminalWrite, terminalResize, terminalClose } from '../api';
	import { EOF_MARKER } from './TerminalConst';

	let host = $state<HTMLDivElement | null>(null);
	let term: import('@xterm/xterm').Terminal | null = null;
	let fitAddon: import('@xterm/addon-fit').FitAddon | null = null;
	let sessionId: string | null = null;
	let closed = $state(false);
	let cwd = $state(terminalStartCwd.value);

	async function open(cwd_: string) {
		await import('@xterm/xterm/css/xterm.css');
		const { Terminal } = await import('@xterm/xterm');
		const { FitAddon } = await import('@xterm/addon-fit');
		if (!host) return;

		if (term) {
			term.dispose();
			term = null;
		}
		if (sessionId) {
			await terminalClose(sessionId);
			sessionId = null;
		}

		closed = false;
		const t = new Terminal({
			convertEol: true,
			fontFamily: "'JetBrains Mono','Fira Code',Consolas,monospace",
			fontSize: 13,
			cursorBlink: true,
			theme: {
				background: '#0d1117',
				foreground: '#e6e9ef',
				cursor: '#4f9cf7',
				cursorAccent: '#0b1220',
				selectionBackground: 'rgba(79,156,247,0.35)',
				black: '#0d1117',
				red: '#e06c75',
				green: '#57c96a',
				yellow: '#e5b567',
				blue: '#4f9cf7',
				magenta: '#c678dd',
				cyan: '#56b6c2',
				white: '#e6e9ef',
			},
		});
		const fit = new FitAddon();
		t.loadAddon(fit);
		t.onData((data) => {
			if (sessionId) void terminalWrite(sessionId, data);
		});
		t.onResize(({ cols, rows }) => {
			if (sessionId) void terminalResize(sessionId, cols, rows);
		});
		t.open(host);
		fit.fit();

		term = t;
		fitAddon = fit;

		const info = await terminalOpen(cwd_, (data) => {
			if (data === EOF_MARKER) {
				closed = true;
				return;
			}
			t.write(data);
		});
		sessionId = info.id;
		cwd = info.cwd;
	}

	async function closeSession() {
		if (sessionId) {
			await terminalClose(sessionId);
			sessionId = null;
		}
		closed = true;
	}

	function back() {
		if (sessionId) void terminalClose(sessionId);
		sessionId = null;
		term?.dispose();
		term = null;
		setView('browser');
	}

	let resizeObs: ResizeObserver | null = null;
	let started = false;

	$effect(() => {
		const el = host;
		if (!el) return;
		if (!started) {
			started = true;
			void open(cwd);
		}
		if (!resizeObs) {
			resizeObs = new ResizeObserver(() => {
				try {
					fitAddon?.fit();
				} catch {
					// Term nicht sichtbar
				}
			});
			resizeObs.observe(el);
		}
		return () => {
			resizeObs?.disconnect();
			resizeObs = null;
		};
	});
</script>

<div class="term-host" style="flex:1; min-height:0; display:flex; flex-direction:column; background:#0d1117;">
	<div
		style="display:flex; align-items:center; gap:8px; padding:8px 12px; background:var(--bg-elev); border-bottom:1px solid var(--border); flex:none;"
	>
		<button class="toolbar-btn" onclick={back}><Icon name="back" size={15} /> Browser</button>
		<span style="display:inline-flex; gap:6px; align-items:center; font-size:13px; color:var(--text-dim);" title={cwd}>
			<Icon name="terminal" size={14} /> {cwd}
		</span>
		<div style="flex:1;"></div>
		{#if closed}
			<span style="font-size:12px; color:var(--text-faint);">Sitzung beendet.</span>
		{/if}
		<button class="toolbar-btn" onclick={() => void open(cwd)} title="Neu starten">
			<Icon name="refresh" size={14} />
		</button>
		<button class="toolbar-btn" onclick={() => void closeSession()}>
			<Icon name="close" size={14} /> Beenden
		</button>
	</div>
	<div bind:this={host} style="flex:1; min-height:0; overflow:hidden;"></div>
</div>