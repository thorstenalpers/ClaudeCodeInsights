/**
 * The price table, and the cost derived from it.
 *
 * Cost is never stored. It is worked out from the token counts and these rates
 * whenever a figure is shown, so correcting a rate never means rescanning a
 * single transcript.
 *
 * USD per million tokens, as published for the API. A model that is not listed
 * falls back to the closest family match, and anything unrecognised is priced
 * at zero rather than guessed — a wrong number is worse than an obvious gap.
 */
export type Rate = {
	input: number;
	output: number;
	cacheRead: number;
	cacheWrite: number;
};

const RATES: { match: RegExp; rate: Rate }[] = [
	{
		match: /opus/i,
		rate: { input: 15, output: 75, cacheRead: 1.5, cacheWrite: 18.75 }
	},
	{
		match: /fable|mythos/i,
		rate: { input: 15, output: 75, cacheRead: 1.5, cacheWrite: 18.75 }
	},
	{
		match: /sonnet/i,
		rate: { input: 3, output: 15, cacheRead: 0.3, cacheWrite: 3.75 }
	},
	{
		match: /haiku/i,
		rate: { input: 0.8, output: 4, cacheRead: 0.08, cacheWrite: 1 }
	}
];

const ZERO: Rate = { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 };

export function rateFor(model: string): Rate {
	return RATES.find((entry) => entry.match.test(model))?.rate ?? ZERO;
}

export function isPriced(model: string): boolean {
	return RATES.some((entry) => entry.match.test(model));
}

export type TokenCounts = {
	inputTokens: number;
	outputTokens: number;
	cacheReadTokens: number;
	cacheWriteTokens: number;
};

export function costOf(model: string, tokens: TokenCounts): number {
	const rate = rateFor(model);
	return (
		(tokens.inputTokens * rate.input +
			tokens.outputTokens * rate.output +
			tokens.cacheReadTokens * rate.cacheRead +
			tokens.cacheWriteTokens * rate.cacheWrite) /
		1_000_000
	);
}

/** What the same tokens would have cost had nothing been served from cache. */
export function uncachedCostOf(model: string, tokens: TokenCounts): number {
	const rate = rateFor(model);
	return (
		((tokens.inputTokens + tokens.cacheReadTokens + tokens.cacheWriteTokens) * rate.input +
			tokens.outputTokens * rate.output) /
		1_000_000
	);
}
