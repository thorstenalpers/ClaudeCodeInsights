<script lang="ts">
	/**
	 * A small multi-series line chart.
	 *
	 * Hand-drawn SVG rather than a charting library: the app ships four charts,
	 * and a dependency that renders in canvas would neither inherit the theme
	 * colours nor survive the window's content policy.
	 */
	type Line = { key: string; values: (number | null)[]; dashed?: boolean };

	type Props = {
		labels: string[];
		series: Line[];
		format: (value: number) => string;
		height?: number;
		/** Fills below each line. Not stacked: the point is which one is higher. */
		area?: boolean;
	};

	let { labels, series, format, height = 200, area = false }: Props = $props();

	const CHART_VARS = ['--chart-1', '--chart-2', '--chart-3', '--chart-4', '--chart-5'];

	// A fixed viewBox with a stretched width: the chart scales to its container
	// without a resize observer, and the stroke keeps its weight through
	// vector-effect rather than being squashed with the geometry.
	const W = 640;
	const PAD = { top: 8, right: 8, bottom: 18, left: 44 };

	const peak = $derived(
		Math.max(
			1,
			...series.flatMap((line) => line.values.filter((value): value is number => value !== null))
		)
	);

	const plot = $derived({
		width: W - PAD.left - PAD.right,
		height: height - PAD.top - PAD.bottom
	});

	function x(index: number): number {
		if (labels.length <= 1) return PAD.left + plot.width / 2;
		return PAD.left + (index / (labels.length - 1)) * plot.width;
	}

	function y(value: number): number {
		return PAD.top + plot.height - (value / peak) * plot.height;
	}

	/** Runs of drawable points. A gap ends a run rather than being drawn
	 *  through, so a month with no plan recorded reads as unknown, not zero. */
	function runs(values: (number | null)[]): { index: number; value: number }[][] {
		const result: { index: number; value: number }[][] = [];
		let current: { index: number; value: number }[] = [];
		values.forEach((value, index) => {
			if (value === null) {
				if (current.length > 0) result.push(current);
				current = [];
				return;
			}
			current.push({ index, value });
		});
		if (current.length > 0) result.push(current);
		return result;
	}

	function path(values: (number | null)[]): string {
		return runs(values)
			.map((run) =>
				run
					.map(
						(point, position) =>
							`${position === 0 ? 'M' : 'L'}${x(point.index).toFixed(1)} ${y(point.value).toFixed(1)}`
					)
					.join(' ')
			)
			.join(' ');
	}

	/** The same runs, closed along the baseline. A single point has no width to
	 *  fill, so it is left to the dot to show. */
	function fill(values: (number | null)[]): string {
		const base = (PAD.top + plot.height).toFixed(1);
		return runs(values)
			.filter((run) => run.length > 1)
			.map((run) => {
				const line = run
					.map(
						(point, position) =>
							`${position === 0 ? 'M' : 'L'}${x(point.index).toFixed(1)} ${y(point.value).toFixed(1)}`
					)
					.join(' ');
				return `${line} L${x(run[run.length - 1].index).toFixed(1)} ${base} L${x(run[0].index).toFixed(1)} ${base} Z`;
			})
			.join(' ');
	}

	const ticks = $derived([0, peak / 2, peak]);
	let hovered = $state<number | null>(null);

	/** Nearest column to the pointer, in the svg's own coordinates. */
	function track(event: PointerEvent) {
		const box = (event.currentTarget as SVGSVGElement).getBoundingClientRect();
		if (box.width === 0 || labels.length === 0) return;
		const local = ((event.clientX - box.left) / box.width) * W;
		const step = plot.width / Math.max(1, labels.length - 1);
		hovered = Math.min(
			labels.length - 1,
			Math.max(0, Math.round((local - PAD.left) / (step || 1)))
		);
	}
</script>

<div class="flex flex-col gap-2">
	<svg
		viewBox="0 0 {W} {height}"
		class="w-full"
		style="height: {height}px"
		preserveAspectRatio="none"
		role="img"
		onpointermove={track}
		onpointerleave={() => (hovered = null)}
	>
		{#each ticks as tick (tick)}
			<line
				x1={PAD.left}
				x2={W - PAD.right}
				y1={y(tick)}
				y2={y(tick)}
				class="stroke-border"
				stroke-width="1"
				vector-effect="non-scaling-stroke"
			/>
			<text
				x={PAD.left - 6}
				y={y(tick) + 3}
				text-anchor="end"
				class="fill-muted-foreground text-[9px] tabular-nums"
			>
				{format(tick)}
			</text>
		{/each}

		{#each series as line, index (line.key)}
			{#if area}
				<path
					d={fill(line.values)}
					fill="var({CHART_VARS[index % CHART_VARS.length]})"
					fill-opacity="0.18"
					stroke="none"
				/>
			{/if}
			<path
				d={path(line.values)}
				fill="none"
				stroke="var({CHART_VARS[index % CHART_VARS.length]})"
				stroke-width="2"
				stroke-linejoin="round"
				stroke-linecap="round"
				stroke-dasharray={line.dashed ? '4 3' : undefined}
				vector-effect="non-scaling-stroke"
			/>
			{#each line.values as value, column (column)}
				{#if value !== null}
					<circle
						cx={x(column)}
						cy={y(value)}
						r={hovered === column ? 3.5 : 2}
						fill="var({CHART_VARS[index % CHART_VARS.length]})"
					/>
				{/if}
			{/each}
		{/each}

		{#if hovered !== null}
			<line
				x1={x(hovered)}
				x2={x(hovered)}
				y1={PAD.top}
				y2={PAD.top + plot.height}
				class="stroke-muted-foreground/40"
				stroke-width="1"
				vector-effect="non-scaling-stroke"
			/>
		{/if}
	</svg>

	<div class="flex w-full text-[10px] text-muted-foreground">
		{#each labels as label, index (label)}
			<span class="flex-1 truncate text-center">
				{index === 0 || index === labels.length - 1 || labels.length <= 12 ? label : ''}
			</span>
		{/each}
	</div>

	<div class="flex flex-wrap items-center gap-3 text-xs">
		{#each series as line, index (line.key)}
			<span class="flex items-center gap-1.5">
				<span
					class="h-0.5 w-4 shrink-0 rounded-full"
					style="background: var({CHART_VARS[index % CHART_VARS.length]})"
				></span>
				<span class="text-muted-foreground">{line.key}</span>
				{#if hovered !== null}
					{@const value = line.values[hovered]}
					{#if value !== null && value !== undefined}
						<span class="tabular-nums">{format(value)}</span>
					{/if}
				{/if}
			</span>
		{/each}
		{#if hovered !== null}
			<span class="text-muted-foreground tabular-nums">{labels[hovered]}</span>
		{/if}
	</div>
</div>
