 export const stats = [
	{ value: '5 GiB', label: 'Benchmark file', note: '68.2M rows × 8 columns' },
	{ value: '77.7M', label: 'Rows in the release app', note: 'indexed 2.7 s after launch' },
	{ value: '~91 MB', label: 'Memory after indexing', note: 'Apple M4 Pro, 24 GB' },
	{ value: '2.7 MB', label: 'macOS release build', note: 'iOS build: 2.5 MB · no Electron' },
	{ value: '0', label: 'Network requests', note: 'no analytics, no SDKs' }
];

export type Feature = {
	index: string;
	title: string;
	body: string;
	tags: string[];
	wide?: boolean;
};

export const features: Feature[] = [
	{
		index: '01',
		title: 'Format detection that reads the file',
		body: 'Encoding, separator, quote style, decimal mark, and header row come from the bytes, not the extension. Every choice stays editable in the format bar.',
		tags: ['UTF-8/16/32', 'MacRoman', 'Windows Latin', 'GBK', 'Cyrillic']
	},
	{
		index: '02',
		title: 'Editing without a spreadsheet',
		body: 'Edit cells, add and delete rows and columns, rename headers, and drag rows or columns into order. Copy and paste ranges as TSV. Undo covers cells, structure, paste, and format changes.',
		tags: ['Undo/redo', 'Drag & drop', 'TSV clipboard']
	},
	{
		index: '03',
		title: 'Sort from the header, filter from a sheet',
		body: 'Click a column header to sort. On iPhone and iPad, stack filter criteria as cards, switch each one on or off, and apply them without leaving the sheet.',
		tags: ['Sort', 'Multi-criteria filters']
	},
	{
		index: '04',
		title: 'Large files are a mode, not a warning',
		body: 'Files at or above your threshold are memory-mapped and indexed in the background. Browsing and cell edits stay on; structural edits and export switch off. Saves stream with progress and a cancel button.',
		tags: ['Memory-mapped', 'Streamed save', '2.7 s index']
	},
	{
		index: '05',
		title: 'Save in the format you actually need',
		body: 'Write the file back, or export with another delimiter, encoding, quote style, decimal mark, and header setting. CSV, TSV, and plain text, in both directions, plus export to JSON and XML with column names as keys.',
		tags: ['CSV', 'TSV', 'TXT', 'JSON', 'XML']
	},
	{
		index: '06',
		title: 'Nothing leaves the machine',
		body: 'The Apple apps are sandboxed with no network entitlement, and the App Store privacy label reads Data Not Collected. The Windows, Linux, and Android builds contain no network code either; the Android app does not request Internet access. No analytics, no third-party SDKs. Your CSV stays a file on your disk.',
		tags: ['Sandbox', 'No telemetry', 'Offline']
	},
	{
		index: '07',
		title: 'The same editor on iPhone and iPad',
		body: 'A touch-first grid with a keyboard accessory, contextual actions for rows and columns, format and filter sheets, and the same detection and encodings as the Mac.',
		tags: ['iOS 17+', 'Dynamic Type', 'Touch grid']
	},
	{
		index: '08',
		title: 'Eleven interface languages',
		body: 'English, French, Spanish, German, Italian, Polish, Belarusian, Ukrainian, Simplified Chinese, Japanese, and Korean, chosen in Settings or following the system.',
		tags: ['String Catalog', 'System language']
	},
	{
		index: '09',
		wide: true,
		title: 'Native code, not a browser in a window',
		body: 'macOS and iOS are pure Swift: a SwiftUI shell, an AppKit grid on the Mac, no Electron and no bundled web runtime. The release build is 2.7 MB on macOS and 2.5 MB on iOS, launches at once, and scrolls at native speed. Windows, Linux, and Android run a Rust build that follows the Swift app, with the CSV core checked against it by golden tests.',
		tags: ['Swift 6', 'AppKit + SwiftUI', 'Rust on Windows, Linux, Android', 'No Electron']
	}
];

export const shots = [
	{
		src: '/img/app-grid.webp',
		srcset: '/img/app-grid-1200.webp 1200w, /img/app-grid.webp 2400w',
		width: 2400,
		height: 1500,
		alt: 'TabTool for macOS showing an emoji CSV with the Emoji, Name, and Category columns.',
		title: 'Emoji, names, and categories in one grid',
		caption:
			'1,799 rows, 12 columns, UTF-8, comma. The first column is the real emoji from the file; the status line shows the detected format.'
	},
	{
		src: '/img/app-columns.webp',
		srcset: '/img/app-columns-1200.webp 1200w, /img/app-columns.webp 2400w',
		width: 2400,
		height: 1500,
		alt: 'A column selected in TabTool for macOS, with the header menu.',
		title: 'Sort, rename, or move a column in place',
		caption:
			'Click a header to select the column. The header menu sorts, renames, moves, or deletes it without leaving the grid.'
	},
	{
		src: '/img/app-large.webp',
		srcset: '/img/app-large-1200.webp 1200w, /img/app-large.webp 2400w',
		width: 2400,
		height: 1500,
		alt: 'TabTool for macOS in large-file mode showing a 1,000,000-row emoji CSV.',
		title: '1,000,000 rows, still editable',
		caption:
			'Memory-mapped and indexed in the background. Structural edits are off; scrolling, jumping, and cell edits stay on.'
	}
];

