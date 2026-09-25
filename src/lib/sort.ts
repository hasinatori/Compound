import type { FileEntry, SortKey } from './types';
import { compareNat } from './format';

/** Gemeinsame Sortierung für Panel-Liste und Tastatur-Navigation. */
export function sortEntries(entries: FileEntry[], key: SortKey, desc: boolean): FileEntry[] {
	const arr = [...entries];
	const dirsFirst = (a: FileEntry, b: FileEntry) => (a.isDir === b.isDir ? 0 : a.isDir ? -1 : 1);
	arr.sort((a, b) => {
		const d = dirsFirst(a, b);
		if (d !== 0) return d;
		let c: number;
		switch (key) {
			case 'size':
				c = a.size - b.size;
				break;
			case 'mtime':
				c = a.mtimeMs - b.mtimeMs;
				break;
			case 'ext':
				c = compareNat(a.ext, b.ext) || compareNat(a.name, b.name);
				break;
			default:
				c = compareNat(a.name, b.name);
		}
		return desc ? -c : c;
	});
	return arr;
}