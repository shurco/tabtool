<script lang="ts">
	import { site } from '#lib/site';
</script>

<section class="cta">
	<div class="shell cta__inner">
		<p class="eyebrow">Status</p>
		<h2>The source is ready.<br />The store builds are next.</h2>
		<p class="lede cta__lede">
			TabTool 1.0 is in development for macOS 14+ and iOS 17+. Follow the repository for the first
			build, the release notes, and the Windows, Linux, and Android plans.
		</p>
		<div class="cta__actions">
			<a class="btn btn--primary" href={site.github} rel="noopener">
				Star on GitHub <span class="btn__arrow" aria-hidden="true">↗</span>
			</a>
			<a class="btn btn--ghost" href="/download/">Platforms and status</a>
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

	.cta__actions {
		display: flex;
		flex-wrap: wrap;
		gap: 12px;
		margin-top: 10px;
	}
</style>
