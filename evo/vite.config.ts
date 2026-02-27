import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import fs from "fs";
import path from "path";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// Plugin to copy monaco-editor files
const copyMonaco = () => {
  return {
    name: 'copy-monaco',
    closeBundle: () => {
      const src = path.resolve(__dirname, 'node_modules/monaco-editor/min/vs');
      const dest = path.resolve(__dirname, 'dist/monaco-editor/vs');
      if (fs.existsSync(src)) {
        fs.mkdirSync(dest, { recursive: true });
        fs.cpSync(src, dest, { recursive: true });
        console.log(`[copy-monaco] Copied Monaco Editor files to ${dest}`);
      } else {
        console.warn(`[copy-monaco] Source directory not found: ${src}`);
      }
    }
  }
}

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [vue(), copyMonaco()],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
    fs: {
      strict: false, // Allow serving files outside of workspace root if needed, but mainly to avoid issues with source maps
    }
  },
  build: {
    sourcemap: false, // Disable source maps in production to avoid the error
  }
}));
