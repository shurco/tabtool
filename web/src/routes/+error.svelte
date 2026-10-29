<script lang="ts">
	import Seo from '#lib/components/Seo.svelte';
	import { page } from '$app/state';
</script>

<Seo
	title="Page not found — TabTool"
	description="That page does not exist. Start from the overview, the comparisons, or the FAQ."
	path="/404/"
/>

<section class="section">
	<div class="shell">
		<p class="eyebrow">Error {page.status}</p>
		<h1>This row is empty.</h1>
		<p class="lede">
			The page you asked for is not here. The overview, the comparisons, and the FAQ all are.
		</p>
		<p class="links mono">
			<a href="/">Overview →</a>
			<a href="/vs/">Comparisons →</a>
			<a href="/faq/">FAQ →</a>
		</p>
	</div>
</section>

<style>
	h1 {
		margin-top: 16px;
		font-size: clamp(2.6rem, 7vw, 5rem);
	}

	.lede {
		margin-top: 20px;
		max-width: 48ch;
	}

	.links {
		display: flex;
		flex-wrap: wrap;
		gap: 24px;
		margin-top: 28px;
	}

	.links a {
		color: var(--accent);
	}
</style>
