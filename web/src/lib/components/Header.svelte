<script lang="ts">
	import { page } from '$app/state';
	import { site, nav } from '#lib/site';

	const current = $derived(page.url.pathname);
</script>

<header class="head">
	<div class="shell head__inner">
		<a class="brand" href="/" aria-label="TabTool home">
			<svg class="brand__mark" viewBox="0 0 64 64" aria-hidden="true">
				<rect width="64" height="64" rx="12" fill="currentColor" />
				<g fill="none" stroke="var(--paper)" stroke-width="2.6">
					<rect x="12" y="12" width="40" height="40" rx="3" />
					<path d="M25.3 12v40M38.6 12v40M12 25.3h40M12 38.6h40" />
				</g>
				<rect x="25.3" y="25.3" width="13.3" height="13.3" fill="var(--accent)" />
			</svg>
			<span class="brand__word">TabTool</span>
		</a>

		<nav class="nav" aria-label="Main">
			{#each nav as item (item.href)}
				<a
					href={item.href}
					class:is-active={current === item.href || current.startsWith(item.href)}
					aria-current={current === item.href ? 'page' : undefined}
				>{item.label}</a>
			{/each}
		</nav>

		<a class="star" href={site.github} rel="noopener">
			<svg viewBox="0 0 16 16" width="15" height="15" aria-hidden="true" fill="currentColor">
				<path
					d="M8 .25a.75.75 0 0 1 .673.418l1.882 3.815 4.21.612a.75.75 0 0 1 .416 1.279l-3.046 2.97.719 4.192a.75.75 0 0 1-1.088.791L8 12.347l-3.766 1.98a.75.75 0 0 1-1.088-.79l.72-4.194L.818 6.374a.75.75 0 0 1 .416-1.28l4.21-.611L7.327.668A.75.75 0 0 1 8 .25Z"
				/>
			</svg>
			<span>Star on GitHub</span>
		</a>
	</div>
</header>

<style>
	.head {
		position: sticky;
		top: 0;
		z-index: 50;
		background: color-mix(in srgb, var(--paper) 92%, transparent);
		backdrop-filter: blur(10px);
		border-bottom: 1px solid var(--line);
	}

	.head__inner {
		display: flex;
		align-items: center;
		gap: 28px;
		height: 64px;
	}

	.brand {
		display: inline-flex;
		align-items: center;
		gap: 10px;
		color: var(--ink);
		text-decoration: none;
		font-weight: 650;
		letter-spacing: -0.01em;
		font-size: 17px;
	}

	.brand:hover {
		text-decoration: none;
	}

	.brand__mark {
		width: 26px;
		height: 26px;
	}

	.nav {
		display: flex;
		align-items: center;
		gap: 22px;
		margin-left: 8px;
		font-size: 14px;
		color: var(--ink-soft);
	}

	.nav a {
		text-decoration: none;
		padding-block: 6px;
		border-bottom: 1px solid transparent;
	}

	.nav a:hover {
		color: var(--ink);
		border-bottom-color: var(--ink);
	}

	.nav a.is-active {
		color: var(--ink);
		border-bottom-color: var(--accent);
	}

	.star {
		margin-left: auto;
		display: inline-flex;
		align-items: center;
		gap: 8px;
		padding: 8px 12px;
		border: 1px solid var(--line-strong);
		border-radius: var(--radius);
		font-family: var(--mono);
		font-size: 12px;
		text-decoration: none;
		background: var(--paper-raised);
	}

	.star:hover {
		text-decoration: none;
		border-color: var(--ink);
		background: var(--accent-wash);
	}

	@media (max-width: 720px) {
		.head__inner {
			height: auto;
			flex-wrap: wrap;
			gap: 8px 18px;
			padding-block: 10px 0;
		}
		.nav {
			display: flex;
			order: 3;
			width: 100%;
			gap: 18px;
			margin-left: 0;
			padding-bottom: 8px;
			overflow-x: auto;
			scrollbar-width: none;
			font-size: 13px;
		}
		.nav::-webkit-scrollbar {
			display: none;
		}
		.star {
			padding: 8px 11px;
		}
	}
</style>
