<script lang="ts">
	import Seo from '#lib/components/Seo.svelte';
	import SectionHeader from '#lib/components/SectionHeader.svelte';
	import Roadmap from '#lib/components/Roadmap.svelte';
	import OpenSource from '#lib/components/OpenSource.svelte';
	import StoreBadges from '#lib/components/StoreBadges.svelte';
	import PlatformIcon from '#lib/components/PlatformIcon.svelte';
	import { site } from '#lib/site';
	import { checksumsURL, releaseFile, type OtherPlatform } from '#lib/downloads';
	import { trackDownloadClick } from '#lib/track';

	const title = 'Download TabTool — free CSV editor for Mac, iOS, Windows, Linux, Android';
	const description =
		'TabTool 1.0 is free on the App Store for macOS 14+ and iOS 17+, and on GitHub Releases for Windows, Linux, and Android. Download it, or build it from source.';

	// File names follow .github/workflows/release.yml; see docs/release.md.
	const releaseBuilds: {
		key: OtherPlatform;
		name: string;
		requirement: string;
		body: string;
		format: string;
		files: { suffix: string; label: string }[];
	}[] = [
		{
			key: 'windows',
			name: 'Windows',
			requirement: 'Windows 10 or 11',
			body: 'The desktop editor with document windows, large-file mode, and export. Unzip it and run tabtool.exe from the extracted folder; nothing is installed. The archive is not code-signed, so SmartScreen may ask once before the first launch.',
			format: 'zip',
			files: [
				{ suffix: 'windows-x86_64.zip', label: 'x64' },
				{ suffix: 'windows-aarch64.zip', label: 'ARM64' }
			]
		},
		{
			key: 'linux',
			name: 'Linux',
			requirement: 'glibc 2.35 or later',
			body: 'The same desktop editor. Extract the archive and run tabtool from the extracted folder. Needs X11 or Wayland and fontconfig; built for Ubuntu 22.04, Debian 12, Fedora 36, and newer.',
			format: 'tar.gz',
			files: [
				{ suffix: 'linux-x86_64.tar.gz', label: 'x86_64' },
				{ suffix: 'linux-aarch64.tar.gz', label: 'aarch64' }
			]
		},
		{
			key: 'android',
			name: 'Android',
			requirement: 'Android 8.0 or later',
			body: 'A touch editor that opens files from the system picker, a file manager, or a share sheet. Install the signed APK; Android asks to allow installs from your browser first.',
			format: 'signed APK, arm64 and x86_64',
			files: [{ suffix: 'android.apk', label: 'APK' }]
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
			<article class="card" id="macos">
				<p class="eyebrow">macOS</p>
				<h2>macOS 14 or later</h2>
				<p class="card__status mono">Status: available</p>
				<p>
					The full editor: format detection, structural editing, sorting, large-file mode, export.
					The release build is 2.7 MB, written in pure Swift.
				</p>
				<StoreBadges platforms={['mac']} placement="download" />
			</article>

			<article class="card" id="ios">
				<p class="eyebrow">iOS and iPadOS</p>
				<h2>iOS 17 or later</h2>
				<p class="card__status mono">Status: available</p>
				<p>
					A touch-first editor with contextual row and column actions, filter cards, and the same
					CSV core. The release build is 2.5 MB. Run it in the simulator with a single script
					command.
				</p>
				<StoreBadges platforms={['ios']} placement="download" />
			</article>
		</div>
	</div>
</section>

<section class="section section--tight" id="releases">
	<div class="shell">
		<p class="mono download__micro">Free · no account · no tracking · MIT · x64 and ARM</p>
		<div class="download download--three">
			{#each releaseBuilds as build (build.name)}
				<article class="card" id={build.key}>
					<p class="eyebrow">{build.name}</p>
					<h2>{build.requirement}</h2>
					<p class="card__status mono">Status: available</p>
					<p>{build.body}</p>
					<p class="card__files mono">Version {site.releaseVersion} · {build.format}</p>
					<div class="card__buttons">
						{#each build.files as file (file.suffix)}
							<a
								class="btn btn--primary"
								href={releaseFile(file.suffix)}
								rel="noopener"
								onclick={() => trackDownloadClick(build.key, 'download')}
							>
								<PlatformIcon platform={build.key} size={18} />
								{file.label}
							</a>
						{/each}
					</div>
				</article>
			{/each}
		</div>
		<p class="download__note">
			Every release lists a <a class="mono" href={checksumsURL} rel="noopener">SHA256SUMS</a> file. Check an archive with
			<code class="mono">sha256sum -c SHA256SUMS --ignore-missing</code> on Linux or
			<span class="mono">Get-FileHash</span> in PowerShell. Older versions and release notes are on
			<a href={site.releases} rel="noopener">GitHub Releases</a>.
		</p>
	</div>
</section>

<section class="section section--tight">
	<div class="shell">
		<SectionHeader
			index="PL"
			kicker="Platforms"
			title="Five platforms, one click each."
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
		font-size: 12px;
		color: var(--muted);
	}

	.card__buttons {
		display: flex;
		flex-wrap: wrap;
		gap: 10px;
		margin-top: auto;
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

	.card :global(.badges) {
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
