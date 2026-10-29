<script lang="ts">
	import { stats } from '#lib/content';
</script>

<div class="stats" role="list">
	{#each stats as stat (stat.label)}
		<div class="stat" role="listitem">
			<p class="stat__value">{stat.value}</p>
			<p class="stat__label">{stat.label}</p>
			<p class="stat__note">{stat.note}</p>
		</div>
	{/each}
</div>

<style>
	.stats {
		display: grid;
		grid-template-columns: repeat(5, minmax(0, 1fr));
		border-top: 1px solid var(--line);
		border-bottom: 1px solid var(--line);
	}

	.stat {
		padding: 22px 20px 24px;
		border-left: 1px solid var(--line);
	}

	.stat:first-child {
		border-left: 0;
		padding-left: 0;
	}

	.stat__value {
		font-family: var(--serif);
		font-size: clamp(1.9rem, 3.4vw, 2.9rem);
		line-height: 1;
		letter-spacing: -0.02em;
		font-variant-numeric: tabular-nums;
	}

	.stat__label {
		margin-top: 12px;
		font-family: var(--mono);
		font-size: 10.5px;
		line-height: 1.4;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		color: var(--ink);
	}

	.stat__note {
		margin-top: 6px;
		font-size: 12.5px;
		line-height: 1.45;
		color: var(--muted);
	}

	@media (max-width: 900px) {
		.stats {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}
		.stat {
			padding-inline: 16px;
			border-top: 1px solid var(--line);
		}
		.stat:nth-child(odd) {
			border-left: 0;
			padding-left: 0;
		}
		.stat:nth-child(-n + 2) {
			border-top: 0;
		}
	}

	@media (max-width: 520px) {
		.stat__value {
			font-size: 1.9rem;
		}
	}
</style>
