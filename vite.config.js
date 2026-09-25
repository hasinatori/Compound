// Конфиг Vite: алиасы, порт дев-сервера, обход Tauri в деве.
// Vite config: aliases, dev-server port, Tauri bypass in dev.
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
// @ts-expect-error type error without @types/node package
import process from "node:process";

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig({
  plugins: [svelte()],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  clearScreen: false,

  server: {
    port: 1420,
    strictPort: true,
    host: host || "127.0.0.1",
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },

  build: {
    // Tauri expects a fixed output dir, see src-tauri/tauri.conf.json -> build.frontendDist
    outDir: "build",
    emptyOutDir: true,
    target: "es2021",
  },
});