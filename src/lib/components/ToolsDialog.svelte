<script lang="ts">
	// Инструменты: дубликаты, сравнение папок, массовое переименование.
	// Tools: duplicates, folder compare, mass rename.
	// Правила переименования собираются здесь, считает и превью — backend.
	// Rename rules are assembled here; counting + preview run in the backend.
	import Icon from './Icon.svelte';
	import {
		uiDialog,
		closeDialog,
		notify,
		panelA,
		panelB,
		focusedPanel,
		lang,
		errMsg,
	} from '../stores.svelte';
	import { t } from '../i18n';
	import { findDuplicates, compareFolders, multiRename, trashItems } from '../api';
	import type {
		CompareItem,
		DuplicateGroup,
		RenamePreviewItem,
		RenameRule,
		RuleType,
	} from '../types';
	import { formatBytes } from '../format';

	const d = $derived(uiDialog.d?.kind === 'tools' ? true : false);
	const baseDir = $derived((focusedPanel.value === 0 ? panelA : panelB).cwd);

	let tab = $state<'dup' | 'cmp' | 'ren'>('dup');

	// ---- Duplikate ----
	let dupRunning = $state(false);
	let dupGroups = $state<DuplicateGroup[]>([]);
	// $state(baseDir) ВМЕСТО $state('') ВАЖНО: derived читается только в $effect,
	// иначе значение застывает на момент монтирования.
	// $state(baseDir) instead of $state('') matters: a derived may only be read
	// inside an effect, otherwise the value freezes at mount time.
	let dupRoot = $state('');
	$effect(() => {
		if (d) dupRoot = baseDir;
	});

	async function runDup() {
		dupRunning = true;
		dupGroups = [];
		try {
			dupGroups = await findDuplicates(dupRoot, () => {});
			notify('info', t('tools.dupDone', lang(), { n: dupGroups.length }));
		} catch (e) {
			notify('error', errMsg(e));
		} finally {
			dupRunning = false;
		}
	}

	async function trashDups(group: DuplicateGroup) {
		if (group.files.length < 2) return;
		// Alle außer dem ersten in den Papierkorb
		const rest = group.files.slice(1);
		try {
			await trashItems(rest);
			notify('ok', t('tools.movedToTrash', lang(), { n: rest.length }));
			await runDup();
		} catch (e) {
			notify('error', errMsg(e));
		}
	}

	// ---- Vergleich ----
	let cmpRunning = $state(false);
	// Те же два начальных значения читаем в $effect, а не при инициализации:
	// иначе «Сравнить» стартует со старым каталогом панели.
	// Seed both paths inside an effect for the same reason: otherwise Compare
	// starts from the panel's stale directory.
	let cmpA = $state('');
	let cmpB = $state('');
	$effect(() => {
		if (d) {
			cmpA = baseDir;
			cmpB = baseDir;
		}
	});
	let cmpDeep = $state(true);
	let cmpItems = $state<CompareItem[]>([]);

	async function runCmp() {
		cmpRunning = true;
		cmpItems = [];
		try {
			cmpItems = await compareFolders(cmpA, cmpB, cmpDeep, () => {});
			notify('info', t('tools.cmpDone', lang(), { n: cmpItems.length }));
		} catch (e) {
			notify('error', errMsg(e));
		} finally {
			cmpRunning = false;
		}
	}

	function cmpStat(i: CompareItem) {
		switch (i.status) {
			case 'only_in_a':
				return { icon: 'back' as const, color: 'var(--info)', label: t('tools.onlyInA', lang()) };
			case 'only_in_b':
				return { icon: 'forward' as const, color: 'var(--warn)', label: t('tools.onlyInB', lang()) };
			case 'different':
				return { icon: 'error' as const, color: 'var(--err)', label: t('tools.different', lang()) };
			default:
				return { icon: 'check' as const, color: 'var(--ok)', label: t('tools.identical', lang()) };
		}
	}

	// ---- Massen-Umbenennen ----
	let renItems = $state<string[]>((focusedPanel.value === 0 ? panelA : panelB).selected);
	let rules = $state<RenameRule[]>([]);
	let renPreview = $state<RenamePreviewItem[]>([]);
	let renError = $state<string | null>(null);

	$effect(() => {
		if (d) {
			const p = focusedPanel.value === 0 ? panelA : panelB;
			renItems = p.selected.length > 0 ? [...p.selected] : [];
			if (rules.length === 0) {
				rules = [defaultsFor('find_replace')];
			}
		}
	});

	async function computePreview() {
		if (renItems.length === 0) {
			notify('warn', t('tools.noSelection', lang()));
			return;
		}
		try {
			renPreview = await multiRename(renItems, rules, true);
			renError = renPreview.some((r) => r.error) ? t('tools.someFailed', lang()) : null;
		} catch (e) {
			renError = errMsg(e);
		}
	}

	async function applyRename() {
		if (renPreview.length === 0) {
			notify('warn', t('tools.previewFirst', lang()));
			return;
		}
		if (renPreview.some((r) => r.conflict)) {
			notify('error', t('tools.conflictsPresent', lang()));
			return;
		}
		try {
			await multiRename(renItems, rules, false);
			notify('ok', t('tools.renamedDone', lang()));
			renPreview = [];
		} catch (e) {
			notify('error', errMsg(e));
		}
	}

	function defaultsFor(t: RuleType): RenameRule {
		switch (t) {
			case 'find_replace':
				return { ruleType: t, find: '', replace: '' };
			case 'regex':
				return { ruleType: t, find: '', replace: '' };
			case 'remove':
				return { ruleType: t, find: '' };
			case 'insert':
				return { ruleType: t, insertAt: 0, insertText: '' };
			case 'case':
				return { ruleType: t, caseMode: 'upper' };
			case 'ext':
				return { ruleType: t, newExt: '' };
			case 'numbering':
				return { ruleType: t, template: 'Datei {n}', start: 1, digits: 2 };
		}
	}

	function addRule() {
		rules.push(defaultsFor('find_replace'));
	}

	function remRule(i: number) {
		rules.splice(i, 1);
	}

	function setRuleType(r: RenameRule, t: RuleType) {
		Object.assign(r, defaultsFor(t));
	}

	const ruleTypes: RuleType[] = ['find_replace', 'regex', 'remove', 'insert', 'case', 'ext', 'numbering'];
	const ruleLabel = (ty: RuleType) => t(`rename.ruleTypes.${ty}`, lang());
