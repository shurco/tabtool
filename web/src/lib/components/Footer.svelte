<script lang="ts">
	import { site, footerNav } from '#lib/site';
</script>

<footer class="foot">
	<div class="shell">
		<div class="foot__top">
			<div class="foot__brand">
				<p class="eyebrow">TabTool</p>
				<p class="foot__claim">
					A native CSV editor for macOS and iOS. Open the file, fix the cell, write it back.
				</p>
				<a class="foot__star" href={site.github} rel="noopener">Star the repository →</a>
			</div>

			<nav class="foot__cols" aria-label="Footer">
				<div>
					<p class="eyebrow">Product</p>
					<ul>
						{#each footerNav.product as item (item.href)}
							<li><a href={item.href}>{item.label}</a></li>
						{/each}
					</ul>
				</div>
				<div>
					<p class="eyebrow">Comparisons</p>
					<ul>
						{#each footerNav.compare as item (item.href)}
							<li><a href={item.href}>{item.label}</a></li>
						{/each}
					</ul>
				</div>
				<div>
					<p class="eyebrow">Legal</p>
					<ul>
						{#each footerNav.legal as item (item.href)}
							<li><a href={item.href}>{item.label}</a></li>
						{/each}
						<li><a href={`${site.github}/blob/HEAD/LICENSE`} rel="noopener">MIT License</a></li>
					</ul>
				</div>
			</nav>
		</div>

		<div class="foot__word" aria-hidden="true">TabTool</div>

		<div class="foot__bottom">
			<p class="mono">© 2026 TabTool · MIT · Built with Swift, SvelteKit, and no telemetry.</p>
			<p class="mono">
				<a href={site.github} rel="noopener">GitHub</a>
				<span aria-hidden="true"> / </span>
				<a href="/privacy/">Privacy</a>
				<span aria-hidden="true"> / </span>
				<a href="/terms/">Terms</a>
			</p>
		</div>
	</div>
</footer>

<style>
	.foot {
		border-top: 1px solid var(--line);
		padding-top: clamp(48px, 6vw, 80px);
	}

	.foot__top {
		display: grid;
		grid-template-columns: minmax(0, 1fr) minmax(0, 1.4fr);
		gap: 48px;
		padding-bottom: clamp(40px, 5vw, 72px);
	}

	.foot__claim {
		margin-top: 14px;
		max-width: 34ch;
		font-family: var(--serif);
		font-size: clamp(1.4rem, 2.4vw, 1.9rem);
		line-height: 1.15;
		letter-spacing: -0.01em;
	}

	.foot__star {
		display: inline-block;
		margin-top: 18px;
		font-family: var(--mono);
		font-size: 12px;
		color: var(--accent);
		text-decoration: none;
	}

	.foot__star:hover {
		text-decoration: underline;
	}

	.foot__cols {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: 32px;
	}

	.foot__cols ul {
		margin-top: 16px;
		display: grid;
		gap: 9px;
		list-style: none;
	}

	.foot__cols a {
		font-size: 14px;
		text-decoration: none;
		color: var(--ink-soft);
	}

	.foot__cols a:hover {
		color: var(--ink);
		text-decoration: underline;
	}

	.foot__word {
		font-family: var(--serif);
		font-size: clamp(4rem, 17vw, 15rem);
		line-height: 0.8;
		letter-spacing: -0.03em;
		color: var(--ink);
		border-bottom: 1px solid var(--line);
		padding-bottom: 0.04em;
		overflow: hidden;
	}

	.foot__bottom {
		display: flex;
		flex-wrap: wrap;
		justify-content: space-between;
		gap: 10px 24px;
		padding-block: 20px 28px;
		color: var(--muted);
	}

	.foot__bottom a {
		text-decoration: none;
	}

	.foot__bottom a:hover {
		color: var(--ink);
	}

	@media (max-width: 860px) {
		.foot__top {
			grid-template-columns: 1fr;
		}
		.foot__cols {
			gap: 24px;
		}
	}

	@media (max-width: 620px) {
		.foot__cols {
			grid-template-columns: 1fr 1fr;
		}
	}
</style>
