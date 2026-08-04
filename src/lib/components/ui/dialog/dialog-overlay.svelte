<script lang="ts">
	import { Dialog as DialogPrimitive } from "bits-ui";
	import { cn } from "$lib/utils.js";

	let {
		ref = $bindable(null),
		class: className,
		...restProps
	}: DialogPrimitive.OverlayProps = $props();
</script>

<!-- Windows dims behind a dialog with Smoke: flat black at 30%, no blur. The
     blur that was here belongs to Acrylic, which Windows uses for flyouts. -->
<DialogPrimitive.Overlay
	bind:ref
	data-slot="dialog-overlay"
	class={cn("bg-black/30 duration-100 data-open:animate-in data-open:fade-in-0 data-closed:animate-out data-closed:fade-out-0 fixed inset-0 isolate z-50", className)}
	{...restProps}
/>
