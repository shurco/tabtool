import { site } from '#lib/site';

export type StorePlatform = 'mac' | 'ios';
export type StorePlacement = 'hero' | 'cta' | 'download' | 'header-bar' | 'compare' | 'about';

/// The App Store campaign link for one placement.
///
/// `pt` is the App Store Connect provider token; when it is empty the link
/// still works and still carries `ct` and `mt`, but Apple cannot attribute
/// the install. Fill `providerToken` in `site.ts` before a campaign launch.
///
/// Keep one `ct` per channel, not per placement: App Store Connect hides
/// campaigns with fewer than five first-time downloads in the selected range.
/// The placement is still recorded in GA4 by `trackStoreClick`.
export function appStoreURL(
	platform: StorePlatform,
	campaign = site.appStore.campaign
): string {
	const base = site.appStore[platform];
	const mediaType = platform === 'mac' ? '12' : '8';
	const params = new URLSearchParams();
	if (site.appStore.providerToken) params.set('pt', site.appStore.providerToken);
	params.set('ct', campaign);
	params.set('mt', mediaType);
	return `${base}${base.includes('?') ? '&' : '?'}${params.toString()}`;
}

/// Records a store click in GA4. The event is deliberately small: platform,
/// placement, and whatever GA4 already collects for the page.
export function trackStoreClick(platform: StorePlatform, placement: StorePlacement): void {
	if (typeof window === 'undefined') return;
	window.gtag?.('event', 'app_store_click', { platform, placement });
}
