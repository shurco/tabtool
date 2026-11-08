<script lang="ts">
	import Seo from '#lib/components/Seo.svelte';
	import StoreBadges from '#lib/components/StoreBadges.svelte';
	import { comparisons } from '#lib/comparisons';
	import { site } from '#lib/site';
	import { appStoreURL, trackStoreClick } from '#lib/track';

	let { data } = $props();
	const item = $derived(data.comparison);
	const path = $derived(`/vs/${item.slug}/`);
	const title = $derived(item.metaTitle);
	const description = $derived(item.metaDescription);
	const others = $derived(comparisons.filter((c) => c.slug !== item.slug));

	const jsonLd = $derived([
		{
			'@context': 'https://schema.org',
			'@type': 'BreadcrumbList',
			itemListElement: [
				{ '@type': 'ListItem', position: 1, name: 'Home', item: `${site.url}/` },
				{ '@type': 'ListItem', position: 2, name: 'Comparisons', item: `${site.url}/vs/` },
				{
					'@type': 'ListItem',
					position: 3,
					name: `TabTool vs ${item.name}`,
					item: `${site.url}${path}`
				}
			]
		},
		{
			'@context': 'https://schema.org',
			'@type': 'WebPage',
			name: title,
			url: `${site.url}${path}`,
			description,
			isPartOf: { '@type': 'WebSite', name: site.name, url: `${site.url}/` },
			about: { '@type': 'SoftwareApplication', name: 'TabTool', operatingSystem: 'macOS 14+, iOS 17+' }
		}
	]);
</script>

<Seo {title} {description} {path} {jsonLd} />

