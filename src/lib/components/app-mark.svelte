<script lang="ts">
	type Props = { class?: string };
	let { class: className = 'size-5' }: Props = $props();
</script>

<!--
  A shield with a sun behind it: the app guards what Claude Code leaves behind,
  and the rays say the point is to see it rather than to lock it away.

  Strokes in currentColor with the shield filled in the background colour, so it
  reads the same on a light and a dark rail without a second asset — and the sun
  stays visible instead of merging into the shield at 16px.
-->
<svg
	class={className}
	viewBox="0 0 24 24"
	fill="none"
	stroke="currentColor"
	stroke-width="1.5"
	stroke-linecap="round"
	stroke-linejoin="round"
	aria-hidden="true"
>
	<!-- Rays above the horizon only: the shield hides the rest, and drawing them
	     left orphaned dashes beside it. -->
	<g opacity="0.8">
		<path d="M12 1.3v1.7M6.9 3.4 7.9 4.8M17.1 3.4 16.1 4.8M3.2 7.6l1.6.6M20.8 7.6l-1.6.6" />
	</g>
	<circle cx="12" cy="8.4" r="3.4" />
	<path
		d="M4.9 10.6h14.2v3.8c0 3.6-2.9 6.7-7.1 8.2-4.2-1.5-7.1-4.6-7.1-8.2z"
		fill="var(--background)"
	/>
</svg>
