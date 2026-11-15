<script lang="ts">
	import { site } from '#lib/site';
	import StoreBadges from '#lib/components/StoreBadges.svelte';
	import { appStoreURL, trackStoreClick } from '#lib/track';
</script>

<section class="cta">
	<div class="shell cta__inner">
		<p class="eyebrow">Status</p>
		<h2>TabTool 1.0 is out.<br />Free on five platforms.</h2>
		<p class="lede cta__lede">
			macOS 14+ and iOS 17+ from the App Store. Windows, Linux, and Android from GitHub Releases,
			with release notes for every version.
		</p>
		<StoreBadges placement="cta" />
		<div class="cta__actions">
			<a
				class="btn btn--primary"
				href={appStoreURL('ios')}
				onclick={() => trackStoreClick('ios', 'cta')}
			>
				Download free <span class="btn__arrow" aria-hidden="true">↗</span>
			</a>
			<a class="btn btn--ghost" href={site.releases} rel="noopener">Windows, Linux, Android</a>
			<a class="btn btn--ghost" href={site.github} rel="noopener">Star on GitHub</a>
			<a class="btn btn--ghost" href="/download/">All downloads</a>
		</div>
	</div>
</section>

<style>
	.cta {
		border-top: 1px solid var(--line);
		padding-block: clamp(64px, 8vw, 112px);
		background-image: linear-gradient(to right, var(--line) 1px, transparent 1px);
		background-size: 25% 100%;
	}

	.cta__inner {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 18px;
	}

	.cta h2 {
		font-size: clamp(2.2rem, 5vw, 4rem);
		max-width: 18ch;
	}

	.cta__lede {
		max-width: 56ch;
	}

	.cta__inner :global(.badges) {
		margin-top: 10px;
	}

	.cta__actions {
		display: flex;
		flex-wrap: wrap;
		gap: 12px;
		margin-top: 10px;
	}
</style>
