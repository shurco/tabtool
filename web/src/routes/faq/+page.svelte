<script lang="ts">
	import Seo from '#lib/components/Seo.svelte';
	import SectionHeader from '#lib/components/SectionHeader.svelte';
	import FaqList from '#lib/components/FaqList.svelte';
	import { faqs } from '#lib/content';
	import { site } from '#lib/site';

	const title = 'TabTool FAQ — pricing, file sizes, privacy, platforms';
	const description =
		'Answers about TabTool: is it free, when it ships, how large a CSV it opens, whether it uploads data, which encodings it supports, and why there are no formulas.';

	const jsonLd = [
		{
			'@context': 'https://schema.org',
			'@type': 'FAQPage',
			mainEntity: faqs.map((item) => ({
				'@type': 'Question',
				name: item.q,
				acceptedAnswer: { '@type': 'Answer', text: item.a }
			}))
		},
		{
			'@context': 'https://schema.org',
			'@type': 'BreadcrumbList',
			itemListElement: [
				{ '@type': 'ListItem', position: 1, name: 'Home', item: `${site.url}/` },
				{ '@type': 'ListItem', position: 2, name: 'FAQ', item: `${site.url}/faq/` }
			]
		}
	];
</script>

<Seo {title} {description} path="/faq/" {jsonLd} />

<section class="section section--tight page-head">
	<div class="shell">
		<nav class="crumbs mono" aria-label="Breadcrumb">
			<a href="/">Home</a><span aria-hidden="true">/</span><span>FAQ</span>
		</nav>
		<SectionHeader
			as="h1"
			index="FAQ"
			kicker="Straight answers"
			title="The questions that decide whether you try it."
			lede="If something is missing, open an issue on GitHub. The answers here match the current development build, not a future roadmap."
		/>
	</div>
</section>

<section class="section section--tight">
	<div class="shell">
		<FaqList />
	</div>
</section>

<section class="section section--tight">
	<div class="shell closing">
		<p class="eyebrow">Next</p>
		<h2>Still deciding?</h2>
		<p class="lede">
			Read how TabTool compares with TableTool, Modern CSV, CSViewer, and Tablecruncher, including
			the cases where the other app wins.
		</p>
		<p class="closing__links mono">
			<a href="/vs/">All comparisons →</a>
			<a href={site.github} rel="noopener">Source on GitHub →</a>
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
		color: var(--ink);
		text-decoration: underline;
	}

	.closing {
		display: grid;
		gap: 16px;
		max-width: 760px;
	}

	.closing h2 {
		font-size: clamp(2rem, 4vw, 3rem);
	}

	.closing__links {
		display: flex;
		flex-wrap: wrap;
		gap: 24px;
		margin-top: 8px;
	}

	.closing__links a {
		color: var(--accent);
	}
</style>
