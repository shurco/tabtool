 export const stats = [
	{ value: '5 GiB', label: 'Largest tested file', note: '68.2M rows × 8 columns' },
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
		body: 'Write the file back, or export with another delimiter, encoding, quote style, decimal mark, and header setting. CSV, TSV, and plain text, in both directions.',
		tags: ['CSV', 'TSV', 'TXT']
	},
	{
		index: '06',
		title: 'Nothing leaves the machine',
		body: 'The app is sandboxed with no network entitlement, no analytics, and no third-party SDKs. The App Store privacy label will read Data Not Collected. Your CSV stays a file on your disk.',
		tags: ['Sandbox', 'No telemetry', 'Offline']
	},	{
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
		body: 'macOS and iOS are pure Swift: a SwiftUI shell, an AppKit grid on the Mac, no Electron and no bundled web runtime. The release build is 2.7 MB on macOS and 2.5 MB on iOS, launches at once, and scrolls at native speed. The Windows and Linux ports are planned in Rust.',
		tags: ['Swift 6', 'AppKit + SwiftUI', 'No Electron', 'Rust ports planned']
	}
];

export const shots = [
	{
		src: '/img/mac-grid.webp',
		srcset: '/img/mac-grid-1200.webp 1200w, /img/mac-grid.webp 2400w',
		width: 2400,
		height: 1502,
		alt: 'TabTool for macOS showing a 1,200-row orders CSV, with the format bar below the grid.',
		title: 'The grid',
		caption:
			'1,200 rows, 8 columns, UTF-8, comma. The format bar below the grid is the file’s own format, changed in place.'
	},
	{
		src: '/img/mac-columns.webp',
		srcset: '/img/mac-columns-1200.webp 1200w, /img/mac-columns.webp 2400w',
		width: 2400,
		height: 1543,
		alt: 'A column selected in TabTool for macOS, with its header menu and the rest of the table dimmed.',
		title: 'Column selected',
		caption:
			'Click a header to select the column. Sort, rename, move, or delete it from the header, the context menu, or the Table menu.'
	},
	{
		src: '/img/mac-large.webp',
		srcset: '/img/mac-large-1200.webp 1200w, /img/mac-large.webp 2400w',
		width: 2400,
		height: 1543,
		alt: 'TabTool in large-file mode showing a 2.1-million-row CSV with the large-file banner.',
		title: 'Large-file mode',
		caption:
			'2.1 million rows in this capture. Structural edits are off; scrolling, jumping, and cell edits stay on. The banner never hides the row count.'
	},
	{
		src: '/img/ios-grid.webp',
		srcset: '/img/ios-grid-500.webp 500w, /img/ios-grid.webp 1000w',
		width: 1000,
		height: 460,
		alt: 'The same CSV open in TabTool for iPhone in landscape, showing the grid and row count.',
		title: 'The same file on iPhone',
		caption:
			'Row numbers, sticky headers, and a status line with the row count, encoding, and separator. The toolbar holds undo, search, export, and the overflow menu.'
	}
];

export const platforms = [
	{
		name: 'macOS',
		requirement: 'macOS 14 or later',
		status: 'In development',
		tone: 'active',
		note: '1.0 release checklist in progress. Pure Swift.'
	},
	{
		name: 'iOS and iPadOS',
		requirement: 'iOS 17 or later',
		status: 'In development',
		tone: 'active',
		note: 'Same CSV core, touch-first editor. Pure Swift.'
	},
	{
		name: 'Windows',
		requirement: 'Not started',
		status: 'Planned',
		tone: 'planned',
		note: 'Planned in Rust. No date yet.'
	},
	{
		name: 'Linux',
		requirement: 'Not started',
		status: 'Planned',
		tone: 'planned',
		note: 'Planned in Rust. No date yet.'
	},
	{
		name: 'Android',
		requirement: 'Not started',
		status: 'Planned',
		tone: 'planned',
		note: 'Language not chosen yet. No date.'
	}
];

export const faqs = [
	{
		q: 'Is TabTool free?',
		a: 'The source code is MIT-licensed, so you can read it, build it, and change it at no cost. Pricing for the signed App Store build will be published with the 1.0 release.'
	},
	{
		q: 'When can I download it?',
		a: 'TabTool 1.0 for macOS 14+ and iOS 17+ is in development. Watch the GitHub repository for the first build and the App Store listing. There is no public download yet.'
	},
	{
		q: 'How large a file can it open?',
		a: 'The tested ceiling is a 5 GiB file with 68.2 million rows and 8 columns. It was fully indexed 2.7 seconds after launch, using about 91 MB of memory. Files at or above the large-file threshold open in a read-only structure mode with background indexing; the threshold is a setting, 100 MiB by default.'
	},
	{
		q: 'Does TabTool upload my data?',
		a: 'No. The app is sandboxed and has no network entitlement. There is no analytics, no crash reporting SDK, and no account. Saving writes to the file you opened, and export writes a new file you choose.'
	},
	{
		q: 'Which formats and encodings does it support?',
		a: 'CSV, TSV, and plain text with automatic detection of encoding, separator, quotes, escape character, decimal mark, and header row. Encodings include UTF-8/16/32, MacRoman, Windows Latin 1 and 2, GBK, ISO Latin 2, Cyrillic, Greek, Turkish, and Japanese. XLSX, ODS, JSON, and database formats are not supported.'
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
		a: 'Windows and Linux will be written in Rust. Android is planned with no language chosen yet. Each platform needs its own editor UI, file handling, and tests, so no dates are promised until a build exists; follow the repository for announcements.'
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
