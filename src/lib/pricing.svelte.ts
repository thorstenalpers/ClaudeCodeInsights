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
 *
 * On a subscription these figures are not a bill. Tokens are covered by the
 * plan, and the number is what the same work would have cost on the API — a
 * yardstick, not an invoice. The billing mode below decides which of the two
 * the window claims it is showing.
 */
export type Rate = {
	input: number;
	output: number;
	cacheRead: number;
	cacheWrite: number;
};

/** A model family, matched by name because that is all a transcript records. */
export type FamilyId = 'opus' | 'fable' | 'sonnet' | 'haiku';

export const FAMILIES: { id: FamilyId; match: RegExp; published: Rate }[] = [
	{
		id: 'opus',
		match: /opus/i,
		published: { input: 15, output: 75, cacheRead: 1.5, cacheWrite: 18.75 }
	},
	{
		id: 'fable',
		match: /fable|mythos/i,
		published: { input: 15, output: 75, cacheRead: 1.5, cacheWrite: 18.75 }
	},
	{
		id: 'sonnet',
		match: /sonnet/i,
		published: { input: 3, output: 15, cacheRead: 0.3, cacheWrite: 3.75 }
	},
	{
		id: 'haiku',
		match: /haiku/i,
		published: { input: 0.8, output: 4, cacheRead: 0.08, cacheWrite: 1 }
	}
];

const ZERO: Rate = { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 };

const RATES_KEY = 'claudeadmin.rates';

function readStoredRates(): Partial<Record<FamilyId, Rate>> {
	if (typeof localStorage === 'undefined') return {};
	try {
		const raw: unknown = JSON.parse(localStorage.getItem(RATES_KEY) ?? '{}');
		if (typeof raw !== 'object' || raw === null) return {};
		const result: Partial<Record<FamilyId, Rate>> = {};
		for (const family of FAMILIES) {
			const entry = (raw as Record<string, unknown>)[family.id];
			if (typeof entry !== 'object' || entry === null) continue;
			const values = entry as Record<string, unknown>;
			const rate: Rate = { ...family.published };
			for (const field of ['input', 'output', 'cacheRead', 'cacheWrite'] as const) {
				const value = values[field];
				if (typeof value === 'number' && Number.isFinite(value) && value >= 0) rate[field] = value;
			}
			result[family.id] = rate;
		}
		return result;
	} catch {
		return {};
	}
}

/**
 * The rates in force, published or corrected.
 *
 * Prices move and this app never asks the network, so the table has to be
 * editable. An edit changes every figure on screen at once, because nothing is
 * stored — the numbers are derived from the token counts each time they are
 * shown.
 */
class Rates {
	overrides = $state<Partial<Record<FamilyId, Rate>>>(readStoredRates());

	for(family: FamilyId): Rate {
		return (
			this.overrides[family] ?? FAMILIES.find((entry) => entry.id === family)?.published ?? ZERO
		);
	}

	isEdited(family: FamilyId): boolean {
		return this.overrides[family] !== undefined;
	}

	set(family: FamilyId, field: keyof Rate, value: number): void {
		const current = this.for(family);
		this.overrides = { ...this.overrides, [family]: { ...current, [field]: value } };
		localStorage.setItem(RATES_KEY, JSON.stringify(this.overrides));
	}

	reset(): void {
		this.overrides = {};
		localStorage.removeItem(RATES_KEY);
	}
}

export const rates = new Rates();

function familyOf(model: string): FamilyId | null {
	return FAMILIES.find((entry) => entry.match.test(model))?.id ?? null;
}

export function rateFor(model: string): Rate {
	const family = familyOf(model);
	return family ? rates.for(family) : ZERO;
}

export function isPriced(model: string): boolean {
	return familyOf(model) !== null;
}

export type TokenCounts = {
	inputTokens: number;
	outputTokens: number;
	cacheReadTokens: number;
	cacheWriteTokens: number;
};

/** The same tokens priced as if one model had done all the work. */
export function costAsModel(target: string, tokens: TokenCounts): number {
	return costOf(target, tokens);
}

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

/**
 * A row that spans several models, priced per model and added up.
 *
 * `null` when the split is empty: a row whose tokens were never linked back to
 * a turn is unknown, not free, and a zero would read as the latter.
 */
