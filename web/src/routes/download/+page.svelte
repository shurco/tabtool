<script lang="ts">
	import Seo from '#lib/components/Seo.svelte';
	import SectionHeader from '#lib/components/SectionHeader.svelte';
	import Roadmap from '#lib/components/Roadmap.svelte';
	import OpenSource from '#lib/components/OpenSource.svelte';
	import StoreBadges from '#lib/components/StoreBadges.svelte';
	import { site } from '#lib/site';

	const title = 'Download TabTool — free CSV editor for Mac, iOS, Windows, Linux, Android';
	const description =
		'TabTool 1.0 is free on the App Store for macOS 14+ and iOS 17+, and on GitHub Releases for Windows, Linux, and Android. Download it, or build it from source.';

	// File names follow .github/workflows/release.yml; see docs/release.md.
	const releaseBuilds = [
		{
			name: 'Windows',
			requirement: 'Windows 10 or 11',
			body: 'The desktop editor with document windows, large-file mode, and export. Unzip it and run tabtool.exe from the extracted folder; nothing is installed. The archive is not code-signed, so SmartScreen may ask once before the first launch.',
			files: ['windows-x86_64.zip', 'windows-aarch64.zip']
		},
		{
			name: 'Linux',
			requirement: 'glibc 2.35 or later',
			body: 'The same desktop editor. Extract the archive and run tabtool from the extracted folder. Needs X11 or Wayland and fontconfig; built for Ubuntu 22.04, Debian 12, Fedora 36, and newer.',
			files: ['linux-x86_64.tar.gz', 'linux-aarch64.tar.gz']
		},
		{
			name: 'Android',
			requirement: 'Android 8.0 or later',
			body: 'A touch editor that opens files from the system picker, a file manager, or a share sheet. Install the signed APK; Android asks to allow installs from your browser first.',
			files: ['android.apk']
		}
	];

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
			title="Get TabTool 1.0 for your platform."
			lede="Mac, iPhone, and iPad install from the App Store with one free download. Windows, Linux, and Android builds are on GitHub Releases. The source on GitHub builds every one of them."
		/>
	</div>
</section>

<section class="section section--tight">
	<div class="shell">
		<p class="mono download__micro">
			Free · no account · no tracking · 2.5 MB on iOS, 2.7 MB on macOS
		</p>
		<div class="download">
			<article class="card">
				<p class="eyebrow">macOS</p>
				<h2>macOS 14 or later</h2>
				<p class="card__status mono">Status: available</p>
				<p>
					The full editor: format detection, structural editing, sorting, large-file mode, export.
					The release build is 2.7 MB, written in pure Swift.
				</p>
				<StoreBadges platforms={['mac']} placement="download" />
				<a class="btn btn--ghost" href={site.github} rel="noopener">Build from source →</a>
			</article>

			<article class="card">
				<p class="eyebrow">iOS and iPadOS</p>
				<h2>iOS 17 or later</h2>
				<p class="card__status mono">Status: available</p>
				<p>
					A touch-first editor with contextual row and column actions, filter cards, and the same
					CSV core. The release build is 2.5 MB. Run it in the simulator with a single script
					command.
				</p>
				<StoreBadges platforms={['ios']} placement="download" />
				<a class="btn btn--ghost" href={`${site.github}#build`} rel="noopener">Build instructions →</a>
			</article>
		</div>
	</div>
</section>

<section class="section section--tight" id="releases">
	<div class="shell">
		<p class="mono download__micro">Free · no account · no tracking · MIT · x64 and ARM</p>
		<div class="download download--three">
			{#each releaseBuilds as build (build.name)}
				<article class="card">
					<p class="eyebrow">{build.name}</p>
					<h2>{build.requirement}</h2>
					<p class="card__status mono">Status: available</p>
					<p>{build.body}</p>
					<ul class="card__files mono">
						{#each build.files as file (file)}
							<li>tabtool-&lt;version&gt;-{file}</li>
						{/each}
					</ul>
					<a class="btn btn--primary" href={site.releases} rel="noopener">
						Download from GitHub <span class="btn__arrow" aria-hidden="true">↗</span>
					</a>
				</article>
			{/each}
		</div>
		<p class="download__note">
			Every release lists a <span class="mono">SHA256SUMS</span> file. Check an archive with
			<code class="mono">sha256sum -c SHA256SUMS --ignore-missing</code> on Linux or
			<span class="mono">Get-FileHash</span> in PowerShell.
		</p>
	</div>
</section>

<section class="section section--tight">
	<div class="shell">
		<SectionHeader
			index="PL"
			kicker="Platforms"
			title="Five platforms, two places to get them."
			lede="The Apple apps are written in Swift. The Windows, Linux, and Android apps are written in Rust and follow the Swift app."
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
			lede="The Apple apps are generated with XcodeGen and built with Xcode. The Windows, Linux, and Android apps build with Cargo from app/universal."
		/>
		<ul class="reqs">
			<li><span class="mono">macOS</span> 14 or later, iOS 17 or later</li>
			<li><span class="mono">Xcode</span> 27.1 with Swift 6.4</li>
			<li><span class="mono">XcodeGen</span> 2.46, installed with Homebrew</li>
			<li><span class="mono">Python 3</span> for the version script only</li>
			<li><span class="mono">Rust</span> 1.99.1 for Windows and Linux; Android also needs the NDK, cargo-ndk, and JDK 17</li>
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

	.download--three {
		grid-template-columns: repeat(3, minmax(0, 1fr));
	}

	.download__note {
		margin-top: 18px;
		max-width: 70ch;
		font-size: 13.5px;
		color: var(--muted);
	}

	.download__note code {
		white-space: nowrap;
	}

	.card__files {
		list-style: none;
		font-size: 12px;
		color: var(--muted);
	}

	.download__micro {
		margin-bottom: 20px;
		color: var(--muted);
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
		color: var(--accent);
	}

	.card p {
		font-size: 15px;
		line-height: 1.6;
		color: var(--ink-soft);
		max-width: 46ch;
	}

	.card :global(.badges),
	.card .btn--primary {
		margin-top: auto;
	}

	.card .btn {
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

	@media (max-width: 1000px) {
		.download--three {
			grid-template-columns: 1fr;
		}
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
