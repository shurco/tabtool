export type ComparisonRow = {
	label: string;
	tabtool: string;
	them: string;
};

export type Comparison = {
	slug: string;
	name: string;
	vendor: string;
	site: string;
	license: string;
	price: string;
	platforms: string;
	metaTitle: string;
	metaDescription: string;
	summary: string;
	intro: string[];
	rows: ComparisonRow[];
	stronger: string[];
	different: string[];
	verdict: string;
	sources: { label: string; url: string }[];
};

export const comparisons: Comparison[] = [
	{
		slug: 'tabletool',
		name: 'TableTool by TablePlus',
		vendor: 'TablePlus Inc.',
		site: 'https://tabletool.io',
		license: 'Closed source',
		price: '$29 perpetual for 2 Macs with 1 year of updates',
		platforms: 'macOS 10.13 or later',
		metaTitle: 'TabTool vs TableTool (TablePlus): CSV editors compared',
		metaDescription:
			'TableTool by TablePlus is a paid CSV client with SQL joins. TabTool is an MIT-licensed macOS and iOS editor. Compare price, features, privacy, and large files.',
		summary:
			'A paid, polished CSV browser with SQL queries. TabTool is an open-source editor with an iPhone app and a documented large-file mode.',
		intro: [
			'TableTool by TablePlus is the commercial option most people find first: a native Mac app from the team behind TablePlus, sold as a one-time purchase with a year of updates. Its distinguishing feature is a built-in SQL query window that can join several CSV files at once.',
			'TabTool takes the opposite approach. It is a single-purpose editor, MIT-licensed, with no query language and no paid tier. The parts that are hard to get right — format detection, structural editing, and multi-gigabyte files — are the parts it spends its complexity on.'
		],
		rows: [
			{ label: 'License', tabtool: 'MIT, source available', them: 'Closed source' },
			{ label: 'Price', tabtool: 'Free on the App Store', them: '$29 for 2 Macs, 1 year of updates' },
			{ label: 'Platforms', tabtool: 'macOS 14+, iOS 17+', them: 'macOS 10.13+' },
			{ label: 'iPhone and iPad app', tabtool: 'Yes, same CSV core', them: 'No' },
			{ label: 'SQL across CSV files', tabtool: 'No', them: 'Yes' },
			{ label: 'Large-file mode', tabtool: 'Memory-mapped index; 5 GiB tested in 2.7 s', them: 'Vendor claims “a GB CSV within a few seconds”' },
			{ label: 'Editing', tabtool: 'Cells, rows, columns, undo/redo', them: 'Offered; the product is positioned as a CSV browser' },
			{ label: 'Filters', tabtool: 'Multi-criteria filters on iOS, sort on both', them: 'Multiple filters, export of filtered results' },
			{ label: 'Privacy', tabtool: 'Sandbox, no network entitlement, no analytics', them: 'Device hash for licensing, crash reports with opt-out' },
			{ label: 'Source code', tabtool: 'Readable and buildable', them: 'Issue tracker only' }
		],
		stronger: [
			'SQL queries and joins across multiple CSV files, which TabTool does not have at all.',
			'A paid product with a company behind it, including licensing support and release notes.',
			'Filtered export is a first-class flow on the Mac; TabTool keeps filters on iOS.'
		],
		different: [
			'MIT license: the parser, the SwiftUI shell, both grids, and the tests are in the repository.',
			'An iPhone and iPad app that opens the same files with the same detection.',
			'A published large-file design: memory-mapped indexing, streamed saves, and a measured 5 GiB run.',
			'No licensing code, no device hash, and no network entitlement in the app.'
		],
		verdict:
			'Choose TableTool if you need SQL over several CSVs and prefer a supported commercial purchase. Choose TabTool if you want an editor you can read and change, a matching iPhone app, and large-file behavior you can verify in the source.',
		sources: [
			{ label: 'TableTool product page', url: 'https://tabletool.io' },
			{ label: 'TableTool pricing', url: 'https://tabletool.io/pricing' },
			{ label: 'TablePlus privacy policy', url: 'https://tableplus.com/privacy' }
		]
	},
	{
		slug: 'tabletool-open-source',
		name: 'TableTool by Jakob Egger',
		vendor: 'Jakob Egger',
		site: 'https://github.com/jakob/TableTool',
		license: 'MIT',
		price: 'Free on the Mac App Store',
		platforms: 'macOS 10.8 or later',
		metaTitle: 'TabTool vs the original open-source TableTool',
		metaDescription:
			'Both are MIT-licensed Mac CSV editors. The original TableTool has not shipped since 2017. Compare large-file support, filters, maintenance, and the iPhone app.',
		summary:
			'The original MIT-licensed Mac CSV editor, unchanged since 2017. TabTool is the same idea with a maintained codebase and an iOS app.',
		intro: [
			'Jakob Egger’s TableTool is the open-source CSV editor many Mac users still have in their Applications folder. It is MIT-licensed, under a megabyte, and free on the Mac App Store. It detects delimiters and encodings, edits cells, and converts between formats.',
			'The last App Store release was 1.2.1 in July 2017, and the repository has been quiet since 2020. TabTool shares the name and the license, not the code: it is a new Swift 6 codebase with a separate CSV core, a large-file mode, and an iOS editor.'
		],
		rows: [
			{ label: 'License', tabtool: 'MIT', them: 'MIT' },
			{ label: 'Price', tabtool: 'Free on the App Store', them: 'Free' },
			{ label: 'Platforms', tabtool: 'macOS 14+, iOS 17+', them: 'macOS 10.8+' },
			{ label: 'Last release', tabtool: '1.0 in development', them: '1.2.1, July 2017' },
			{ label: 'Last commit', tabtool: 'Active', them: 'April 2020' },
			{ label: 'Large-file mode', tabtool: 'Memory-mapped, background index, streamed save', them: 'Not documented' },
			{ label: 'Filters and sorting', tabtool: 'Sort on both platforms, filters on iOS', them: 'Not documented' },
			{ label: 'iPhone and iPad app', tabtool: 'Yes', them: 'No' },
			{ label: 'Binary size', tabtool: 'Not published yet', them: '837.8 KB on the App Store' },
			{ label: 'Privacy', tabtool: 'No network entitlement, no analytics', them: 'No account or network features listed' }
		],
		stronger: [
			'Tiny footprint: under one megabyte, with a decade of real-world use.',
			'Available today on the Mac App Store, for free.',
			'A simpler feature set if all you need is a quick look and a format conversion.'
		],
		different: [
			'Maintained code: Swift 6, strict concurrency, 254 core tests and 320 app unit tests in the repository.',
			'Documented large-file mode with a 5 GiB benchmark and a streamed save path.',
			'Sorting, iOS filters, drag-and-drop reordering, and an iPhone editor.',
			'Format detection with an override bar for every detected value.'
		],
		verdict:
			'If you need a free Mac CSV viewer today and never touch huge files, the original TableTool still works. If you want an editor that is maintained, handles multi-gigabyte files, and runs on iPhone, TabTool is the successor in spirit.',
		sources: [
			{ label: 'jakob/TableTool on GitHub', url: 'https://github.com/jakob/TableTool' },
			{ label: 'Table Tool on the Mac App Store', url: 'https://apps.apple.com/us/app/table-tool/id1122008420' }
		]
	},
	{
		slug: 'modern-csv',
		name: 'Modern CSV',
		vendor: 'Modern CSV',
		site: 'https://www.moderncsv.com',
		license: 'Closed source, free tier',
		price: 'Free tier; $39 Personal or $59 Business, one-time',
		platforms: 'Windows, macOS, Linux',
		metaTitle: 'TabTool vs Modern CSV: features, platforms, price',
		metaDescription:
			'Modern CSV is a cross-platform workbench with pivot tables and statistics. TabTool is an open-source Mac and iOS editor. Compare features, price, and platforms.',
		summary:
			'A cross-platform CSV workbench with analysis tools. TabTool is a narrower Mac and iOS editor, and open source.',
		intro: [
			'Modern CSV is the broadest tool in this comparison: Windows, macOS, and Linux, with a free tier and paid Personal and Business licenses. Its feature list goes beyond editing into pivot tables, column statistics, deduplication, regex find and replace, and JSON/XML conversion.',
			'TabTool does none of that. It opens delimited text, lets you edit it, and writes it back in the format you choose. The trade is scope for depth: a native Apple app, a documented large-file mode, and an MIT-licensed codebase without a paid tier.'
		],
		rows: [
			{ label: 'License', tabtool: 'MIT', them: 'Closed source' },
			{ label: 'Price', tabtool: 'Free on the App Store', them: 'Free tier; $39/$59 one-time' },
			{ label: 'Platforms', tabtool: 'macOS 14+, iOS 17+', them: 'Windows, macOS, Linux' },
			{ label: 'iPhone and iPad app', tabtool: 'Yes', them: 'No' },
			{ label: 'Pivot tables and statistics', tabtool: 'No', them: 'Yes' },
			{ label: 'JSON/XML import and export', tabtool: 'No', them: 'Yes' },
			{ label: 'Large files', tabtool: '5 GiB tested, 2.7 s index, ~91 MB RAM', them: 'Read-only mode; vendor claims a small footprint' },
			{ label: 'Regex find and replace', tabtool: 'No', them: 'Yes' },
			{ label: 'Editing and undo', tabtool: 'Full structural editing, undo/redo', them: 'Yes' },
			{ label: 'Privacy', tabtool: 'No network entitlement, no analytics', them: 'Local processing, no cloud' }
		],
		stronger: [
			'Analysis: pivot tables, column statistics, unique values, and histograms.',
			'More interchange formats, including JSON and XML, plus PDF printing.',
			'Three desktop operating systems, with a free tier to evaluate.',
			'Business licensing and a lifetime license model.'
		],
		different: [
			'MIT license, with the CSV core and both editors in the repository.',
			'A native SwiftUI and AppKit app, plus an iPhone and iPad editor.',
			'A published large-file benchmark with memory figures, not only a claim.',
			'No account, no license key, and no network code in the app.'
		],
		verdict:
			'Modern CSV is the better pick if you need analysis, JSON/XML, or Linux and Windows. TabTool is the better pick if you want a small, open, native Apple editor that handles very large delimited files and also runs on iPhone.',
		sources: [
			{ label: 'Modern CSV', url: 'https://www.moderncsv.com' },
			{ label: 'Modern CSV pricing', url: 'https://www.moderncsv.com/buy/' }
		]
	},
	{
		slug: 'csviewer',
		name: 'CSViewer',
		vendor: 'EasyMorph',
		site: 'https://csviewer.com',
		license: 'Closed source, free',
		price: 'Free, including commercial use',
		platforms: 'Windows (64-bit) only',
		metaTitle: 'TabTool vs CSViewer: Mac editing vs Windows profiling',
		metaDescription:
			'CSViewer is a free Windows-only CSV viewer with column profiling. TabTool is an open-source macOS and iOS CSV editor. Compare platforms, editing, and large files.',
		summary:
			'A free Windows-only CSV viewer built for profiling. TabTool is a Mac and iOS editor, and open source.',
		intro: [
			'CSViewer is free, including for commercial use, and built for looking at data rather than changing it. It loads millions of rows, profiles every column, and draws charts and crosstabs. It runs on 64-bit Windows only; the vendor’s site confirms there is no Mac version.',
			'That makes the comparison simple. If your data lives on Windows and you need a free profiler, CSViewer is a strong fit. If you need to edit cells on a Mac or an iPhone, it is the wrong tool, and TabTool is built for exactly that.'
		],
		rows: [
			{ label: 'License', tabtool: 'MIT', them: 'Closed source' },
			{ label: 'Price', tabtool: 'Free on the App Store', them: 'Free, including commercial use' },
			{ label: 'Platforms', tabtool: 'macOS 14+, iOS 17+', them: 'Windows 64-bit only' },
			{ label: 'Cell editing', tabtool: 'Yes, with undo', them: 'Positioned as a viewer; editing is not listed' },
			{ label: 'Column profiling', tabtool: 'No', them: 'Nulls, uniques, min/max, aggregates, charts' },
			{ label: 'iPhone and iPad app', tabtool: 'Yes', them: 'No' },
			{ label: 'Large files', tabtool: '5 GiB tested in 2.7 s', them: 'Millions of rows in a viewer' },
			{ label: 'Source code', tabtool: 'MIT, public', them: 'Closed' },
			{ label: 'Telemetry', tabtool: 'None', them: 'None advertised; works air-gapped' }
		],
		stronger: [
			'Column profiling and charts, built for exploration rather than editing.',
			'Free for commercial use with no purchase step.',
			'Saved views and multi-file collections for repeat analysis.'
		],
		different: [
			'Runs on macOS and iOS, not Windows.',
			'Edits cells, rows, and columns, and writes the file back.',
			'Open source under MIT, so the format handling can be audited.',
			'Designed around opening one large file quickly rather than profiling many.'
		],
		verdict:
			'CSViewer and TabTool barely compete: different operating systems, different jobs. Use CSViewer to profile data on Windows. Use TabTool to edit a CSV on a Mac or iPhone.',
		sources: [
			{ label: 'CSViewer', url: 'https://csviewer.com' }
		]
	},
	{
		slug: 'tablecruncher',
		name: 'Tablecruncher',
		vendor: 'Tablecruncher',
		site: 'https://tablecruncher.com',
		license: 'GPL-3.0-or-later',
		price: 'Free',
		platforms: 'macOS 10.15+, Windows, Linux (beta)',
		metaTitle: 'TabTool vs Tablecruncher: two open-source CSV editors',
		metaDescription:
			'Tablecruncher is a GPL-3.0-or-later CSV editor with JavaScript macros on macOS, Windows, and Linux. TabTool is MIT-licensed with a native iOS app. Compare performance and scope.',
		summary:
			'An open-source CSV editor with JavaScript macros and three desktop platforms. TabTool is MIT-licensed and Apple-only, for now.',
		intro: [
			'Tablecruncher became free and open source in April 2025. It is GPL-3.0-or-later, written in C++ with FLTK, and runs on macOS, Windows, and Linux (beta). It opens files over 2 GB, offers JavaScript macros, regex find and replace, and JSON export, and its own FAQ admits the interface will never win a design prize.',
			'TabTool is the other open-source option: MIT rather than GPL, Swift rather than C++, native AppKit and SwiftUI rather than FLTK, and macOS plus iOS. Both projects care about large files, which makes this the most direct comparison on the site.'
		],
		rows: [
			{ label: 'License', tabtool: 'MIT', them: 'GPL-3.0-or-later' },
			{ label: 'Price', tabtool: 'Free on the App Store', them: 'Free' },
			{ label: 'Platforms', tabtool: 'macOS 14+, iOS 17+', them: 'macOS 10.15+, Windows, Linux beta' },
			{ label: 'Language and UI', tabtool: 'Swift, SwiftUI + AppKit', them: 'C++17, FLTK' },
			{ label: 'iPhone and iPad app', tabtool: 'Yes', them: 'No' },
			{ label: 'Large files', tabtool: '5 GiB tested, 2.7 s index, ~91 MB RAM', them: 'Over 2 GB and 15M+ rows; 100 MB in under 5 s (vendor)' },
			{ label: 'Macros', tabtool: 'No', them: 'JavaScript' },
			{ label: 'JSON export', tabtool: 'No', them: 'Yes' },
			{ label: 'License for forks', tabtool: 'Permissive; keep the notice', them: 'Copyleft; derivative work stays GPL' }
		],
		stronger: [
			'Windows and Linux builds, including a beta for Linux.',
			'JavaScript macros and regex find and replace.',
			'GPL community and an existing multi-year release history.'
		],
		different: [
			'MIT license: easier to embed, relicense, or ship inside another product.',
			'Native Apple UI and an iOS editor that share one CSV core.',
			'A measured 5 GiB run with memory and save timings published in the README.',
			'A narrower scope: no macros, no scripting runtime, fewer moving parts.'
		],
		verdict:
			'For a free editor with macros on Windows or Linux, Tablecruncher is the practical choice. For an Apple-native editor with an iPhone app and an MIT license, TabTool is the closer fit. Both are honest open-source alternatives to the paid tools.',
		sources: [
			{ label: 'Tablecruncher', url: 'https://tablecruncher.com' },
			{ label: 'Tablecruncher on GitHub', url: 'https://github.com/Tablecruncher/tablecruncher' }
		]
	}
];

export const comparisonBySlug = (slug: string) => comparisons.find((c) => c.slug === slug);
