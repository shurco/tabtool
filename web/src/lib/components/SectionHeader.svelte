<script lang="ts">
	let {
		index,
		title,
		kicker,
		lede,
		as = 'h2'
	}: {
		index: string;
		title: string;
		kicker?: string;
		lede?: string;
		as?: 'h1' | 'h2';
	} = $props();
</script>

<div class="section-head">
	<span class="section-head__index" aria-hidden="true">{index}</span>
	<div class="section-head__body">
		{#if kicker}<p class="eyebrow">{kicker}</p>{/if}
		<svelte:element this={as} class="section-head__title">{title}</svelte:element>
		{#if lede}<p class="lede">{lede}</p>{/if}
	</div>
</div>
