<script lang="ts">
	import { platforms } from '#lib/content';
	import { site } from '#lib/site';
	import StoreBadges from '#lib/components/StoreBadges.svelte';
</script>

<ul class="roadmap">
	{#each platforms as platform (platform.name)}
		<li class="roadmap__row">
			<span class="roadmap__name">{platform.name}</span>
			<span class="roadmap__req mono">{platform.requirement}</span>
			<span class="roadmap__get">
				<StoreBadges platforms={[platform.key]} placement="platforms" size="sm" />
			</span>
			<span class="roadmap__note">{platform.note}</span>
		</li>
	{/each}
</ul>

<p class="roadmap__footnote">
	macOS and iOS install from the App Store. Windows, Linux, and Android builds are attached to each
	release on <a href={site.releases} rel="noopener">GitHub Releases</a>, with a SHA256SUMS file to
	verify the download. Release notes live there too.
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
		align-items: center;
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
