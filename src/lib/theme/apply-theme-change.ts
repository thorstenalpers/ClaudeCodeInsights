/**
 * Applies a change to the theme classes on <html>.
 *
 * Every theme change must go through here. Chromium keeps the *old* colour
 * indefinitely on any element that has a `transition` covering `background-color`
 * when that colour comes from a CSS custom property and the property changes on
 * an ancestor. shadcn's Button carries `transition-all`, so switching the theme
 * without this guard leaves buttons painted in the previous theme — permanently,
 * not just for the transition duration. Measured in Chromium 2026-07-31: still
 * the stale value after 3 s and a forced repaint.
 *
 * Suppressing transitions across the swap sidesteps it, and a theme change has
 * no business animating anyway.
 */
export function applyThemeChange(mutate: () => void): void {
	const style = document.createElement('style');
	style.textContent = '*,*::before,*::after{transition:none!important;animation:none!important}';
	document.head.appendChild(style);

	mutate();

	// Force the new values to be computed while transitions are still suppressed.
	void document.body.offsetHeight;

	requestAnimationFrame(() => requestAnimationFrame(() => style.remove()));
}
