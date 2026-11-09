<script lang="ts">
	import Seo from '#lib/components/Seo.svelte';
	import SectionHeader from '#lib/components/SectionHeader.svelte';
	import StoreBadges from '#lib/components/StoreBadges.svelte';
	import { site } from '#lib/site';

	const title = 'Press kit — TabTool, a native CSV editor for macOS and iOS';
	const description =
		'Boilerplate, fast facts, screenshots, the app icon, and contact details for writing about TabTool, a free open-source CSV editor for macOS and iOS.';

	const facts = [
		['Name', 'TabTool'],
		['What it is', 'A native CSV, TSV, and plain-text editor for macOS and iOS'],
		['Price', 'Free on the App Store'],
		['License', 'MIT, source on GitHub'],
		['Platforms', 'macOS 14+, iOS 17+ (iPhone and iPad)'],
		['Largest tested file', '5 GiB, 77.7 million rows, indexed in 2.7 seconds in the release app'],
		['Memory after indexing', 'About 91 MB'],
		['Privacy', 'Sandboxed, no network entitlement, no analytics, no account'],
		['Languages', '11 interface languages'],
		['Developer', 'Dmitry Shurco'],
		['App Store', 'https://apps.apple.com/app/tabtool-csv-editor/id6827919069']
	];
</script>

<Seo {title} {description} path="/press/" />

<section class="section section--tight page-head">
	<div class="shell">
		<nav class="crumbs mono" aria-label="Breadcrumb">
			<a href="/">Home</a><span aria-hidden="true">/</span><span>Press kit</span>
		</nav>
		<SectionHeader
			as="h1"
			index="PR"
			kicker="Press kit"
			title="TabTool in one page."
			lede="Everything needed to write about the app accurately: boilerplate, measured facts, assets, and the line between a CSV editor and a spreadsheet."
		/>
	</div>
</section>

<section class="section section--tight">
	<div class="shell split">
		<div>
			<p class="eyebrow">Boilerplate</p>
			<p class="prose">
				TabTool is a free, open-source CSV editor for macOS 14+ and iOS 17+. It opens CSV, TSV,
				and plain text files, detects the encoding, separator, quote style, decimal mark, and
				header row from the bytes, and lets you edit cells, rows, and columns. Large files are
				memory-mapped and indexed in the background: a tested 5 GiB file with 77.7 million rows
				was fully indexed 2.7 seconds after opening in the release app. The app is sandboxed with no network
				entitlement, no analytics, and no account.
			</p>
		</div>
		<div>
			<p class="eyebrow">What it is not</p>
			<p class="prose">
				TabTool is not a spreadsheet. It has no formulas, styles, charts, pivot tables, or XLSX
				support. It is a focused editor for delimited text, and it says so in the app and on the
				site. Please do not describe it as an Excel replacement.
			</p>
			<StoreBadges placement="about" />
		</div>
	</div>
</section>

<section class="section section--tight">
	<div class="shell">
		<p class="eyebrow">Fast facts</p>
		<dl class="facts">
			{#each facts as [label, value] (label)}
				<div>
					<dt>{label}</dt>
					<dd>{value}</dd>
				</div>
			{/each}
		</dl>
	</div>
</section>

<section class="section section--tight">
	<div class="shell">
		<p class="eyebrow">Assets</p>
		<ul class="assets">
			<li>
				<a href="/img/app-icon.png">App icon</a>
				<span class="mono">512 × 512 PNG</span>
			</li>
			<li>
				<a href="/img/og.png">Open Graph image</a>
				<span class="mono">1200 × 630 PNG</span>
			</li>
			<li>
				<a href="/img/app-grid.webp">macOS grid with emoji</a>
				<span class="mono">2400 × 1500 WebP</span>
			</li>
			<li>
				<a href="/img/app-columns.webp">macOS column actions</a>
				<span class="mono">2400 × 1500 WebP</span>
			</li>
			<li>
				<a href="/img/app-large.webp">macOS large-file mode</a>
				<span class="mono">2400 × 1500 WebP</span>
			</li>
			<li>
				<a href="/img/app-ios-grid.png">iPhone grid</a>
				<span class="mono">1320 × 2868 PNG</span>
			</li>
			<li>
				<a href="/img/app-ipad-grid.png">iPad grid</a>
				<span class="mono">2064 × 2752 PNG</span>
			</li>
			<li>
				<a href="/media/tabtool-demo.mp4">Demo video</a>
				<span class="mono">MP4, 1400 × 644</span>
			</li>
		</ul>
		<p class="note mono">
			All screenshots are real captures of the app with the public emoji data set. No composited UI. The MIT
			notice is bundled with the app.
		</p>
	</div>
</section>

<section class="section section--tight">
	<div class="shell split">
		<div>
			<p class="eyebrow">Contact</p>
			<p class="prose">
				For press inquiries, review assets, or corrections, email
				<a href="mailto:press@shurco.app">press@shurco.app</a>. For bugs and feature requests,
				open an issue at <a href={site.github} rel="noopener">github.com/shurco/tabtool</a>.
			</p>
		</div>
		<div>
			<p class="eyebrow">Links</p>
			<ul class="links">
				<li><a href={site.appStore.ios} rel="noopener">App Store listing</a></li>
				<li><a href={site.appStore.mac} rel="noopener">Mac App Store listing</a></li>
				<li><a href={site.github} rel="noopener">Source code</a></li>
				<li><a href="/download/">Download page</a></li>
				<li><a href="/faq/">FAQ</a></li>
			</ul>
		</div>
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

	.split {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: clamp(28px, 5vw, 64px);
	}

	.prose {
		margin-top: 16px;
		max-width: 60ch;
		font-size: 16px;
		line-height: 1.7;
		color: var(--ink-soft);
	}

	.prose a {
		color: var(--accent);
	}

	.facts {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 1px;
		margin-top: 20px;
		background: var(--line);
		border: 1px solid var(--line);
	}

	.facts div {
		padding: 18px 20px;
		background: var(--paper);
	}

	.facts dt {
		font-family: var(--mono);
		font-size: 11px;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		color: var(--muted);
	}

	.facts dd {
		margin: 8px 0 0;
		font-size: 15px;
		line-height: 1.45;
		overflow-wrap: anywhere;
	}

	.assets {
		margin-top: 20px;
		list-style: none;
		border-top: 1px solid var(--line);
	}

	.assets li {
		display: flex;
		flex-wrap: wrap;
		justify-content: space-between;
		gap: 8px 24px;
		padding-block: 14px;
		border-bottom: 1px solid var(--line);
		font-size: 15px;
	}

	.assets a {
		color: var(--accent);
	}

	.assets .mono,
	.note {
		color: var(--muted);
	}

	.note {
		margin-top: 18px;
		max-width: 70ch;
		line-height: 1.6;
	}

	.links {
		margin-top: 16px;
		display: grid;
		gap: 9px;
		list-style: none;
		font-size: 15px;
	}

	.links a {
		color: var(--accent);
	}

	@media (max-width: 820px) {
		.split,
		.facts {
			grid-template-columns: 1fr;
		}
	}
</style>
