<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { appStoreURL, trackStoreClick } from '#lib/track';

	const storageKey = 'tabtool-download-bar-dismissed';
	let visible = $state(false);

	const hiddenHere = $derived(page.url.pathname === '/privacy/' || page.url.pathname === '/terms/');

	onMount(() => {
		if (sessionStorage.getItem(storageKey) === '1') return;
		const media = window.matchMedia('(max-width: 760px)');
		const update = () => {
			visible = media.matches;
		};
		update();
		media.addEventListener('change', update);
		return () => media.removeEventListener('change', update);
	});

	function dismiss() {
		visible = false;
		sessionStorage.setItem(storageKey, '1');
	}
</script>

{#if visible && !hiddenHere}
	<aside class="download-bar" aria-label="Download TabTool">
		<div class="download-bar__copy">
			<strong>TabTool</strong>
			<span>Free CSV editor for iPhone, iPad, and Mac</span>
		</div>
		<a
			class="download-bar__cta"
			href={appStoreURL('ios', 'header-bar')}
			onclick={() => trackStoreClick('ios', 'header-bar')}
		>
			Download free
		</a>
		<button class="download-bar__close" type="button" onclick={dismiss} aria-label="Dismiss">
			×
		</button>
	</aside>
{/if}

<style>
	.download-bar {
		position: fixed;
		inset-inline: 0;
		bottom: 0;
		z-index: 60;
		display: none;
		align-items: center;
		gap: 12px;
		padding: 10px 16px calc(10px + env(safe-area-inset-bottom));
		background: var(--paper-raised);
		border-top: 1px solid var(--line-strong);
		box-shadow: 0 -12px 30px -24px rgb(20 21 15 / 0.45);
	}

	.download-bar__copy {
		display: flex;
		flex-direction: column;
		min-width: 0;
		line-height: 1.25;
	}

	.download-bar__copy strong {
		font-size: 14px;
	}

	.download-bar__copy span {
		font-size: 11.5px;
		color: var(--muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.download-bar__cta {
		flex: none;
		padding: 10px 14px;
		border-radius: var(--radius);
		background: var(--ink);
		color: var(--paper);
		font-size: 13px;
		font-weight: 600;
		text-decoration: none;
	}

	.download-bar__cta:hover {
		text-decoration: none;
		background: var(--accent);
	}

	.download-bar__close {
		flex: none;
		border: 0;
		background: transparent;
		color: var(--muted);
		font-size: 22px;
		line-height: 1;
		padding: 4px 2px;
		cursor: pointer;
	}

	@media (max-width: 760px) {
		:global(body) {
			padding-bottom: calc(74px + env(safe-area-inset-bottom));
		}

		.download-bar {
			display: flex;
		}
	}
</style>
