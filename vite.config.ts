import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
	plugins: [tailwindcss(), sveltekit()],

	// Tauri needs a fixed port; letting Vite silently pick another one leaves the
	// window pointing at nothing.
	clearScreen: false,
	server: {
		port: 5177,
		strictPort: true,
		host: host || false,
		hmr: host ? { protocol: 'ws', host, port: 5174 } : undefined,
		watch: {
			// Cargo has its own watcher. Letting Vite watch the Rust tree as well
			// turns every backend rebuild into a frontend reload storm.
			ignored: ['**/src-tauri/**']
		}
	},

	build: {
		target: 'esnext',
		// DevTools are off in release builds, so a shipped source map is dead weight.
		sourcemap: !!process.env.TAURI_ENV_DEBUG
	}
});
