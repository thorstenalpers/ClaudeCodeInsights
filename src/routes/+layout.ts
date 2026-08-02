// A Tauri window has no server behind it and always boots at the root, so the
// whole app is one client-routed shell. Prerendering the fixed pages would only
// produce files the router never asks for — the adapter's fallback is the page.
export const ssr = false;
export const prerender = false;
