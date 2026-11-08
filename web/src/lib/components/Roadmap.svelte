<script lang="ts">
	import { platforms } from '#lib/content';
</script>

<ul class="roadmap">
	{#each platforms as platform (platform.name)}
		<li class="roadmap__row" class:is-planned={platform.tone === 'planned'}>
			<span class="roadmap__name">{platform.name}</span>
			<span class="roadmap__req mono">{platform.requirement}</span>
			<span class="roadmap__status mono">
				<span class="roadmap__dot" aria-hidden="true"></span>{platform.status}
			</span>
			<span class="roadmap__note">{platform.note}</span>
		</li>
	{/each}
</ul>

<p class="roadmap__footnote">
	macOS and iOS are on the App Store. Other platforms are planned, not promised. No dates until a build exists. The repository is the source of
	truth.
</p>

<style>
	.roadmap {
		list-style: none;
		border-top: 1px solid var(--line);
	}

	.roadmap__row {
		display: grid;
		grid-template-columns: minmax(120px, 0.7fr) minmax(150px, 1fr) 170px minmax(200px, 1.6fr);
		gap: 16px;
		align-items: baseline;
		padding-block: 18px;
		border-bottom: 1px solid var(--line);
	}

	.roadmap__name {
		font-family: var(--serif);
		font-size: 1.5rem;
		letter-spacing: -0.01em;
	}

	.roadmap__req {
		color: var(--muted);
	}

	.roadmap__status {
		display: inline-flex;
		align-items: center;
		gap: 8px;
		font-size: 11px;
		letter-spacing: 0.08em;
		text-transform: uppercase;
	}

	.roadmap__dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--accent);
	}

	.is-planned .roadmap__dot {
		background: var(--warn);
	}

	.roadmap__note {
		font-size: 14px;
		color: var(--ink-soft);
	}

	.roadmap__footnote {
		margin-top: 18px;
		max-width: 70ch;
		font-size: 13.5px;
		color: var(--muted);
	}

	@media (max-width: 860px) {
		.roadmap__row {
			grid-template-columns: 1fr 1fr;
			gap: 6px 16px;
		}
		.roadmap__note {
			grid-column: 1 / -1;
		}
	}
</style>
