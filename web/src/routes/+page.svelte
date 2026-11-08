<script lang="ts">
	import Seo from '#lib/components/Seo.svelte';
	import SectionHeader from '#lib/components/SectionHeader.svelte';
	import Stats from '#lib/components/Stats.svelte';
	import Features from '#lib/components/Features.svelte';
	import VideoBlock from '#lib/components/VideoBlock.svelte';
	import Shots from '#lib/components/Shots.svelte';
	import OpenSource from '#lib/components/OpenSource.svelte';
	import Roadmap from '#lib/components/Roadmap.svelte';
	import CompareTeaser from '#lib/components/CompareTeaser.svelte';
	import FaqList from '#lib/components/FaqList.svelte';
	import Cta from '#lib/components/Cta.svelte';
	import StoreBadges from '#lib/components/StoreBadges.svelte';
	import { site } from '#lib/site';
	import { faqs } from '#lib/content';
	import { appStoreURL, trackStoreClick } from '#lib/track';

	const jsonLd = [
		{
			'@context': 'https://schema.org',
			'@type': 'SoftwareApplication',
			name: 'TabTool',
			applicationCategory: 'UtilitiesApplication',
			applicationSubCategory: 'CSV editor',
			operatingSystem: 'macOS 14+, iOS 17+',
			description: site.description,
			url: `${site.url}/`,
			license: 'https://opensource.org/license/mit',
			downloadUrl: site.appStore.ios,
			installUrl: site.appStore.ios,
			softwareVersion: '1.0',
			fileFormat: ['text/csv', 'text/tab-separated-values', 'text/plain'],
			screenshot: `${site.url}/img/mac-grid.webp`,
			sameAs: [site.github, site.appStore.ios],
			offers: {
				'@type': 'Offer',
				price: '0',
				priceCurrency: 'USD',
				url: site.appStore.ios,
				description: 'Free on the App Store for macOS and iOS; source code under the MIT license'
			},
			author: { '@type': 'Organization', name: 'TabTool contributors', url: site.github }
		},
		{
			'@context': 'https://schema.org',
			'@type': 'WebSite',
			name: site.name,
			url: `${site.url}/`,
			description: site.description,
			inLanguage: 'en'
		},
		{
			'@context': 'https://schema.org',
			'@type': 'FAQPage',
			mainEntity: faqs.map((item) => ({
				'@type': 'Question',
				name: item.q,
				acceptedAnswer: { '@type': 'Answer', text: item.a }
			}))
		}
	];
</script>

<Seo title={site.title} description={site.description} path="/" {jsonLd} />

<section class="hero">
	<div class="shell hero__inner">
		<div class="hero__copy">
			<p class="eyebrow">Native Swift · macOS + iOS · MIT</p>
			<h1>A CSV editor that doesn’t <em>flinch</em>.</h1>
			<p class="lede hero__lede">
				Open a 5 GiB file, fix a cell, write it back. TabTool indexes 77.7 million rows in 2.7
				seconds, keeps them in about 90 MB of memory, and never sends your data anywhere.
			</p>
			<StoreBadges placement="hero" />
			<div class="hero__actions">
				<a
					class="btn btn--primary"
					href={appStoreURL('ios', 'hero')}
					onclick={() => trackStoreClick('ios', 'hero')}
				>
					Download free <span class="btn__arrow" aria-hidden="true">↗</span>
				</a>
				<a class="btn btn--ghost" href={site.github} rel="noopener">Star on GitHub</a>
				<a class="btn btn--ghost" href="#performance">Watch it run</a>
			</div>
			<p class="hero__micro mono">Free · under 3 MB · no account · no tracking</p>
		</div>

		<figure class="hero__shot">
			<div class="hero__bar">
				<span class="mono">real capture</span>
				<span class="mono">macOS · orders.csv</span>
			</div>
			<img
				src="/img/mac-grid.webp"
				srcset="/img/mac-grid-1200.webp 1200w, /img/mac-grid.webp 2400w"
				sizes="(max-width: 960px) 100vw, 640px"
				width="2400"
				height="1502"
				alt="TabTool for macOS with a CSV of 1,200 orders open, showing the grid and the format bar."
				fetchpriority="high"
				decoding="async"
			/>
			<figcaption class="hero__caption mono">
				1,200 rows × 8 columns · UTF-8 · comma · detected on open
			</figcaption>
		</figure>
	</div>
</section>

<section class="section section--tight" aria-label="Measured numbers">
	<div class="shell">
		<Stats />
	</div>
</section>

