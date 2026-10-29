<script lang="ts">
	import { features } from '#lib/content';
</script>

<ul class="features">
	{#each features as feature (feature.index)}
		<li class="feature" class:feature--wide={feature.wide}>
			<span class="feature__index" aria-hidden="true">{feature.index}</span>
			<h3>{feature.title}</h3>
			<p class="feature__body">{feature.body}</p>
			<ul class="feature__tags" aria-label="Details">
				{#each feature.tags as tag (tag)}
					<li>{tag}</li>
				{/each}
			</ul>
		</li>
	{/each}
</ul>

<style>
	.features {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		list-style: none;
		border-top: 1px solid var(--line);
		border-left: 1px solid var(--line);
	}

	.feature {
		position: relative;
		display: flex;
		flex-direction: column;
		gap: 12px;
		padding: clamp(22px, 3vw, 34px);
		border-right: 1px solid var(--line);
		border-bottom: 1px solid var(--line);
		background: var(--paper);
		transition: background 0.18s ease, color 0.18s ease;
	}

	.feature:hover {
		background: var(--ink);
		color: var(--paper);
	}

	.feature__index {
		font-family: var(--mono);
		font-size: 11px;
		letter-spacing: 0.12em;
		color: var(--muted);
	}

	.feature:hover .feature__index,
	.feature:hover .feature__body,
	.feature:hover .feature__tags li {
		color: color-mix(in srgb, var(--paper) 72%, transparent);
	}

	.feature h3 {
		font-size: clamp(1.15rem, 1.8vw, 1.45rem);
		max-width: 24ch;
	}

	.feature__body {
		font-size: 15px;
		line-height: 1.55;
		color: var(--ink-soft);
		max-width: 46ch;
		text-wrap: pretty;
	}

	.feature__tags {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		margin-top: auto;
		padding-top: 12px;
		list-style: none;
	}

	.feature__tags li {
		font-family: var(--mono);
		font-size: 10.5px;
		letter-spacing: 0.04em;
		color: var(--muted);
		border: 1px solid var(--line);
		border-radius: 2px;
		padding: 3px 7px;
	}

	.feature:hover .feature__tags li {
		border-color: color-mix(in srgb, var(--paper) 26%, transparent);
	}

	.feature--wide {
		grid-column: 1 / -1;
	}

	.feature--wide .feature__body {
		max-width: 92ch;
	}

	.feature--wide h3 {
		max-width: none;
		font-size: clamp(1.3rem, 2.2vw, 1.7rem);
	}

	@media (max-width: 760px) {
		.features {
			grid-template-columns: 1fr;
		}
		.feature--wide {
			grid-column: auto;
		}
	}
</style>
