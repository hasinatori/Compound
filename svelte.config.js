// Конфиг Svelte. По сути только препроцессор под TS.
// Svelte config. Basically just the TS preprocessor.
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

/** @type {import('@sveltejs/vite-plugin-svelte').SvelteConfig} */
const config = {
  preprocess: vitePreprocess(),
};

export default config;