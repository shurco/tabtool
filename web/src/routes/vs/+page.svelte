<script lang="ts">
	import Seo from '#lib/components/Seo.svelte';
	import SectionHeader from '#lib/components/SectionHeader.svelte';
	import { comparisons } from '#lib/comparisons';
	import { site } from '#lib/site';

	const title = 'TabTool compared with five CSV editors';
	const description =
		'Feature, price, platform, and privacy comparisons with TableTool, Modern CSV, CSViewer, Tablecruncher, and the original open-source TableTool.';

	const jsonLd = [
		{
			'@context': 'https://schema.org',
			'@type': 'CollectionPage',
			name: title,
			url: `${site.url}/vs/`,
			description,
			hasPart: comparisons.map((item) => ({
				'@type': 'WebPage',
				name: `TabTool vs ${item.name}`,
				url: `${site.url}/vs/${item.slug}/`
			}))
		},
		{
			'@context': 'https://schema.org',
			'@type': 'BreadcrumbList',
			itemListElement: [
				{ '@type': 'ListItem', position: 1, name: 'Home', item: `${site.url}/` },
				{ '@type': 'ListItem', position: 2, name: 'Comparisons', item: `${site.url}/vs/` }
			]
		}
	];
</script>

<Seo {title} {description} path="/vs/" {jsonLd} />

<section class="section section--tight page-head">
	<div class="shell">
		<nav class="crumbs mono" aria-label="Breadcrumb">
			<a href="/">Home</a><span aria-hidden="true">/</span><span>Comparisons</span>
		</nav>
		<SectionHeader
			as="h1"
			index="VS"
			kicker="Comparisons"
			title="Five editors, five honest pages."
			lede="Each page has the same shape: a feature table, the cases where the other app is better, and the cases where TabTool is. Vendor facts were checked on 9 November 2026."
		/>
	</div>
</section>

<section class="section section--tight">
	<div class="shell">
		<ul class="vs-list">
			{#each comparisons as item (item.slug)}
				<li>
					<a class="vs-card" href={`/vs/${item.slug}/`}>
						<span class="vs-card__vendor mono">{item.vendor}</span>
						<span class="vs-card__name">vs {item.name}</span>
						<span class="vs-card__summary">{item.summary}</span>
						<span class="vs-card__meta mono">{item.license} · {item.platforms}</span>
						<span class="vs-card__arrow" aria-hidden="true">Read →</span>
					</a>
				</li>
			{/each}
		</ul>
		<p class="note">
			Missing a tool you use? <a href={`${site.github}/issues`} rel="noopener">Open an issue</a> and
			suggest it. Comparisons are updated when a vendor changes something material.
		</p>
	</div>
</section>

<style>
	.page-head {
		padding-bottom: 0;
	}

	.crumbs {
		display: flex;
		gap: 8px;
		margin-bottom: 32px;
		color: var(--muted);
	}

	.crumbs a {
		text-decoration: none;
	}

	.crumbs a:hover {
		text-decoration: underline;
		color: var(--ink);
	}

	.vs-list {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 1px;
		background: var(--line);
		border: 1px solid var(--line);
		list-style: none;
	}

	.vs-card {
		display: flex;
		flex-direction: column;
		gap: 10px;
		height: 100%;
		padding: clamp(22px, 2.6vw, 32px);
		background: var(--paper);
		text-decoration: none;
		transition: background 0.16s ease;
	}

	.vs-card:hover {
		background: var(--paper-raised);
		text-decoration: none;
	}

	.vs-card__vendor {
		color: var(--muted);
		font-size: 11px;
		letter-spacing: 0.1em;
		text-transform: uppercase;
	}

	.vs-card__name {
		font-family: var(--serif);
		font-size: clamp(1.7rem, 2.8vw, 2.3rem);
		line-height: 1.05;
		letter-spacing: -0.015em;
	}

	.vs-card__summary {
		font-size: 14.5px;
		line-height: 1.5;
		color: var(--ink-soft);
		max-width: 46ch;
	}

	.vs-card__meta {
		margin-top: auto;
		padding-top: 12px;
		color: var(--muted);
		font-size: 11px;
	}

	.vs-card__arrow {
		font-family: var(--mono);
		font-size: 12px;
		color: var(--accent);
	}

	.note {
		margin-top: 20px;
		font-size: 13.5px;
		color: var(--muted);
	}

	.note a {
		color: var(--accent);
	}

	@media (max-width: 760px) {
		.vs-list {
			grid-template-columns: 1fr;
		}
	}
</style>
