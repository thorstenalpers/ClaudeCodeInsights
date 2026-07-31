import { mount, tick } from 'svelte';
import App from './App.svelte';
import './app.css';
import { bridge } from './lib/bridge/client';

const app = mount(App, {
  target: document.getElementById('app')!,
});

/** Resolves once a real frame has been painted, or after `fallbackMs` regardless. */
function afterFirstPaint(fallbackMs: number): Promise<void> {
  return new Promise((resolve) => {
    // Two nested frames: the first is scheduled before the upcoming paint, the
    // second only runs after it.
    requestAnimationFrame(() => requestAnimationFrame(() => resolve()));

    // rAF is throttled to a standstill whenever the WebView is not compositing
    // — minimised, occluded, or hidden by the host. Without this fallback the
    // splash would never be dismissed in exactly those cases.
    setTimeout(resolve, fallbackMs);
  });
}

/**
 * Tells the host the splash screen may go away.
 *
 * NavigationCompleted fires long before anything is on screen, so the host waits
 * for this instead.
 */
async function signalReady(): Promise<void> {
  if (!bridge.isHosted) return;

  await tick();
  await afterFirstPaint(500);

  await bridge.call('app.ready').catch(() => {
    // The host reveals the UI on a timeout anyway; a failure here must not
    // leave the user with a blank window.
  });
}

void signalReady();

export default app;
