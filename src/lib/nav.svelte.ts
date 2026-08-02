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
}

export const nav = new Nav();
