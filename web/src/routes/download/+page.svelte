<script lang="ts">
	import Seo from '#lib/components/Seo.svelte';
	import SectionHeader from '#lib/components/SectionHeader.svelte';
	import Roadmap from '#lib/components/Roadmap.svelte';
	import OpenSource from '#lib/components/OpenSource.svelte';
	import { site } from '#lib/site';

	const title = 'Download TabTool — macOS, iOS, Windows, Linux, Android';
	const description =
		'TabTool 1.0 for macOS 14+ and iOS 17+ is in development. Windows, Linux, and Android are planned. See what is available, what is not, and how to build from source.';

	const jsonLd = [
		{
			'@context': 'https://schema.org',
			'@type': 'BreadcrumbList',
			itemListElement: [
				{ '@type': 'ListItem', position: 1, name: 'Home', item: `${site.url}/` },
				{ '@type': 'ListItem', position: 2, name: 'Download', item: `${site.url}/download/` }
			]
		}
	];
</script>

<Seo {title} {description} path="/download/" {jsonLd} />

<section class="section section--tight page-head">
	<div class="shell">
		<nav class="crumbs mono" aria-label="Breadcrumb">
			<a href="/">Home</a><span aria-hidden="true">/</span><span>Download</span>
		</nav>
		<SectionHeader
			as="h1"
			index="DL"
			kicker="Availability"
			title="Nothing to install yet. Here is the honest status."
			lede="The macOS and iOS apps are feature-complete for 1.0, but the release checklist, signing identity, and store listings are still open. The source builds today."
		/>
	</div>
</section>

<section class="section section--tight">
	<div class="shell">
		<div class="download">
			<article class="card">
				<p class="eyebrow">macOS</p>
				<h2>macOS 14 or later</h2>
				<p class="card__status mono">Status: in development</p>
				<p>
					The full editor: format detection, structural editing, sorting, large-file mode, export.
					Requires Xcode 27.1 to build. An App Store listing will follow the 1.0 release checklist.
				</p>
				<a class="btn btn--primary" href={site.github} rel="noopener">Build from source →</a>
			</article>

			<article class="card">
				<p class="eyebrow">iOS and iPadOS</p>
				<h2>iOS 17 or later</h2>
				<p class="card__status mono">Status: in development</p>
				<p>
					A touch-first editor with contextual row and column actions, filter cards, and the same
					CSV core. Run it in the simulator with a single script command.
				</p>
				<a class="btn btn--ghost" href={`${site.github}#build`} rel="noopener">Build instructions →</a>
			</article>
		</div>
	</div>
</section>

<section class="section section--tight">
	<div class="shell">
		<SectionHeader
			index="PL"
			kicker="Platforms"
			title="What exists, what is planned."
			lede="Windows, Linux, and Android are planned. No dates are promised until a build exists."
		/>
		<Roadmap />
	</div>
</section>

<section class="section section--tight">
	<div class="shell">
		<SectionHeader
			index="RQ"
			kicker="Requirements"
			title="What you need to build it."
			lede="The app is generated with XcodeGen and built with Xcode; there is no CocoaPods or SwiftPM dependency tree to fetch."
		/>
		<ul class="reqs">
			<li><span class="mono">macOS</span> 14 or later, iOS 17 or later</li>
			<li><span class="mono">Xcode</span> 27.1 with Swift 6.4</li>
			<li><span class="mono">XcodeGen</span> 2.46, installed with Homebrew</li>
			<li><span class="mono">Python 3</span> for the version script only</li>
			<li><span class="mono">Disk</span> space for a 5 GiB benchmark file if you run the large-file tests</li>
		</ul>
	</div>
</section>

<OpenSource />

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

	.download {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: clamp(18px, 2.4vw, 32px);
	}

	.card {
		display: flex;
		flex-direction: column;
		gap: 14px;
		padding: clamp(22px, 3vw, 34px);
		border: 1px solid var(--line);
		background: var(--paper-raised);
	}

	.card h2 {
		font-size: clamp(1.8rem, 3vw, 2.4rem);
	}

	.card__status {
		color: var(--warn);
	}

	.card p {
		font-size: 15px;
		line-height: 1.6;
		color: var(--ink-soft);
		max-width: 46ch;
	}

	.card .btn {
		margin-top: auto;
		align-self: flex-start;
	}

	.reqs {
		list-style: none;
		border-top: 1px solid var(--line);
	}

	.reqs li {
		display: flex;
		gap: 18px;
		align-items: baseline;
		padding-block: 14px;
		border-bottom: 1px solid var(--line);
		font-size: 15px;
		color: var(--ink-soft);
	}

	.reqs .mono {
		min-width: 110px;
		color: var(--ink);
	}

	@media (max-width: 760px) {
		.download {
			grid-template-columns: 1fr;
		}
		.reqs li {
			flex-direction: column;
			gap: 4px;
		}
	}
</style>
