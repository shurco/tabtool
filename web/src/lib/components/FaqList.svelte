<script lang="ts">
	import { faqs } from '#lib/content';

	let { limit, moreHref = '/faq/' }: { limit?: number; moreHref?: string } = $props();
	const shown = $derived(limit ? faqs.slice(0, limit) : faqs);
</script>

<div class="faq">
	{#each shown as item, i (item.q)}
		<details class="faq__item">
			<summary>
				<span class="faq__q">{item.q}</span>
				<span class="faq__icon" aria-hidden="true"></span>
			</summary>
			<p>{item.a}</p>
		</details>
	{/each}
</div>

{#if limit && limit < faqs.length}
	<p class="faq__more">
		<a href={moreHref}>All {faqs.length} questions →</a>
	</p>
{/if}

<style>
	.faq {
		border-top: 1px solid var(--line);
	}

	.faq__item {
		border-bottom: 1px solid var(--line);
	}

	.faq__item summary {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 24px;
		padding-block: 20px;
		cursor: pointer;
		list-style: none;
	}

	.faq__item summary::-webkit-details-marker {
		display: none;
	}

	.faq__q {
		font-size: clamp(1rem, 1.4vw, 1.1rem);
		font-weight: 550;
		letter-spacing: -0.005em;
	}

	.faq__icon {
		position: relative;
		flex: 0 0 auto;
		width: 12px;
		height: 12px;
		align-self: center;
	}

	.faq__icon::before,
	.faq__icon::after {
		content: '';
		position: absolute;
		inset: 5px 0 auto;
		height: 1.5px;
		background: var(--ink);
		transition: transform 0.2s ease;
	}

	.faq__icon::after {
		transform: rotate(90deg);
	}

	.faq__item[open] .faq__icon::after {
		transform: rotate(0deg);
	}

	.faq__item p {
		max-width: 68ch;
		padding-bottom: 24px;
		font-size: 15.5px;
		line-height: 1.65;
		color: var(--ink-soft);
		text-wrap: pretty;
	}

	.faq__more {
		margin-top: 20px;
		font-family: var(--mono);
		font-size: 12px;
	}

	.faq__more a {
		color: var(--accent);
	}
</style>
