import { svelte } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';
import path from 'node:path';
import { defineConfig } from 'vite';

// The bundle is served from a WebView2 virtual host, never from a web server,
// so every asset URL must stay relative to index.html.
export default defineConfig(({ mode }) => ({
  base: './',
  plugins: [tailwindcss(), svelte()],
  resolve: {
    alias: {
      $lib: path.resolve(import.meta.dirname, './src/lib'),
    },
  },
  build: {
    outDir: '../ClaudeUsageAnalyzer.App/wwwroot',
    emptyOutDir: true,
    target: 'esnext',
    // DevTools are off in Release, so a shipped source map is dead weight in
    // the publish folder — it is the single largest file in the bundle.
    sourcemap: mode !== 'production',
  },
}));
