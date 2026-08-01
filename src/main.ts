import { mount, tick } from 'svelte';
import App from './App.svelte';
import './app.css';
import { isHosted, reportReady } from './lib/ipc.svelte';

const app = mount(App, {
	target: document.getElementById('app')!
});

/** Resolves once a real frame has been painted, or after `fallbackMs` regardless. */
function afterFirstPaint(fallbackMs: number): Promise<void> {
	return new Promise((resolve) => {
		// Two nested frames: the first is scheduled before the upcoming paint, the
		// second only runs after it.
		requestAnimationFrame(() => requestAnimationFrame(() => resolve()));

		// The main window starts hidden, and a window that is not compositing gets
		// its animation frames throttled to a standstill — without this fallback the
		// splash would wait forever for a frame that is not coming.
		setTimeout(resolve, fallbackMs);
	});
}

async function signalReady(): Promise<void> {
	if (!isHosted) return;

	await tick();
	await afterFirstPaint(500);
	await reportReady('frontend').catch(() => {
		// The host reveals the window on a timeout anyway; a failure here must not
		// strand the user on the splash screen.
	});
}

void signalReady();

export default app;