export const platforms = [
	{
		name: 'macOS',
		requirement: 'macOS 14 or later',
		source: 'Mac App Store',
		note: 'Pure Swift. Signed and sandboxed by the App Store.'
	},
	{
		name: 'iOS and iPadOS',
		requirement: 'iOS 17 or later',
		source: 'App Store',
		note: 'Same CSV core, touch-first editor. One listing with the Mac.'
	},
	{
		name: 'Windows',
		requirement: 'Windows 10 or 11, x64 or ARM64',
		source: 'GitHub Releases',
		note: 'Zip with tabtool.exe. Unsigned, no installer.'
	},
	{
		name: 'Linux',
		requirement: 'glibc 2.35+, x86_64 or aarch64',
		source: 'GitHub Releases',
		note: 'Tar.gz with the tabtool binary. Needs X11 or Wayland.'
	},
	{
		name: 'Android',
		requirement: 'Android 8.0 or later',
		source: 'GitHub Releases',
		note: 'Signed APK for arm64 and x86_64 devices. Not on Google Play yet.'
	}
];

export const faqs = [
	{
		q: 'Is TabTool free?',
		a: 'Yes. TabTool is free on the App Store for macOS and iOS and free on GitHub Releases for Windows, Linux, and Android. The source code is MIT-licensed, so you can read it, build it, and change it at no cost.'
	},
	{
		q: 'Where can I download it?',
		a: 'macOS 14+ and iOS 17+: the App Store, where one listing covers the Mac, iPhone, and iPad. Windows, Linux, and Android: the Releases page of the GitHub repository, as a zip, a tar.gz, or an APK. You can also build any of them from the source.'
	},
	{
		q: 'How large a file can it open?',
		a: 'The tested ceiling is a 5 GiB file with 77.7 million rows. It was fully indexed 2.7 seconds after launch in the release app, using about 91 MB of memory. Files at or above the large-file threshold open in a read-only structure mode with background indexing; the threshold is a setting, 100 MiB by default.'
	},
	{
		q: 'Does TabTool upload my data?',
		a: 'No. The Apple apps are sandboxed and have no network entitlement; the Windows, Linux, and Android builds contain no network code, and the Android app does not request Internet access. There is no analytics, no crash reporting SDK, and no account. Saving writes to the file you opened, and export writes a new file you choose.'
	},
	{
		q: 'Which formats and encodings does it support?',
		a: 'CSV, TSV, and plain text with automatic detection of encoding, separator, quotes, escape character, decimal mark, and header row. Encodings include UTF-8/16/32, MacRoman, Windows Latin 1 and 2, GBK, ISO Latin 2, Cyrillic, Greek, Turkish, and Japanese. Export also writes JSON and XML. Opening XLSX, ODS, JSON, and database files is not supported.'
	},
	{
		q: 'Does it have formulas, styles, or charts?',
		a: 'No, by design. TabTool is a CSV editor: cells, rows, columns, sorting, filters, and format conversion. For formulas and charts, a spreadsheet is the right tool.'
	},
	{
		q: 'Does it work offline?',
		a: 'Yes. Every feature works without a connection. The app does not contact a server at any point.'
	},
	{
		q: 'What about Windows, Linux, and Android?',
		a: 'They are available from GitHub Releases. All three are written in Rust: Windows and Linux ship as archives for x64 and ARM, Android as a signed APK. They follow the Swift app, and golden tests check their CSV core against it. The Windows and Linux archives are not code-signed, so Windows SmartScreen may warn on first launch; each release lists SHA-256 checksums to verify the download.'
	},
	{
		q: 'Is the iOS app a companion or a full editor?',
		a: 'It is a full editor for cells, rows, columns, sorting, filtering, and format changes. Large-file mode on iOS is limited to browsing and cell edits, like the Mac.'
	},
	{
		q: 'How do I contribute?',
		a: 'Open an issue or a pull request on GitHub. Translations live in a String Catalog; native-speaker reviews are especially welcome because the ten non-English translations are machine-generated.'
	},
	{
		q: 'Can I use it commercially?',
		a: 'The MIT license allows commercial use of the source. If you redistribute a build, keep the copyright and permission notice with it.'
	},
	{
		q: 'How is this different from a spreadsheet?',
		a: 'A spreadsheet stores formulas, styles, and a calculation model. TabTool stores nothing but the delimited text you give it, detects the format on open, and writes the same bytes back. The comparison pages cover specific CSV editors.'
	}
];
