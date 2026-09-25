// Хелперы вывода: байты, даты, права, мн. число. Без ран.
// Output helpers: bytes, dates, perms, plural. No runes.

export function formatBytes(n: number): string {
	if (!Number.isFinite(n) || n < 0) return '–';
	if (n < 1024) return `${n} B`;
	const units = ['KB', 'MB', 'GB', 'TB', 'PB'];
	let v = n;
	let u = -1;
	do {
		v /= 1024;
		u++;
	} while (v >= 1024 && u < units.length - 1);
	return `${v.toFixed(v >= 100 ? 0 : v >= 10 ? 1 : 2)} ${units[u]}`;
}

export function formatDate(ms: number, lang: 'de' | 'en' = 'de'): string {
	if (!Number.isFinite(ms) || ms <= 0) return '–';
	const d = new Date(ms);
	const now = new Date();
	const sameDay = d.toDateString() === now.toDateString();
	const sameYear = d.getFullYear() === now.getFullYear();
	const time = d.toLocaleTimeString(lang === 'de' ? 'de-DE' : 'en-GB', { hour: '2-digit', minute: '2-digit' });
	if (sameDay) return time;
	if (sameYear) {
		return d.toLocaleDateString(lang === 'de' ? 'de-DE' : 'en-GB', { day: '2-digit', month: 'short' }) + ' ' + time;
	}
	return d.toLocaleDateString(lang === 'de' ? 'de-DE' : 'en-GB', {
		day: '2-digit',
		month: '2-digit',
		year: 'numeric',
	});
}

export function formatPerms(p: string): string {
	if (!p || p.length < 9) return p;
	const map = (c: string) => (c === 'r' ? 'r' : c === 'w' ? 'w' : c === 'x' ? 'x' : '-');
	return p
		.slice(0, 9)
		.split('')
		.map((c, i) => (i % 3 === 0 ? map(c) : map(c)))
		.join('');
}

export function formatPct(part: number, total: number): number {
	if (total <= 0) return 0;
	return Math.min(100, Math.round((part / total) * 100));
}

export function fileIconName(e: { isDir: boolean; kind: string; ext: string }): string {
	if (e.isDir) return 'folder';
	switch (e.kind) {
		case 'image':
			return 'image';
		case 'video':
			return 'video';
		case 'audio':
			return 'audio';
		case 'archive':
			return 'zip';
		case 'code':
		case 'text':
			return e.kind === 'code' ? 'code' : 'text';
		case 'trash':
			return 'trash';
		default:
			return 'file';
	}
}

export function plural(n: number, one: string, many: string, lang: 'de' | 'en' = 'de'): string {
	if (lang === 'de') return n === 1 ? one : many;
	return n === 1 ? one : many;
}

export function clamp(n: number, min: number, max: number): number {
	return Math.min(max, Math.max(min, n));
}

// Natürlicher Dateinamen-Vergleich (Dezimale in "Datei 2" < "Datei 10").
export function compareNat(a: string, b: string): number {
	return a.localeCompare(b, undefined, { numeric: true, sensitivity: 'base' });
}

export function extOf(name: string): string {
	const i = name.lastIndexOf('.');
	if (i <= 0) return '';
	return name.slice(i + 1).toLowerCase();
}