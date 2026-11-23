import { guides } from './guides';

export const site = {
	name: 'TabTool',
	url: 'https://tabtool.shurco.app',
	title: 'TabTool — open-source CSV editor for Mac, iOS, Windows, Linux, Android',
	description:
		'A free, open-source CSV editor. Native Swift on macOS and iOS from the App Store; Rust builds for Windows, Linux, and Android on GitHub Releases. Opens multi-gigabyte files in seconds, edits cells with undo, and never sends your data anywhere.',
	github: 'https://github.com/shurco/tabtool',
	releases: 'https://github.com/shurco/tabtool/releases',
	// Keep in step with universal.name in app/version.json: asset names carry the version.
	releaseVersion: '1.0.1',
	appStore: {
		ios: 'https://apps.apple.com/app/tabtool-csv-editor/id6827919069',
		mac: 'https://apps.apple.com/app/tabtool-csv-editor/id6827919069?platform=mac',
		providerToken: '128327050',
		campaign: 'website'
	},
	updated: '2026-11-23'
};

export const nav = [
	{ href: '/download/', label: 'Download' },
	{ href: '/vs/', label: 'Compare' },
	{ href: '/guides/', label: 'Guides' },
	{ href: '/faq/', label: 'FAQ' }
];

export const footerNav = {
	product: [
		{ href: '/', label: 'Overview' },
		{ href: '/download/', label: 'Download' },
		{ href: '/vs/', label: 'Comparisons' },
		{ href: '/guides/', label: 'Guides' },
		{ href: '/faq/', label: 'FAQ' },
		{ href: '/feedback/', label: 'Feedback' },
		{ href: '/press/', label: 'Press kit' }
	],
	guides: guides.map((guide) => ({ href: `/guides/${guide.slug}/`, label: guide.title })),
	legal: [
		{ href: '/privacy/', label: 'Privacy Policy' },
		{ href: '/terms/', label: 'User Agreement' }
	],
	compare: [
		{ href: '/vs/tabletool/', label: 'vs TableTool (TablePlus)' },
		{ href: '/vs/tabletool-open-source/', label: 'vs TableTool (open source)' },
		{ href: '/vs/modern-csv/', label: 'vs Modern CSV' },
		{ href: '/vs/csviewer/', label: 'vs CSViewer' },
		{ href: '/vs/tablecruncher/', label: 'vs Tablecruncher' }
	]
};