</script>

{#if d}
	<div class="overlay">
		<div class="dialog" style="width: 900px; max-width: 95vw; height: 85vh;">
			<div class="dialog-header">
				<Icon name="tools" size={16} />&nbsp;{t('tools.title', lang())}
				<button class="toolbar-btn" onclick={closeDialog}><Icon name="close" size={15} /></button>
			</div>
			<div class="tabs">
				<button class="tab" class:active={tab === 'dup'} onclick={() => (tab = 'dup')}>
					{t('tools.dupFind', lang())}
				</button>
				<button class="tab" class:active={tab === 'cmp'} onclick={() => (tab = 'cmp')}>
					{t('tools.cmpFolders', lang())}
				</button>
				<button class="tab" class:active={tab === 'ren'} onclick={() => (tab = 'ren')}>
					{t('tools.multiRename', lang())}
				</button>
			</div>
			<div class="dialog-body" style="display:flex; flex-direction:column; gap:10px;">
				{#if tab === 'dup'}
					<div style="display:flex; gap:10px; align-items:center;">
						<label style="font-size:13px; display:flex; gap:8px; align-items:center; flex:1;">
							{t('tools.searchIn', lang())}
							<input bind:value={dupRoot} style="flex:1;" />
						</label>
						<button class="btn primary" onclick={() => void runDup()} disabled={dupRunning}>
							<Icon name="search" size={14} /> {t('tools.searchBtn', lang())}
						</button>
					</div>
					<div style="flex:1; overflow:auto;">
						{#if dupRunning}
							<div style="text-align:center; color:var(--text-dim); padding:30px;">{t('tools.scanning', lang())}</div>
						{:else if dupGroups.length === 0}
							<div style="text-align:center; color:var(--text-faint); padding:30px;">
								{t('tools.dupEmpty', lang())}
							</div>
						{:else}
							{#each dupGroups as g, gi (g.hash)}
								<div style="margin-bottom:12px; background:var(--bg-active); border-radius:8px; padding:10px;">
									<div style="display:flex; align-items:center; gap:8px; margin-bottom:8px;">
										<Icon name="duplicate" size={14} />
										<span style="font-size:13px; font-weight:600;">{t('tools.group', lang(), { n: gi + 1 })}</span>
										<span style="font-size:12px; color:var(--text-dim);">{formatBytes(g.size)} · {g.hash.slice(0, 12)}…</span>
										<div style="flex:1;"></div>
										<button class="btn" onclick={() => void trashDups(g)}>
											<Icon name="trash" size={13} /> {t('tools.scrapN', lang(), { n: g.files.length - 1 })}
										</button>
									</div>
									{#each g.files as f, fi (f)}
										<div style="font-size:12px; padding:2px 4px; color:{fi === 0 ? 'var(--text)' : 'var(--text-dim)'}; word-break:break-all;">
											{fi === 0 ? '★ ' : '· '}{f}
										</div>
									{/each}
								</div>
							{/each}
						{/if}
					</div>
				{:else if tab === 'cmp'}
					<div style="display:flex; gap:10px; flex-wrap:wrap; align-items:center;">
						<label style="font-size:13px; display:flex; gap:6px; align-items:center;">
							A:
							<input bind:value={cmpA} style="width:280px;" />
						</label>
						<label style="font-size:13px; display:flex; gap:6px; align-items:center;">
							B:
							<input bind:value={cmpB} style="width:280px;" />
						</label>
						<label style="font-size:13px; display:flex; gap:6px; align-items:center; cursor:pointer;">
							<input type="checkbox" bind:checked={cmpDeep} /> {t('tools.deep', lang())}
						</label>
						<button class="btn primary" onclick={() => void runCmp()} disabled={cmpRunning}>
							<Icon name="swap" size={14} /> {t('tools.cmpBtn', lang())}
						</button>
					</div>
					<div style="flex:1; overflow:auto;">
						{#if cmpRunning}
							<div style="text-align:center; color:var(--text-dim); padding:30px;">{t('tools.comparing', lang())}</div>
						{:else if cmpItems.length === 0}
							<div style="text-align:center; color:var(--text-faint); padding:30px;">
								{t('tools.cmpEmpty', lang())}
							</div>
						{:else}
							<table class="tbl">
								<thead>
									<tr>
										<th>{t('misc.status', lang())}</th>
										<th>{t('misc.element', lang())}</th>
										<th style="text-align:right;">A</th>
										<th style="text-align:right;">B</th>
									</tr>
								</thead>
								<tbody>
									{#each cmpItems as it (it.relative)}
										{@const s = cmpStat(it)}
										<tr>
											<td style="color:{s.color};">
												<span style="display:inline-flex; gap:5px; align-items:center;">
													<Icon name={s.icon} size={13} />{s.label}
												</span>
											</td>
											<td style="word-break:break-all;">{it.relative}</td>
											<td style="text-align:right; color:var(--text-dim);">{it.sizeA >= 0 ? formatBytes(it.sizeA) : '–'}</td>
											<td style="text-align:right; color:var(--text-dim);">{it.sizeB >= 0 ? formatBytes(it.sizeB) : '–'}</td>
										</tr>
									{/each}
								</tbody>
							</table>
						{/if}
					</div>
				{:else}
					<div class="form-row">
						<div>{t('tools.selectedCount', lang(), { n: renItems.length })}</div>
						<div style="max-height:110px; overflow:auto; background:var(--bg-active); border-radius:6px; padding:8px; font-size:12px; color:var(--text-dim);">
							{#each renItems as it (it)}
								<div style="word-break:break-all;">{it}</div>
							{/each}
						</div>
					</div>
					<div style="display:flex; align-items:center; gap:8px; margin-bottom:6px;">
						<span style="font-size:13px; font-weight:600;">{t('tools.rules', lang())}</span>
						<div style="flex:1;"></div>
						<button class="btn" onclick={addRule}><Icon name="plus" size={13} /> {t('tools.rule', lang())}</button>
						<button class="btn" onclick={() => void computePreview()}>
							<Icon name="preview" size={13} /> {t('tools.preview', lang())}
						</button>
						<button class="btn primary" onclick={() => void applyRename()}>
							<Icon name="rename" size={13} /> {t('action.rename', lang())}
						</button>
					</div>
					<div style="display:flex; flex-direction:column; gap:8px; margin-bottom:8px;">
						{#each rules as r, i (i)}
							<div style="display:flex; gap:8px; align-items:center;">
								<select
									bind:value={r.ruleType}
									onchange={(e) => setRuleType(r, (e.currentTarget as HTMLSelectElement).value as RuleType)}
									style="width:150px; flex:none;"
								>
									{#each ruleTypes as rt}
										<option value={rt}>{ruleLabel(rt)}</option>
									{/each}
								</select>
								{#if r.ruleType === 'find_replace' || r.ruleType === 'regex' || r.ruleType === 'remove'}
									<input bind:value={r.find} placeholder={t('tools.findPlaceholder', lang())} style="width:150px;" />
								{/if}
								{#if r.ruleType === 'find_replace' || r.ruleType === 'regex'}
									<input bind:value={r.replace} placeholder={t('tools.replacePlaceholder', lang())} style="width:150px;" />
								{/if}
								{#if r.ruleType === 'insert'}
									<input bind:value={r.insertAt} type="number" min="0" style="width:80px;" />
									<input bind:value={r.insertText} placeholder={t('tools.placeholderText', lang())} style="width:150px;" />
								{/if}
								{#if r.ruleType === 'case'}
									<select bind:value={r.caseMode} style="width:150px;">
										<option value="upper">{t('rename.uppercase', lang())}</option>
										<option value="lower">{t('rename.lowercase', lang())}</option>
										<option value="title">{t('rename.titlecase', lang())}</option>
									</select>
								{/if}
								{#if r.ruleType === 'ext'}
									<input bind:value={r.newExt} placeholder={t('tools.placeholderNewExt', lang())} style="width:150px;" />
								{/if}
								{#if r.ruleType === 'numbering'}
									<input bind:value={r.template} placeholder={t('tools.templatePlaceholder', lang())} style="width:170px;" />
									<input bind:value={r.start} type="number" min="0" style="width:70px;" />
									<input bind:value={r.digits} type="number" min="1" max="6" style="width:70px;" title={t('tools.digits', lang())} />
								{/if}
								<button class="toolbar-btn" onclick={() => remRule(i)}>
									<Icon name="minus" size={13} />
								</button>
							</div>
						{/each}
					</div>
					{#if renError}
						<div style="color:var(--err); font-size:12px; margin-bottom:6px;">{renError}</div>
					{/if}
					<div style="flex:1; overflow:auto; min-height:120px;">
						{#if renPreview.length > 0}
							<table class="tbl">
								<thead>
									<tr><th>{t('tools.from', lang())}</th><th>→ {t('tools.to', lang())}</th><th>{t('misc.status', lang())}</th></tr>
								</thead>
								<tbody>
									{#each renPreview as pv (pv.from)}
										<tr style="color:{pv.conflict ? 'var(--warn)' : pv.error ? 'var(--err)' : 'var(--text)'};">
											<td style="word-break:break-all;">{pv.from}</td>
											<td style="word-break:break-all;">{(pv.from.split('/').slice(0, -1).join('/') + '/' + pv.to.split('/').pop())}</td>
											<td>{pv.error ? errMsg(pv.error) : (pv.conflict ? t('tools.conflict', lang()) : 'ok')}</td>
										</tr>
									{/each}
								</tbody>
							</table>
						{/if}
					</div>
				{/if}
			</div>
		</div>
	</div>
{/if}