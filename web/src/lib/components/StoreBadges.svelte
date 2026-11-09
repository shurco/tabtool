<script lang="ts">
	import { appStoreURL, trackStoreClick, type StorePlacement, type StorePlatform } from '#lib/track';

	let {
		platforms = ['mac', 'ios'],
		placement = 'download'
	}: { platforms?: StorePlatform[]; placement?: StorePlacement } = $props();

	// Official Apple badges, unmodified, from developer.apple.com/app-store/marketing/guidelines.
	const badges = {
		mac: {
			src: '/img/badges/mac-app-store.svg',
			alt: 'Download on the Mac App Store',
			width: 156
		},
		ios: {
			src: '/img/badges/app-store.svg',
			alt: 'Download on the App Store',
			width: 120
		}
	} as const;
</script>

<div class="badges">
	{#each platforms as platform (platform)}
		{@const badge = badges[platform]}
		<a
			href={appStoreURL(platform, placement)}
			rel="noopener"
			onclick={() => trackStoreClick(platform, placement)}
		>
			<img src={badge.src} alt={badge.alt} width={badge.width} height="40" />
		</a>
	{/each}
</div>

<style>
	.badges {
		display: flex;
		flex-wrap: wrap;
		gap: 12px;
		margin-top: 28px;
	}

	a {
		display: block;
		line-height: 0;
	}

	img {
		height: 48px;
		width: auto;
	}
</style>
