<script lang="ts">
	type Props = { class?: string };
	let { class: className = 'size-5' }: Props = $props();
</script>

<!--
  The burst, and what it adds up to: three bars rising out of the corner.
  Insights is the counting, not the conversation.

  Black on its own white plate in both themes — the mark is the app's face,
  and a face that inverts with the rail is not one. The bars sit on a white
  patch punched out of the burst: at 16px two shapes that touch become one
  blob, and that gap is what keeps them apart.
-->
<svg class={className} viewBox="0 0 24 24" aria-hidden="true">
	<rect width="24" height="24" rx="5.25" fill="#FFFFFF" />

	<g fill="#000000" transform="translate(-1.5 -2.2) scale(0.8)">
		<path d="M14.00 9.80 L10.00 9.80 L11.32 1.90 L12.68 1.90 Z" />
		<path d="M14.97 11.86 L12.14 9.03 L18.66 4.38 L19.62 5.34 Z" />
		<path d="M14.20 14.00 L14.20 10.00 L22.10 11.32 L22.10 12.68 Z" />
		<path d="M12.14 14.97 L14.97 12.14 L17.29 16.33 L16.33 17.29 Z" />
		<path d="M10.00 14.20 L14.00 14.20 L12.68 22.10 L11.32 22.10 Z" />
		<path d="M9.03 12.14 L11.86 14.97 L5.34 19.62 L4.38 18.66 Z" />
		<path d="M9.80 10.00 L9.80 14.00 L1.90 12.68 L1.90 11.32 Z" />
		<path d="M11.86 9.03 L9.03 11.86 L4.38 5.34 L5.34 4.38 Z" />
	</g>
	<circle cx="8.1" cy="7.4" r="1.15" fill="#FFFFFF" />

	<rect x="11.6" y="11.4" width="10.3" height="10.2" rx="2.4" fill="#FFFFFF" />
	<g fill="#000000">
		<rect x="13.0" y="17.0" width="2.3" height="3.8" rx="0.6" />
		<rect x="16.0" y="15.0" width="2.3" height="5.8" rx="0.6" />
		<rect x="19.0" y="13.0" width="2.3" height="7.8" rx="0.6" />
	</g>
</svg>
