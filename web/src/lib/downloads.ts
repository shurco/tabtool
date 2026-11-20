import { site } from '#lib/site';
import { appStoreURL } from '#lib/track';

export type Platform = 'mac' | 'ios' | 'windows' | 'linux' | 'android';
export type OtherPlatform = Exclude<Platform, 'mac' | 'ios'>;

export const isOther = (platform: Platform): platform is OtherPlatform =>
	platform !== 'mac' && platform !== 'ios';

/// A file attached to the current GitHub Release. Names follow
/// .github/workflows/release.yml: `tabtool-<version>-<suffix>`.
export const releaseFile = (suffix: string) =>
	`${site.releases}/download/v${site.releaseVersion}/tabtool-${site.releaseVersion}-${suffix}`;

export const checksumsURL = `${site.releases}/download/v${site.releaseVersion}/SHA256SUMS`;

export const otherLabels: Record<OtherPlatform, string> = {
	windows: 'Windows',
	linux: 'Linux',
	android: 'Android'
};

/// Where a platform button goes: the App Store for Apple, the download page card
/// (which offers every architecture) for the rest.
export const platformHref = (platform: Platform) =>
	isOther(platform) ? `/download/#${platform}` : appStoreURL(platform);
