<script lang="ts">
	type Props = {
		suspect: boolean;
		hint: string;
		buttonLabel: string;
		onOpen: () => void;
	};
	let { suspect, hint, buttonLabel, onOpen }: Props = $props();
</script>

<div class="flex flex-col gap-2">
	{#if suspect}
		<p role="status" class="rounded-md border border-separator bg-field px-3 py-2 text-xs text-secondary">
			{hint}
		</p>
	{/if}
	<button
		type="button"
		onclick={onOpen}
		class="self-start text-xs text-accent underline"
	>
		{buttonLabel}
	</button>
</div>
