import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
export default {
	preprocess: vitePreprocess(),
	kit: {
		// A desktop app has no server to fall back to, so the build is one shell
		// that routes on the client.
		adapter: adapter({ fallback: 'index.html', strict: false }),
		alias: {
			$views: 'src/views'
		}
	}
};
