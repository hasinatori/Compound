// Пути иконок в стиле lucide (24x24, stroke). Не менять координаты.
// Icon paths, lucide style (24x24, stroke). Keep coordinates intact.
export interface IconDef {
	paths: string[]; // <path d=...>
	circles?: Array<[number, number, number]>;
	lines?: Array<[number, number, number, number]>;
}

export const ICONS: Record<string, IconDef> = {
	folder: { paths: ['M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z'] },
	folderOpen: { paths: ['M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v1H7a2 2 0 0 0-2 2v6l-2-2z', 'M5 12l-1.8 5.4A2 2 0 0 0 5.1 20h12a2 2 0 0 0 1.9-1.4L21 12'] },
	file: { paths: ['M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z', 'M14 3v5h5'] },
	text: { paths: ['M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z', 'M14 3v5h5', 'M9 13h6', 'M9 17h6'] },
	code: { paths: ['M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z', 'M14 3v5h5', 'm10 13-2 2 2 2', 'm14 13 2 2-2 2'] },
	image: { paths: ['M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z', 'M14 3v5h5', 'm10 13-2 2 2 2 4-4'] },
	video: { paths: ['M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z', 'M14 3v5h5', 'm10 12 5 3-5 3z'] },
	audio: { paths: ['M12 5a3 3 0 0 1 6 0v9a3 3 0 1 1-6 0V5z', 'M9 15v-1', 'M9 15a3 3 0 0 1-6 0v-1', 'M9 11a3 3 0 0 1-6 0'] },
	zip: { paths: ['M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z', 'M14 3v5h5', 'M9 13.5c0-.5.5-1 1-1s1 .5 1 1c0 2-2 2-2 4h3', 'M12 13.5h.01'] },
	trash: { paths: ['M3 6h18', 'M8 6V4a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v2', 'M6 6l1 14a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2l1-14', 'M10 11v6', 'M14 11v6'] },
	home: { paths: ['M3 11.5 12 4l9 7.5', 'M5 10v10h14V10'] },
	hard: { paths: ['M4 6h16v12H4z', 'M4 10h16', 'M8 15h.01'] },
	up: { paths: ['M12 19V5', 'm5 12 7-7 7 7'] },
	back: { paths: ['M19 12H5', 'm12 19-7-7 7-7'] },
	forward: { paths: ['M5 12h14', 'm12 5 7 7-7 7'] },
	search: { paths: ['M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14z', 'm21 21-4.3-4.3'] },
	edit: { paths: ['M12 20h9', 'M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4z'] },
	terminal: { paths: ['M4 17 10 12 4 7', 'M12 19h8'] },
	copy: { paths: ['M8 8h12v12H8z', 'M16 8V4H4v12h4'] },
	cut: { paths: ['M6 7a3 3 0 1 0 0-6 3 3 0 0 0 0 6z', 'M6 23a3 3 0 1 0 0-6 3 3 0 0 0 0 6z', 'M8.1 8.2 20 19', 'M8.1 15.8 20 5'] },
	paste: { paths: ['M9 5H5a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V7a2 2 0 0 0-2-2h-4', 'M9 3h6v4H9z'] },
	rename: { paths: ['M12 20h9', 'M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4z', 'M12 5h3'] },
	duplicate: { paths: ['M9 9h10v10H9z', 'M5 15H4a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h10a1 1 0 0 1 1 1v1'] },
	newFile: { paths: ['M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z', 'M14 3v5h5', 'M12 18v-6', 'M9 15h6'] },
	newFolder: { paths: ['M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z', 'M12 11v6', 'M9 14h6'] },
	archive: { paths: ['M3 7h18v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z', 'M3 7l2-3h14l2 3', 'M9 12h6'] },
	extract: { paths: ['M12 3v12', 'm7 10 5 5 5-5', 'M5 19h14'] },
	settings: { paths: ['M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8z', 'M12 2v3', 'M12 19v3', 'M2 12h3', 'M19 12h3', 'M4.9 4.9l2.1 2.1', 'M17 17l2.1 2.1', 'M19.1 4.9 17 7', 'M7 17l-2.1 2.1'] },
	tools: { paths: ['M14.7 6.3a4 4 0 0 0-5.4 5.4L3 18v3h3l6.3-6.3a4 4 0 0 0 5.4-5.4l-3 3-2.8-.8-.8-2.8z'] },
	preview: { paths: ['M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12z', 'M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z'] },
	close: { paths: ['M6 6l12 12', 'M18 6 6 18'] },
	check: { paths: ['m5 13 4 4L19 7'] },
	info: { paths: ['M12 4a8 8 0 1 0 0 16 8 8 0 0 0 0-16z', 'M12 11v5', 'M12 8h.01'] },
	warn: { paths: ['M12 3 2 21h20z', 'M12 10v4', 'M12 17.5h.01'] },
	error: { paths: ['M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18z', 'm9 9 6 6', 'm15 9-6 6'] },
	sort: { paths: ['M8 6h13', 'M8 12h10', 'M8 18h6', 'M3 5l2 2 2-2', 'M3 11l2 2 2-2', 'M3 17l2 2 2-2', 'M8 18h0'] },
	sortDesc: { paths: ['M8 6h13', 'M8 12h10', 'M8 18h6', 'M3 19l2-2 2 2', 'M3 13l2-2 2 2', 'M3 7l2-2 2 2', 'M8 18h0'] },
	list: { paths: ['M8 6h13', 'M8 12h13', 'M8 18h13', 'M3 6h.01', 'M3 12h.01', 'M3 18h.01'] },
	single: { paths: ['M3 5h18v14H3z'] },
	dual: { paths: ['M3 5h8v14H3z', 'M13 5h8v14h-8z'] },
	refresh: { paths: ['M21 12a9 9 0 1 1-2.6-6.4', 'M21 3v5h-5'] },
	openEx: { paths: ['M14 3h7v7', 'M21 3 11 13', 'M21 14v5a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5'] },
	closeParen: { paths: ['M4 5a5 5 0 0 1 0 14', 'M20 19a5 5 0 0 1 0-14'] },
	minus: { paths: ['M5 12h14'] },
	plus: { paths: ['M12 5v14', 'M5 12h14'] },
	clock: { paths: ['M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18z', 'M12 7v5l3 2'] },
	cal: { paths: ['M4 5h16v16H4z', 'M4 9h16', 'M8 3v4', 'M16 3v4'] },
	hash: { paths: ['M4 9h16', 'M4 15h16', 'M10 3 8 21', 'M16 3l-2 18'] },
	swap: { paths: ['M8 3 4 7l4 4', 'M4 7h16', 'M16 21l4-4-4-4', 'M20 17H4'] },
	external: { paths: ['M14 3h7v7', 'M21 3 11 13', 'M21 14v5a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5'] },
	bookmark: { paths: ['M6 3h12v18l-6-4-6 4z'] },
	star: { paths: ['M12 3l2.7 5.5 6.3.9-4.5 4.4 1 6.2-5.5-2.9-5.5 2.9 1-6.2L3 9.4l6.3-.9z'] },
	drive: { paths: ['M2 13h20', 'M4 13l2-7h12l2 7v4a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1z'] },
	count: { paths: ['M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18z', 'M9 12h6', 'M12 9v6'] },
	key: { paths: ['M15 8a3 3 0 1 0-3 3', 'M12 11 4 19', 'M8 15l2 2'] },
	layout: { paths: ['M3 5h18v14H3z', 'M3 9h18', 'M9 9v10'] },
	chevron: { paths: ['m6 9 6 6 6-6'] },
	grid: { paths: ['M4 4h7v7H4z', 'M13 4h7v7h-7z', 'M4 13h7v7H4z', 'M13 13h7v7h-7z'] },
};

export function iconPath(name: string): IconDef {
	return ICONS[name] ?? ICONS.file;
}