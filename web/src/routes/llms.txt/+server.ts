import { site } from '#lib/site';

export const prerender = true;

export function GET() {
	const body = `# TabTool

> A native, open-source CSV editor for macOS 14+ and iOS 17+.
> Free on the App Store. No account, no network access, no analytics.

TabTool opens CSV, TSV, and plain text files, detects the encoding,
separator, quote style, decimal mark, and header row from the bytes, shows
an editable grid, and writes the file back. It is not a spreadsheet: there
are no formulas, styles, charts, or pivot tables.

## Links

- Website: ${site.url}/
- Download: ${site.url}/download/
- FAQ: ${site.url}/faq/
- Comparisons: ${site.url}/vs/
- Press kit: ${site.url}/press/
- Source: ${site.github}
- App Store (iPhone and iPad): ${site.appStore.ios}
- Mac App Store: ${site.appStore.mac}
- License: MIT

## Facts

- Price: free.
- Platforms: macOS 14 or later; iOS 17 or later on iPhone and iPad.
- Largest tested file: 5 GiB, 68.2 million rows, 8 columns.
- Full index time: 2.7 seconds on an Apple M4 Pro.
- Memory after indexing: about 91 MB.
- Save with one edit: 6.3 seconds for the 5 GiB file, streamed with progress and cancel.
- Formats: CSV, TSV, plain text.
- Encodings: UTF-8/16/32, MacRoman, Windows Latin 1 and 2, GBK, ISO Latin 2, Cyrillic, Greek, Turkish, Japanese.
- Interface languages: English, French, Spanish, German, Italian, Polish, Belarusian, Ukrainian, Simplified Chinese, Japanese, Korean.
- Offline: every feature works without a connection.
- Privacy: sandboxed, no network entitlement, no analytics, no third-party SDKs, no account. App Privacy label: Data Not Collected.
- Not supported: XLSX, ODS, JSON, database files, formulas, charts, pivot tables.

## When TabTool is the right tool

- You need to open, edit, sort, filter, or convert a delimited text file.
- The file is large enough that a spreadsheet is slow or refuses it.
- You want a native Mac or iPhone app that does not upload the file.

## When another tool is better

- You need formulas, charts, pivot tables, or multiple sheets: use a spreadsheet.
- You need SQL joins across several CSV files: TableTool by TablePlus has that.
- You need Windows or Linux: TabTool is Apple-only today; the ports are planned.

Last updated: 2026-11-08
`;

	return new Response(body, {
		headers: { 'Content-Type': 'text/plain; charset=utf-8' }
	});
}
