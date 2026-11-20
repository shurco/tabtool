<script lang="ts">
	import { appStoreURL, trackDownloadClick, trackStoreClick, type StorePlacement } from '#lib/track';
	import { isOther, otherLabels, platformHref, type Platform } from '#lib/downloads';
	import PlatformIcon from '#lib/components/PlatformIcon.svelte';

	let {
		platforms = ['mac', 'ios', 'windows', 'linux', 'android'],
		placement = 'download',
		size = 'md'
	}: { platforms?: Platform[]; placement?: StorePlacement; size?: 'md' | 'sm' } = $props();

	// Official Apple badges, unmodified, from developer.apple.com/app-store/marketing/guidelines.
	const badges = {
		mac: { src: '/img/badges/mac-app-store.svg', alt: 'Download on the Mac App Store', width: 156 },
		ios: { src: '/img/badges/app-store.svg', alt: 'Download on the App Store', width: 120 }
	} as const;
</script>

<div class="badges" class:badges--sm={size === 'sm'}>
	{#each platforms as platform, i (platform)}
		{#if isOther(platform)}
			{#if i > 0 && !isOther(platforms[i - 1])}<span class="badges__break"></span>{/if}
			<a
				class="pbtn"
				href={platformHref(platform)}
				onclick={() => trackDownloadClick(platform, placement)}
			>
				<PlatformIcon {platform} size={size === 'sm' ? 18 : 24} />
				<span class="pbtn__text">
					<small>Download for</small>
					<strong>{otherLabels[platform]}</strong>
				</span>
			</a>
		{:else}
			{@const badge = badges[platform]}
			<a
				class="store"
				href={appStoreURL(platform)}
				rel="noopener"
				onclick={() => trackStoreClick(platform, placement)}
			>
				<img src={badge.src} alt={badge.alt} width={badge.width} height="40" />
			</a>
		{/if}
	{/each}
</div>

<style>
	.badges {
		--h: 48px;
		display: flex;
		flex-wrap: wrap;
		gap: 12px;
		margin-top: 28px;
	}

	/* Apple badges on one line, the other platforms on the next. */
	.badges__break {
		flex-basis: 100%;
		height: 0;
		margin-top: -12px;
	}

	.badges--sm {
		--h: 36px;
		gap: 8px;
		margin-top: 0;
	}

	.store {
		display: block;
		line-height: 0;
	}

	.store img {
		height: var(--h);
		width: auto;
	}

	/* Matches the Apple badges: black, thin grey border, same height. */
	.pbtn {
		box-sizing: border-box;
		display: inline-flex;
		align-items: center;
		gap: calc(var(--h) / 4);
		height: var(--h);
		padding-inline: calc(var(--h) / 3);
		border: 1px solid #a6a6a6;
		border-radius: calc(var(--h) / 6);
		background: #000;
		color: #fff;
		text-decoration: none;
		transition: background 0.16s ease;
	}

	.pbtn:hover {
		background: #262626;
		text-decoration: none;
	}

	.pbtn__text {
		display: flex;
		flex-direction: column;
		line-height: 1.1;
	}

	.pbtn small {
		font-size: calc(var(--h) / 4.4);
		letter-spacing: 0.01em;
	}

	.pbtn strong {
		font-size: calc(var(--h) / 2.4);
		font-weight: 600;
		letter-spacing: -0.01em;
	}
</style>
