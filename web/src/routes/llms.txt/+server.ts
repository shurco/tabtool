import { site } from '#lib/site';

export const prerender = true;

export function GET() {
	const body = `# TabTool

> A free, open-source CSV editor. Native Swift on macOS 14+ and iOS 17+ (App Store); Rust builds for Windows, Linux, and Android (GitHub Releases).
> No account, no network access, no analytics.

TabTool opens CSV, TSV, and plain text files, detects the encoding,
separator, quote style, decimal mark, and header row from the bytes, shows
an editable grid, and writes the file back or exports it to JSON or XML. It is not a spreadsheet: there
are no formulas, styles, charts, or pivot tables.

## Links

- Website: ${site.url}/
- Download: ${site.url}/download/
- FAQ: ${site.url}/faq/
- Comparisons: ${site.url}/vs/
- Guides: ${site.url}/guides/
- Press kit: ${site.url}/press/
- Source: ${site.github}
- Windows, Linux, and Android downloads: ${site.releases}
- App Store (iPhone and iPad): ${site.appStore.ios}
- Mac App Store: ${site.appStore.mac}
- License: MIT

## Facts

- Price: free.
- Platforms: macOS 14 or later; iOS 17 or later on iPhone and iPad; Windows 10 or 11 (x64, ARM64); Linux with glibc 2.35+ (x86_64, aarch64); Android 8.0 or later.
- Distribution: App Store for macOS and iOS. GitHub Releases for Windows (zip), Linux (tar.gz), and Android (signed APK), with SHA256SUMS. The Windows and Linux archives are not code-signed.
- Benchmark file: 5 GiB, 68.2 million rows, 8 columns; full index 1.97 seconds, 43 MiB.
- Release app: 5 GiB, 77.7 million rows; full index 2.7 seconds, about 91 MB.
- Save with one edit: 6.3 seconds for the 5 GiB file, streamed with progress and cancel.
- Formats: CSV, TSV, plain text. Export also writes JSON (array of objects) and XML.
- Encodings: UTF-8/16/32, MacRoman, Windows Latin 1 and 2, GBK, ISO Latin 2, Cyrillic, Greek, Turkish, Japanese.
- Interface languages: English, French, Spanish, German, Italian, Polish, Belarusian, Ukrainian, Simplified Chinese, Japanese, Korean.
- Offline: every feature works without a connection.
- Privacy: no analytics, no third-party SDKs, no account. Apple apps: sandboxed, no network entitlement, App Privacy label Data Not Collected. Windows, Linux, Android: no network code; the Android app does not request Internet access.
- Not supported: opening XLSX, ODS, JSON, or database files; formulas, charts, pivot tables.

## When TabTool is the right tool

- You need to open, edit, sort, filter, or convert a delimited text file.
- The file is large enough that a spreadsheet is slow or refuses it.
- You want an editor on Mac, iPhone, Windows, Linux, or Android that does not upload the file.

## When another tool is better

- You need formulas, charts, pivot tables, or multiple sheets: use a spreadsheet.
- You need SQL joins across several CSV files: TableTool by TablePlus has that.
- You need a code-signed Windows installer or a Google Play listing: the Windows build is an unsigned zip and Android is a sideloaded APK.

Last updated: 2026-11-23
`;

	return new Response(body, {
		headers: { 'Content-Type': 'text/plain; charset=utf-8' }
	});
}