<section class="section section--tight">
	<div class="shell">
		<nav class="crumbs mono" aria-label="Breadcrumb">
			<a href="/">Home</a><span aria-hidden="true">/</span>
			<a href="/vs/">Comparisons</a><span aria-hidden="true">/</span>
			<span>vs {item.name}</span>
		</nav>

		<header class="vs-head">
			<p class="eyebrow">Comparison · checked October 2026</p>
			<h1>TabTool vs <span class="vs-head__name">{item.name}</span></h1>
			<p class="lede vs-head__lede">{item.summary}</p>
		</header>

		<dl class="facts">
			<div>
				<dt class="eyebrow">Their license</dt>
				<dd>{item.license}</dd>
			</div>
			<div>
				<dt class="eyebrow">Their price</dt>
				<dd>{item.price}</dd>
			</div>
			<div>
				<dt class="eyebrow">Their platforms</dt>
				<dd>{item.platforms}</dd>
			</div>
		</dl>

		<div class="prose-block">
			{#each item.intro as paragraph}
				<p>{paragraph}</p>
			{/each}
		</div>
	</div>
</section>

<section class="section section--tight">
	<div class="shell">
		<div class="table-wrap">
			<table>
				<caption class="mono"
					>Vendor facts checked October 2026. Prices and features change; verify on
					<a href={item.site} rel="noopener">{item.site.replace('https://', '')}</a>.</caption
				>
				<thead>
					<tr>
						<th scope="col">Criteria</th>
						<th scope="col">TabTool</th>
						<th scope="col">{item.name}</th>
					</tr>
				</thead>
				<tbody>
					{#each item.rows as row (row.label)}
						<tr>
							<th scope="row">{row.label}</th>
							<td>{row.tabtool}</td>
							<td>{row.them}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
		<p class="table-hint mono">Swipe the table sideways to see the {item.name} column →</p>
	</div>
</section>

<section class="section section--tight">
	<div class="shell split">
		<div>
			<h2 class="h3">Where {item.name} is stronger</h2>
			<ul class="points points--them">
				{#each item.stronger as point}
					<li>{point}</li>
				{/each}
			</ul>
		</div>
		<div>
			<h2 class="h3">Where TabTool is different</h2>
			<ul class="points">
				{#each item.different as point}
					<li>{point}</li>
				{/each}
			</ul>
		</div>
	</div>
</section>

<section class="section section--tight">
	<div class="shell">
		<div class="verdict">
			<p class="eyebrow">Verdict</p>
			<p class="verdict__text">{item.verdict}</p>
		</div>

		<div class="sources">
			<p class="eyebrow">Sources</p>
			<ul>
				{#each item.sources as source}
					<li>
						<a href={source.url} rel="noopener">{source.label}</a>
						<span class="mono">{source.url.replace('https://', '')}</span>
					</li>
				{/each}
			</ul>
		</div>
	</div>
</section>

<section class="section section--tight">
	<div class="shell">
		<p class="eyebrow">Keep reading</p>
		<ul class="related">
			{#each others as other (other.slug)}
				<li><a href={`/vs/${other.slug}/`}>vs {other.name} →</a></li>
			{/each}
		</ul>
		<StoreBadges placement="compare" />
		<div class="vs-cta">
			<a
				class="btn btn--primary"
				href={appStoreURL('ios', 'compare')}
				onclick={() => trackStoreClick('ios', 'compare')}
			>
				Download free <span class="btn__arrow" aria-hidden="true">↗</span>
			</a>
			<a class="btn btn--ghost" href={site.github} rel="noopener">Star on GitHub</a>
		</div>
	</div>
</section>

<style>
	.crumbs {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
		margin-bottom: 36px;
		color: var(--muted);
	}

	.crumbs a {
		text-decoration: none;
	}

	.crumbs a:hover {
		text-decoration: underline;
		color: var(--ink);
	}

	.vs-head {
		max-width: 900px;
	}

	.vs-head h1 {
		margin-top: 16px;
		font-size: clamp(2.6rem, 6vw, 5rem);
		line-height: 0.98;
	}

	.vs-head__name {
		font-style: italic;
		color: var(--accent);
	}

	.vs-head__lede {
		margin-top: 22px;
		max-width: 60ch;
	}

	.facts {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: 1px;
		margin-top: 40px;
		background: var(--line);
		border: 1px solid var(--line);
	}

	.facts div {
		padding: 18px 20px;
		background: var(--paper);
	}

	.facts dd {
		margin: 8px 0 0;
		font-size: 14.5px;
		line-height: 1.5;
	}

	.prose-block {
		margin-top: 40px;
		max-width: 72ch;
		display: grid;
		gap: 16px;
		font-size: 16.5px;
		line-height: 1.7;
		color: var(--ink-soft);
	}

	.table-wrap {
		overflow-x: auto;
		border: 1px solid var(--line);
	}

	.table-hint {
		display: none;
		margin-top: 10px;
		color: var(--muted);
		font-size: 11px;
	}

	@media (max-width: 820px) {
		.table-hint {
			display: block;
		}
	}

	table {
		width: 100%;
		min-width: 720px;
		border-collapse: collapse;
		font-size: 14.5px;
	}

	caption {
		caption-side: top;
		text-align: left;
		padding: 14px 16px;
		color: var(--muted);
		border-bottom: 1px solid var(--line);
	}

	caption a {
		color: var(--accent);
	}

	th,
	td {
		text-align: left;
		vertical-align: top;
		padding: 14px 16px;
		border-bottom: 1px solid var(--line);
		line-height: 1.45;
	}

	thead th {
		font-family: var(--mono);
		font-size: 11px;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		color: var(--muted);
		font-weight: 500;
	}

	tbody th {
		font-weight: 550;
		width: 24%;
	}

	tbody tr:last-child th,
	tbody tr:last-child td {
		border-bottom: 0;
	}

	tbody td:first-of-type {
		background: var(--accent-wash);
	}

	.split {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: clamp(28px, 5vw, 64px);
	}

	.h3 {
		font-family: var(--serif);
		font-size: clamp(1.7rem, 3vw, 2.2rem);
		line-height: 1.1;
	}

	.points {
		margin-top: 20px;
		padding-left: 20px;
		display: grid;
		gap: 12px;
		font-size: 15px;
		line-height: 1.6;
		color: var(--ink-soft);
	}

	.points li::marker {
		color: var(--accent);
	}

	.points--them li::marker {
		color: var(--muted);
	}

	.verdict {
		border-top: 1px solid var(--line);
		padding-top: 28px;
	}

	.verdict__text {
		margin-top: 14px;
		max-width: 48ch;
		font-family: var(--serif);
		font-size: clamp(1.5rem, 2.7vw, 2.05rem);
		line-height: 1.3;
		letter-spacing: -0.01em;
	}

	.sources {
		margin-top: 44px;
	}

	.sources ul {
		margin-top: 14px;
		display: grid;
		gap: 8px;
		list-style: none;
	}

	.sources li {
		display: flex;
		flex-wrap: wrap;
		gap: 4px 14px;
		align-items: baseline;
		font-size: 14px;
	}

	.sources a {
		color: var(--accent);
	}

	.sources .mono {
		color: var(--muted);
		font-size: 11px;
	}

	.related {
		margin-top: 16px;
		display: flex;
		flex-wrap: wrap;
		gap: 10px 24px;
		list-style: none;
		font-size: 14px;
	}

	.related a {
		text-decoration: none;
		color: var(--ink-soft);
	}

	.related a:hover {
		color: var(--ink);
		text-decoration: underline;
	}

	.vs-cta {
		margin-top: 28px;
		display: flex;
		flex-wrap: wrap;
		gap: 10px;
	}

	@media (max-width: 820px) {
		.facts,
		.split {
			grid-template-columns: 1fr;
		}
	}
</style>
