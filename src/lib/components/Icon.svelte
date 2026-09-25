<script lang="ts">
	// Иконка по имени из icons.ts (lucide 24x24, stroke). Тонкая обёртка.
	// Single icon by name from icons.ts (lucide 24x24, stroke). Thin wrapper.
	import { iconPath } from '../icons';

	let { name = 'file', size = 16, color, strokeWidth = 1.7 }: {
		name?: string;
		size?: number;
		color?: string;
		strokeWidth?: number;
	} = $props();

	const def = $derived(iconPath(name));
</script>

<svg
	width={size}
	height={size}
	viewBox="0 0 24 24"
	fill="none"
	stroke={color ?? 'currentColor'}
	stroke-width={strokeWidth}
	stroke-linecap="round"
	stroke-linejoin="round"
	aria-hidden="true"
	style="flex: none;"
>
	{#each def.paths as p}
		<path d={p} />
	{/each}
	{#each def.circles ?? [] as c}
		<circle cx={c[0]} cy={c[1]} r={c[2]} />
	{/each}
	{#each def.lines ?? [] as l}
		<line x1={l[0]} y1={l[1]} x2={l[2]} y2={l[3]} />
	{/each}
</svg>