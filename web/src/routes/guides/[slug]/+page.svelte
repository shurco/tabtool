<script lang="ts">
	import Seo from '#lib/components/Seo.svelte';
	import StoreBadges from '#lib/components/StoreBadges.svelte';
	import { site } from '#lib/site';

	let { data } = $props();
	const item = $derived(data.guide);
	const path = $derived(`/guides/${item.slug}/`);

	const jsonLd = $derived([
		{
			'@context': 'https://schema.org',
			'@type': 'BreadcrumbList',
			itemListElement: [
				{ '@type': 'ListItem', position: 1, name: 'Home', item: `${site.url}/` },
				{ '@type': 'ListItem', position: 2, name: 'Guides', item: `${site.url}/guides/` },
				{ '@type': 'ListItem', position: 3, name: item.title, item: `${site.url}${path}` }
			]
		},
		{
			'@context': 'https://schema.org',
			'@type': 'WebPage',
			name: item.metaTitle,
			url: `${site.url}${path}`,
			description: item.metaDescription,
			isPartOf: { '@type': 'WebSite', name: site.name, url: `${site.url}/` },
			about: { '@type': 'SoftwareApplication', name: 'TabTool', operatingSystem: 'macOS 14+, iOS 17+' }
		}
	]);
</script>

<Seo title={item.metaTitle} description={item.metaDescription} {path} {jsonLd} />

<section class="section section--tight">
	<div class="shell">
		<nav class="crumbs mono" aria-label="Breadcrumb">
			<a href="/">Home</a><span aria-hidden="true">/</span>
			<a href="/guides/">Guides</a><span aria-hidden="true">/</span>
			<span>{item.title}</span>
		</nav>

		<header class="guide-head">
			<p class="eyebrow">Guide · checked {item.checked}</p>
			<h1>{item.title}</h1>
			<p class="lede guide-head__lede">{item.lede}</p>
		</header>

		{#if item.image}
			<img
				class="guide-shot"
				src={item.image.src}
				srcset={item.image.srcset}
				sizes="(max-width: 1200px) 100vw, 1200px"
				width={item.image.width}
				height={item.image.height}
				alt={item.image.alt}
			/>
		{/if}

		<dl class="facts">
			{#each item.facts as fact (fact.label)}
				<div>
					<dt class="eyebrow">{fact.label}</dt>
					<dd>{fact.value}</dd>
				</div>
			{/each}
		</dl>

		<div class="prose-block">
			{#each item.problem as paragraph (paragraph)}
				<p>{paragraph}</p>
			{/each}
		</div>
	</div>
</section>

<section class="section section--tight">
	<div class="shell">
		<h2 class="h3">Your options</h2>
		<ol class="options">
			{#each item.options as option (option.name)}
				<li class="option">
					<h3 class="option__name">{option.name}</h3>
					<p>{option.summary}</p>
					<p><strong>Good for:</strong> {option.good}</p>
					<p><strong>Costs:</strong> {option.cost}</p>
					{#if option.code}
						<!-- svelte-ignore a11y_no_noninteractive_tabindex: keyboard users must be able to scroll the code block -->
						<pre class="option__code" tabindex="0"><code>{option.code}</code></pre>
					{/if}
					{#if option.link}
						<p class="option__link">
							<a href={option.link.url} rel="noopener">{option.link.label} →</a>
						</p>
					{/if}
				</li>
			{/each}
		</ol>
	</div>
</section>

{#if item.benchmark}
	<section class="section section--tight">
		<div class="shell">
			<h2 class="h3">How fast is TabTool on a huge file?</h2>
			<div class="table-wrap">
				<table>
					<thead>
						<tr>
							<th scope="col">What was measured</th>
							<th scope="col">Result</th>
							<th scope="col">Setup</th>
						</tr>
					</thead>
					<tbody>
						{#each item.benchmark as row (row.label)}
							<tr>
								<th scope="row">{row.label}</th>
								<td>{row.value}</td>
								<td>{row.note}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
			<p class="note">
				Method is in the
				<a href={`${site.github}#large-files`} rel="noopener">large files section on GitHub</a>; the
				script is
				<a href={`${site.github}/blob/HEAD/scripts/bench-large`} rel="noopener"><code>scripts/bench-large</code></a>.
			</p>
		</div>
	</section>
{/if}

<section class="section section--tight">
	<div class="shell">
		<p class="eyebrow">Try TabTool</p>
		<StoreBadges platforms={['mac']} placement="guide" campaign="guides" />
		<div class="guide-cta">
			<a class="btn btn--primary" href="/download/">Windows, Linux, Android</a>
			<a class="btn btn--ghost" href={site.github} rel="noopener">Star on GitHub</a>
		</div>

		<div class="sources">
			<p class="eyebrow">Sources · checked {item.checked}</p>
			<ul>
				{#each item.sources as source (source.url)}
					<li>
						<a href={source.url} rel="noopener">{source.label}</a>
						<span class="mono">{source.url.replace('https://', '')}</span>
					</li>
				{/each}
			</ul>
		</div>
	</div>
</section>

<style>
	.guide-head {
		max-width: 900px;
	}

	.guide-head h1 {
		margin-top: 16px;
		font-size: clamp(2.4rem, 5.5vw, 4.4rem);
		line-height: 1;
	}

	.guide-head__lede {
		margin-top: 22px;
		max-width: 60ch;
	}

	.guide-shot {
		display: block;
		max-width: 100%;
		height: auto;
		margin-top: 36px;
		border: 1px solid var(--line);
	}

	.h3 {
		font-family: var(--serif);
		font-size: clamp(1.7rem, 3vw, 2.2rem);
		line-height: 1.1;
	}

	.options {
		margin-top: 24px;
		display: grid;
		gap: 36px;
		list-style: none;
	}

	.option {
		max-width: 72ch;
		display: grid;
		gap: 10px;
		font-size: 15.5px;
		line-height: 1.6;
		color: var(--ink-soft);
	}

	.option__name {
		font-family: var(--serif);
		font-size: clamp(1.4rem, 2.4vw, 1.8rem);
		line-height: 1.15;
		color: var(--ink);
	}

	.option__code {
		overflow-x: auto;
		padding: 14px 16px;
		background: var(--paper-raised);
		border: 1px solid var(--line);
		font-family: var(--mono);
		font-size: 13px;
		line-height: 1.5;
		color: var(--ink);
	}

	.option__link a {
		color: var(--accent);
	}

	.table-wrap {
		--table-min: 640px;
		--th-width: 30%;
		margin-top: 20px;
	}

	.note {
		margin-top: 14px;
		font-size: 13.5px;
		color: var(--muted);
	}

	.note a {
		color: var(--accent);
	}

	.guide-cta {
		margin-top: 20px;
		display: flex;
		flex-wrap: wrap;
		gap: 10px;
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
		overflow-wrap: anywhere;
	}

	.sources a {
		color: var(--accent);
	}

	.sources .mono {
		color: var(--muted);
		font-size: 11px;
	}
</style>
