<script lang="ts">
	import { site } from '#lib/site';

	let copied = $state(false);
	const clone = $derived(`git clone ${site.github}.git`);

	async function copy() {
		try {
			await navigator.clipboard.writeText(clone);
			copied = true;
			setTimeout(() => (copied = false), 1800);
		} catch {
			copied = false;
		}
	}
</script>

<section class="oss" id="open-source">
	<div class="shell oss__inner">
		<div class="oss__copy">
			<p class="eyebrow oss__eyebrow">Open source</p>
			<h2>MIT, all of it.</h2>
			<p class="oss__lede">
				The CSV core, the SwiftUI shell, the AppKit grid, the iOS grid, and the tests are in one
				repository. No license server, no telemetry, no open-core asterisk.
			</p>
			<div class="oss__actions">
				<a class="btn btn--light" href={site.github} rel="noopener">
					Star on GitHub <span class="btn__arrow" aria-hidden="true">↗</span>
				</a>
				<a class="oss__link" href={`${site.github}/issues`} rel="noopener">Issues and pull requests</a>
			</div>
			<p class="oss__ask">
				If TabTool saves you a spreadsheet detour, a star is the cheapest way to help other people
				find it.
			</p>
		</div>

		<div class="oss__terminal">
			<div class="oss__bar">
				<span class="mono">Build from source</span>
				<button type="button" class="oss__copybtn mono" onclick={copy} aria-live="polite">
					{copied ? 'Copied' : 'Copy'}
				</button>
			</div>
			<pre><code><span class="c"># Xcode 27.1, Swift 6.4, XcodeGen 2.46</span>
brew install xcodegen
<span class="c"># clone and generate the project</span>
{clone}
cd tabtool
./scripts/app run macos</code></pre>
			<div class="oss__meta">
				<span class="mono">MIT license</span>
				<span class="mono">254 core tests</span>
				<span class="mono">320 app tests</span>
			</div>
		</div>
	</div>
</section>

<style>
	.oss {
		background: var(--dark);
		color: var(--paper);
		border-top: 1px solid var(--dark-line);
		padding-block: clamp(56px, 7vw, 96px);
		background-image: linear-gradient(to right, var(--dark-line) 1px, transparent 1px);
		background-size: 25% 100%;
	}

	.oss__inner {
		display: grid;
		grid-template-columns: minmax(0, 1.05fr) minmax(0, 1fr);
		gap: clamp(32px, 5vw, 72px);
		align-items: start;
	}

	.oss__inner > * {
		min-width: 0;
	}

	.oss__eyebrow {
		color: color-mix(in srgb, var(--paper) 55%, transparent);
	}

	.oss h2 {
		margin-top: 14px;
		font-size: clamp(2.6rem, 6vw, 4.6rem);
	}

	.oss__lede {
		margin-top: 18px;
		max-width: 46ch;
		font-size: 16px;
		line-height: 1.6;
		color: color-mix(in srgb, var(--paper) 76%, transparent);
	}

	.oss__actions {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 20px;
		margin-top: 26px;
	}

	.btn--light {
		background: var(--paper);
		color: var(--ink);
		border-color: var(--paper);
	}

	.btn--light:hover {
		background: var(--accent);
		color: var(--accent-ink);
		border-color: var(--accent);
	}

	.oss__link {
		font-family: var(--mono);
		font-size: 12px;
		color: color-mix(in srgb, var(--paper) 72%, transparent);
	}

	.oss__ask {
		margin-top: 26px;
		max-width: 40ch;
		font-family: var(--serif);
		font-size: clamp(1.2rem, 1.9vw, 1.5rem);
		line-height: 1.25;
		color: var(--paper);
	}

	.oss__terminal {
		border: 1px solid var(--dark-line);
		background: color-mix(in srgb, var(--paper) 4%, var(--dark));
	}

	.oss__bar {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 10px 14px;
		border-bottom: 1px solid var(--dark-line);
		color: color-mix(in srgb, var(--paper) 60%, transparent);
	}

	.oss__copybtn {
		background: transparent;
		border: 1px solid var(--dark-line);
		border-radius: 2px;
		padding: 4px 9px;
		cursor: pointer;
		color: var(--paper);
	}

	.oss__copybtn:hover {
		border-color: var(--paper);
	}

	pre {
		margin: 0;
		padding: 18px 16px;
		overflow-x: auto;
		font-family: var(--mono);
		font-size: 12.5px;
		line-height: 1.9;
		color: color-mix(in srgb, var(--paper) 88%, transparent);
	}

	@media (max-width: 600px) {
		pre {
			white-space: pre-wrap;
			overflow-wrap: anywhere;
			font-size: 11.5px;
		}
	}

	.c {
		color: color-mix(in srgb, var(--paper) 42%, transparent);
	}

	.oss__meta {
		display: flex;
		flex-wrap: wrap;
		gap: 8px 18px;
		padding: 12px 16px;
		border-top: 1px solid var(--dark-line);
		color: color-mix(in srgb, var(--paper) 50%, transparent);
	}

	@media (max-width: 900px) {
		.oss__inner {
			grid-template-columns: 1fr;
		}
	}
</style>