<section class="section" id="features">
	<div class="shell">
		<SectionHeader
			index="02"
			kicker="What it does"
			title="A small editor with strong opinions."
			lede="No formulas, no charts, no plugins. TabTool does the boring parts of CSV work carefully: read the format, let you change the data, write the bytes back."
		/>
		<Features />
	</div>
</section>

<section class="section" id="performance">
	<div class="shell">
		<SectionHeader
			index="03"
			kicker="Performance"
			title="Two million rows, no spinner."
			lede="Large files are memory-mapped and indexed in the background, so the first rows appear immediately. This is an unedited capture of the current build scrolling a 109 MB file."
		/>
		<VideoBlock />
		<ul class="notes">
			<li>
				<span class="notes__value mono">2.7 s</span>
				<span>to index a 5 GiB file with 68.2 million rows on an Apple M4 Pro.</span>
			</li>
			<li>
				<span class="notes__value mono">~91 MB</span>
				<span>of memory with 77.7 million rows loaded in the release app.</span>
			</li>
			<li>
				<span class="notes__value mono">6.3 s</span>
				<span>to save the 5 GiB file with one edit, streamed with progress and cancel.</span>
			</li>
		</ul>
	</div>
</section>

<section class="section" id="screens">
	<div class="shell">
		<SectionHeader
			index="04"
			kicker="Screens"
			title="Every state is the real app."
			lede="The screenshots below come from the development build on 29 October 2026. No mockups, no composited UI."
		/>
		<Shots />
	</div>
</section>

<OpenSource />

<section class="section" id="roadmap">
	<div class="shell">
		<SectionHeader
			index="06"
			kicker="Roadmap"
			title="macOS and iOS now. The rest is planned."
			lede="TabTool 1.0 is a Mac and iPhone app. Windows, Linux, and Android are on the list, with no dates attached until real builds exist."
		/>
		<Roadmap />
	</div>
</section>

<section class="section" id="compare">
	<div class="shell">
		<SectionHeader
			index="07"
			kicker="Comparisons"
			title="Compared with the field."
			lede="Five separate pages, each with a feature table and a section on where the other app is the better choice."
		/>
		<CompareTeaser />
	</div>
</section>

<section class="section" id="faq">
	<div class="shell">
		<SectionHeader
			index="08"
			kicker="FAQ"
			title="Questions people actually ask."
			lede="Pricing, file sizes, privacy, platforms, and what TabTool deliberately does not do."
		/>
		<FaqList limit={6} />
	</div>
</section>

<Cta />

<style>
	.hero {
		padding-block: clamp(40px, 6vw, 88px) clamp(32px, 4vw, 56px);
		border-bottom: 1px solid var(--line);
	}

	.hero__inner {
		display: grid;
		grid-template-columns: minmax(0, 1.02fr) minmax(0, 1fr);
		gap: clamp(32px, 5vw, 72px);
		align-items: center;
	}

	.hero h1 {
		margin-top: 18px;
		font-size: clamp(3.1rem, 7.4vw, 6.4rem);
		line-height: 0.94;
		letter-spacing: -0.03em;
	}

	.hero h1 em {
		font-style: italic;
		color: var(--accent);
	}

	.hero__lede {
		margin-top: 24px;
		max-width: 46ch;
	}

	.hero__actions {
		display: flex;
		flex-wrap: wrap;
		gap: 10px;
		margin-top: 16px;
	}

	.hero__micro {
		margin-top: 18px;
		color: var(--muted);
	}

	.hero__shot {
		margin: 0;
		border: 1px solid var(--line);
		background: var(--paper-sunk);
		border-radius: var(--radius);
		padding: 10px 10px 0;
		box-shadow: var(--shadow);
	}

	.hero__bar {
		display: flex;
		justify-content: space-between;
		gap: 12px;
		padding: 2px 2px 10px;
		color: var(--muted);
	}

	.hero__shot img {
		width: 100%;
		border: 1px solid var(--line);
		border-bottom: 0;
		background: var(--paper-raised);
	}

	.hero__caption {
		padding: 10px 2px 12px;
		color: var(--muted);
		border-top: 1px solid var(--line);
		margin-top: 0;
	}

	.notes {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: 1px;
		margin-top: 32px;
		background: var(--line);
		border: 1px solid var(--line);
		list-style: none;
	}

	.notes li {
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding: 20px;
		background: var(--paper);
		font-size: 14px;
		color: var(--ink-soft);
	}

	.notes__value {
		font-size: 13px;
		color: var(--accent);
		letter-spacing: 0.04em;
	}

	@media (max-width: 960px) {
		.hero__inner {
			grid-template-columns: 1fr;
		}
		.hero__shot {
			order: 2;
		}
	}

	@media (max-width: 760px) {
		.notes {
			grid-template-columns: 1fr;
		}
	}
</style>
