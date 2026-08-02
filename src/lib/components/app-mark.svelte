<script lang="ts">
	type Props = { class?: string };
	let { class: className = 'size-5' }: Props = $props();
</script>

<!--
  Three sliders: the app administers what Claude Code stores rather than only
  charting it, and a control surface says that where the old ascending bars said
  "usage over time". Drawn from rects and circles so it stays crisp at 16px.
-->
<svg
	class={className}
	viewBox="0 0 24 24"
	fill="none"
	stroke="currentColor"
	stroke-width="2"
	stroke-linecap="round"
	aria-hidden="true"
>
	<path d="M4 6h6M14 6h6M4 12h12M20 12h0M4 18h3M11 18h9" />
	<circle cx="12" cy="6" r="2" fill="currentColor" stroke="none" />
	<circle cx="18" cy="12" r="2" fill="currentColor" stroke="none" />
	<circle cx="9" cy="18" r="2" fill="currentColor" stroke="none" />
</svg>
