/**
 * What the router cannot answer on its own.
 *
 * The URL says which session is open; it does not say what to call it. The
 * sessions table already knows the topic when it navigates, so it hands it over
 * here and the header can show a name instead of a UUID while the transcript is
 * still loading.
 */
class Nav {
	detailLabel = $state('');

	/**
	 * An activity the sessions page should open filtered to.
	 *
	 * Handed over rather than put in the URL: `resolve()` addresses routes, not
	 * query strings, and the app has no address bar for a link to be worth
	 * anything in. The page takes it once and clears it, so arriving any other
	 * way shows the whole history.
	 */
	sessionActivity = $state<string | null>(null);

	take(): string | null {
		const activity = this.sessionActivity;
		this.sessionActivity = null;
		return activity;
	}
}

export const nav = new Nav();
