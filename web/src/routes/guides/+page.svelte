<script lang="ts">
	import Seo from '#lib/components/Seo.svelte';
	import SectionHeader from '#lib/components/SectionHeader.svelte';
	import { guides } from '#lib/guides';
	import { site } from '#lib/site';

	const title = 'CSV guides for Mac and beyond';
	const description =
		'Checked guides for CSV problems on Mac: large files, Numbers limits, semicolon files, garbled encoding, and Shopify product CSVs.';

	const jsonLd = [
		{
			'@context': 'https://schema.org',
			'@type': 'CollectionPage',
			name: title,
			url: `${site.url}/guides/`,
			description,
			hasPart: guides.map((item) => ({
				'@type': 'WebPage',
				name: item.title,
				url: `${site.url}/guides/${item.slug}/`
			}))
		},
		{
			'@context': 'https://schema.org',
			'@type': 'BreadcrumbList',
			itemListElement: [
				{ '@type': 'ListItem', position: 1, name: 'Home', item: `${site.url}/` },
				{ '@type': 'ListItem', position: 2, name: 'Guides', item: `${site.url}/guides/` }
			]
		}
	];
</script>

<Seo {title} {description} path="/guides/" {jsonLd} />

<section class="section section--tight page-head">
	<div class="shell">
		<nav class="crumbs mono" aria-label="Breadcrumb">
			<a href="/">Home</a><span aria-hidden="true">/</span><span>Guides</span>
		</nav>
		<SectionHeader
			as="h1"
			index="GD"
			kicker="Guides"
			title="Practical CSV guides."
			lede="Each guide lists every real option, what it is good at, what it costs, and where TabTool fits. Facts and commands are checked against current documentation."
		/>
	</div>
</section>

<section class="section section--tight">
	<div class="shell">
		<ul class="guide-list">
			{#each guides as item (item.slug)}
				<li>
					<a class="guide-card" href={`/guides/${item.slug}/`}>
						<span class="guide-card__meta mono">Checked {item.checked}</span>
						<span class="guide-card__name">{item.title}</span>
						<span class="guide-card__summary">{item.lede}</span>
						<span class="guide-card__arrow" aria-hidden="true">Read →</span>
					</a>
				</li>
			{/each}
		</ul>
	</div>
</section>

<style>
	.page-head {
		padding-bottom: 0;
	}

	.guide-list {
		display: grid;
		gap: 1px;
		background: var(--line);
		border: 1px solid var(--line);
		list-style: none;
	}

	.guide-card {
		display: flex;
		flex-direction: column;
		gap: 10px;
		padding: clamp(22px, 2.6vw, 32px);
		background: var(--paper);
		text-decoration: none;
		transition: background 0.16s ease;
	}

	.guide-card:hover {
		background: var(--paper-raised);
		text-decoration: none;
	}

	.guide-card__meta {
		color: var(--muted);
		font-size: 11px;
		letter-spacing: 0.1em;
		text-transform: uppercase;
	}

	.guide-card__name {
		font-family: var(--serif);
		font-size: clamp(1.7rem, 2.8vw, 2.3rem);
		line-height: 1.05;
		letter-spacing: -0.015em;
	}

	.guide-card__summary {
		font-size: 14.5px;
		line-height: 1.5;
		color: var(--ink-soft);
		max-width: 60ch;
	}

	.guide-card__arrow {
		font-family: var(--mono);
		font-size: 12px;
		color: var(--accent);
	}
</style>