export function costOfSplit(split: ({ model: string } & TokenCounts)[]): number | null {
	if (split.length === 0) return null;
	return split.reduce((sum, entry) => sum + costOf(entry.model, entry), 0);
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

export type BillingMode = 'api' | 'subscription';

const BILLING_KEY = 'claudeadmin.billing';

class Billing {
	mode = $state<BillingMode>(readStoredMode());

	set(next: BillingMode): void {
		this.mode = next;
		localStorage.setItem(BILLING_KEY, next);
	}
}

function readStoredMode(): BillingMode {
	if (typeof localStorage === 'undefined') return 'api';
	return localStorage.getItem(BILLING_KEY) === 'subscription' ? 'subscription' : 'api';
}

export const billing = new Billing();

/**
 * The published subscription plans, in USD per month.
 *
 * Hardcoded on purpose: the app never reaches the network, so these cannot be
 * fetched. They are here to answer one question — is the plan carrying its
 * weight — and a figure that is a month out of date still answers it.
 */
export const PLANS = [
	{ id: 'free', monthly: 0 },
	{ id: 'pro', monthly: 20 },
	{ id: 'max5', monthly: 100 },
	{ id: 'max20', monthly: 200 }
] as const;

export type PlanId = (typeof PLANS)[number]['id'];

/** What is actually being paid for, as opposed to what the plan costs at all. */
export const SUBSCRIPTIONS = PLANS.filter((entry) => entry.monthly > 0);

const PLAN_KEY = 'claudeadmin.plan';
const PLAN_HISTORY_KEY = 'claudeadmin.planHistory';
const PLAN_PRICE_KEY = 'claudeadmin.planPrices';

function readStoredPlan(): PlanId {
	if (typeof localStorage === 'undefined') return 'pro';
	const stored = localStorage.getItem(PLAN_KEY);
	return PLANS.some((plan) => plan.id === stored) ? (stored as PlanId) : 'pro';
}

/** A plan and the month it started, as `YYYY-MM`. */
export type PlanPeriod = { from: string; id: PlanId };

function isMonth(value: unknown): value is string {
	return typeof value === 'string' && /^\d{4}-\d{2}$/.test(value);
}

function readStoredPrices(): Partial<Record<PlanId, number>> {
	if (typeof localStorage === 'undefined') return {};
	try {
		const raw: unknown = JSON.parse(localStorage.getItem(PLAN_PRICE_KEY) ?? '{}');
		if (typeof raw !== 'object' || raw === null) return {};
		const result: Partial<Record<PlanId, number>> = {};
		for (const entry of PLANS) {
			const value = (raw as Record<string, unknown>)[entry.id];
			if (typeof value === 'number' && Number.isFinite(value) && value >= 0)
				result[entry.id] = value;
		}
		return result;
	} catch {
		return {};
	}
}

function readStoredHistory(): PlanPeriod[] {
	if (typeof localStorage === 'undefined') return [];
	try {
		const raw: unknown = JSON.parse(localStorage.getItem(PLAN_HISTORY_KEY) ?? '[]');
		if (!Array.isArray(raw)) return [];
		return raw
			.filter((entry): entry is PlanPeriod => {
				if (typeof entry !== 'object' || entry === null) return false;
				const period = entry as Record<string, unknown>;
				return isMonth(period.from) && PLANS.some((plan) => plan.id === period.id);
			})
			.sort((a, b) => a.from.localeCompare(b.from));
	} catch {
		return [];
	}
}

/**
 * The plan in force, and the ones before it.
 *
 * A year of usage rarely sat on one plan, so a single monthly figure would
 * misprice every month before the last upgrade. The history is what the user
 * remembers signing up for — nothing on the machine records it, so it cannot be
 * derived and has to be told.
 */
class Plan {
	id = $state<PlanId>(readStoredPlan());
	/** Sorted by start month, oldest first. */
	periods = $state<PlanPeriod[]>(readStoredHistory());
	/** What the plan actually costs here; the published figure is USD, and a
	 *  subscription bought elsewhere is billed in another currency at another
	 *  rate. */
	prices = $state<Partial<Record<PlanId, number>>>(readStoredPrices());

	monthlyOf(id: PlanId): number {
		return this.prices[id] ?? PLANS.find((plan) => plan.id === id)?.monthly ?? 0;
	}

	get monthly(): number {
		return this.monthlyOf(this.id);
	}

	isPriced(id: PlanId): boolean {
		return this.prices[id] !== undefined;
	}

	setPrice(id: PlanId, value: number): void {
		this.prices = { ...this.prices, [id]: value };
		localStorage.setItem(PLAN_PRICE_KEY, JSON.stringify(this.prices));
	}

	resetPrices(): void {
		this.prices = {};
		localStorage.removeItem(PLAN_PRICE_KEY);
	}

	set(next: PlanId): void {
		this.id = next;
		localStorage.setItem(PLAN_KEY, next);
	}

	/** The plan that was running in a given `YYYY-MM`; the current one if the
	 *  month predates every recorded change. */
	at(month: string): PlanId {
		let current: PlanId | null = null;
		for (const period of this.periods) {
			if (period.from <= month) current = period.id;
		}
		return current ?? this.id;
	}

	monthlyAt(month: string): number {
		return this.monthlyOf(this.at(month));
	}

	/** One plan per start month: setting the same month again corrects it. */
	record(from: string, id: PlanId): void {
		const rest = this.periods.filter((period) => period.from !== from);
		this.periods = [...rest, { from, id }].sort((a, b) => a.from.localeCompare(b.from));
		this.save();
	}

	forget(from: string): void {
		this.periods = this.periods.filter((period) => period.from !== from);
		this.save();
	}

	private save(): void {
		localStorage.setItem(PLAN_HISTORY_KEY, JSON.stringify(this.periods));
	}
}

export const plan = new Plan();
