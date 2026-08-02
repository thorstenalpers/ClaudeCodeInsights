<script lang="ts">
	type Props = { class?: string };
	let { class: className = 'size-5' }: Props = $props();
</script>

<!--
  An eight-pointed burst with one spoke cut short: the app is about reading what
  a model left behind, and the odd spoke is the figure that stands out.

  Filled wedges rather than strokes, in currentColor, so the mark keeps its
  taper on a light or a dark rail without a second asset and without thinning
  into hairlines at 16px. The eye is punched in the background colour so the
  centre stays open at every size.
-->
<svg class={className} viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
	<path d="M14.00 9.80 L10.00 9.80 L11.32 1.90 L12.68 1.90 Z" />
	<path d="M14.97 11.86 L12.14 9.03 L18.66 4.38 L19.62 5.34 Z" />
	<path d="M14.20 14.00 L14.20 10.00 L22.10 11.32 L22.10 12.68 Z" />
	<path d="M12.14 14.97 L14.97 12.14 L17.29 16.33 L16.33 17.29 Z" />
	<path d="M10.00 14.20 L14.00 14.20 L12.68 22.10 L11.32 22.10 Z" />
	<path d="M9.03 12.14 L11.86 14.97 L5.34 19.62 L4.38 18.66 Z" />
	<path d="M9.80 10.00 L9.80 14.00 L1.90 12.68 L1.90 11.32 Z" />
	<path d="M11.86 9.03 L9.03 11.86 L4.38 5.34 L5.34 4.38 Z" />
	<circle cx="12" cy="12" r="1.5" fill="var(--background)" />
</svg>
