<script lang="ts">
	import { comparisons } from '#lib/comparisons';
</script>

<ul class="compare">
	{#each comparisons as item (item.slug)}
		<li>
			<a class="compare__row" href={`/vs/${item.slug}/`}>
				<span class="compare__name">{item.name}</span>
				<span class="compare__sum">{item.summary}</span>
				<span class="compare__arrow" aria-hidden="true">→</span>
			</a>
		</li>
	{/each}
</ul>

<style>
	.compare {
		list-style: none;
		border-top: 1px solid var(--line);
	}

	.compare__row {
		display: grid;
		grid-template-columns: minmax(180px, 0.75fr) minmax(0, 1.8fr) 24px;
		gap: 24px;
		align-items: baseline;
		padding-block: 20px;
		border-bottom: 1px solid var(--line);
		text-decoration: none;
		transition: background 0.15s ease, padding 0.15s ease;
	}

	.compare__row:hover {
		background: var(--paper-raised);
		text-decoration: none;
		padding-inline: 12px;
	}

	.compare__name {
		font-family: var(--serif);
		font-size: clamp(1.3rem, 2vw, 1.6rem);
		letter-spacing: -0.01em;
	}

	.compare__sum {
		font-size: 14.5px;
		color: var(--muted);
		line-height: 1.5;
	}

	.compare__arrow {
		font-family: var(--mono);
		color: var(--accent);
		text-align: right;
	}

	@media (max-width: 760px) {
		.compare__row {
			grid-template-columns: 1fr 24px;
			gap: 4px 12px;
		}
		.compare__sum {
			grid-column: 1 / -1;
		}
		.compare__arrow {
			grid-row: 1;
			grid-column: 2;
		}
	}
</style>
