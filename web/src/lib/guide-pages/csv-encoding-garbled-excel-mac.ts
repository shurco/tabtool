import type { Guide } from '../guides';

export const guide: Guide = {
	slug: 'csv-encoding-garbled-excel-mac',
	title: 'Fix garbled characters in a CSV in Excel for Mac',
	metaTitle: 'Fix garbled characters in a CSV in Excel for Mac',
	metaDescription:
		'Accents and symbols turn into odd characters when a CSV opens in Excel. Why it happens, and how to fix it with File origin, iconv, or a UTF-8 BOM.',
	checked: '23 November 2026',
	lede: 'When a CSV shows odd characters instead of accents or symbols, the text is usually fine and the app read the bytes with the wrong encoding. Here is how to fix it, and what each way costs.',
	image: {
		src: '/img/app-grid.webp',
		srcset: '/img/app-grid-1200.webp 1200w, /img/app-grid.webp 2400w',
		alt: 'TabTool for macOS showing a CSV with emoji and accented text.',
		width: 2400,
		height: 1500
	},
	facts: [
		{
			label: 'Excel opens UTF-8 normally',
			value: 'If the file was saved with a BOM. Microsoft gives other ways to open one without a BOM.'
		},
		{
			label: 'BOM bytes',
			value: 'A UTF-8 BOM is the three bytes EF BB BF at the start of a file.'
		},
		{
			label: 'Excel for Mac import',
			value: 'Data > From Text (Legacy) opens the Text Import Wizard, where File origin sets the character set.'
		}
	],
	problem: [
		'A CSV is bytes. The encoding says which bytes mean which character. If the app guesses a different encoding than the one the file was written in, accented letters and symbols turn into other characters. The data is not changed on disk.',
		'Microsoft says Excel opens a UTF-8 CSV normally if it was saved with a BOM, and gives other ways to open it if not. A UTF-8 file without a BOM is the usual case behind garbled text. A file in a legacy encoding that Excel does not expect gives the same symptom.'
	],
	options: [
		{
			name: 'Excel: import with File origin',
			summary:
				'On Excel for Mac, choose Data > From Text (Legacy), open the file in the Finder dialog, and in the Text Import Wizard set File origin to the character set of the file. For a UTF-8 file, pick the UTF-8 entry.',
			good: 'Fixes the display without changing the file.',
			cost: "You have to know the encoding. Microsoft's page does not list the available character sets, and says to leave File origin at its default in most cases.",
			link: {
				label: 'Microsoft: Import and shape data in Excel for Mac',
				url: 'https://support.microsoft.com/en-us/office/import-and-shape-data-in-excel-for-mac-power-query-7b2f337d-e7d2-4fdf-bf00-3dfbb1e5e9c5'
			}
		},
		{
			name: 'Excel: save as CSV UTF-8',
			summary:
				"If you made the file in Excel, Save As offers CSV UTF-8 (Comma delimited) (.csv), which writes the sheet's data as UTF-8 text. Microsoft's Excel for Mac page marks this format as Excel 2016 for Mac only; check that it appears in your version.",
			good: 'The file you pass on is UTF-8, which most other tools read.',
			cost: 'Only works for a file Excel can already show correctly. It saves the active sheet, and formulas and formatting are not kept.',
			link: {
				label: 'Microsoft: File formats supported in Excel for Mac',
				url: 'https://support.microsoft.com/en-us/excel/file-formats-supported-in-excel-for-mac'
			}
		},
		{
			name: 'iconv',
			summary:
				'iconv ships with macOS. -f names the encoding the file has now and -t the encoding you want. Replace WINDOWS-1252 with the real source encoding; iconv -l lists the names.',
			good: 'Needs no install, and writes a new UTF-8 file next to the original.',
			cost: 'You must name the right source encoding. A wrong guess still produces a file, with the wrong characters in it. iconv does not add a BOM.',
			code: `iconv -f WINDOWS-1252 -t UTF-8 in.csv > out.csv`,
			link: { label: 'SS64: macOS iconv', url: 'https://ss64.com/mac/iconv.html' }
		},
		{
			name: 'Add a UTF-8 BOM',
			summary:
				'If the file is already UTF-8 and has no BOM, write the three BOM bytes in front of it. Microsoft says Excel opens a UTF-8 CSV normally when it has a BOM.',
			good: 'One command, and the text is unchanged.',
			cost: 'Run it once: a second run adds a second BOM. Some tools do not expect a BOM at the start of a file.',
			code: `printf '\\xEF\\xBB\\xBF' | cat - in.csv > out.csv`,
			link: {
				label: 'Microsoft: Opening CSV UTF-8 files correctly in Excel',
				url: 'https://support.microsoft.com/en-us/office/opening-csv-utf-8-files-correctly-in-excel-8a935af5-3416-4edd-ba7e-3dfd2bc4a032'
			}
		},
		{
			name: 'TabTool',
			summary:
				'A free, MIT-licensed CSV editor. It detects the encoding on open from UTF-8, Windows CP1252, MacRoman, and GBK, and shows it in the format bar. If the text looks wrong, choose another encoding there: TabTool re-reads the file from disk with it, and asks before it drops unsaved edits.',
			good: 'You see the result at once, for any of the supported encodings, including Windows-1250 and Cyrillic, which detection does not try.',
			cost: 'Saving keeps the encoding and the UTF-8 BOM the file was opened with. Export can write another encoding, but never adds a UTF-8 BOM to a file that had none. So TabTool will not fix an Excel problem that needs a BOM; use the BOM command above for that.',
			link: { label: 'TabTool CSV format and detection', url: 'https://github.com/shurco/tabtool/blob/HEAD/docs/csv-format.md' }
		}
	],
	sources: [
		{
			label: 'Microsoft: Opening CSV UTF-8 files correctly in Excel',
			url: 'https://support.microsoft.com/en-us/office/opening-csv-utf-8-files-correctly-in-excel-8a935af5-3416-4edd-ba7e-3dfd2bc4a032'
		},
		{
			label: 'Microsoft: Import and shape data in Excel for Mac',
			url: 'https://support.microsoft.com/en-us/office/import-and-shape-data-in-excel-for-mac-power-query-7b2f337d-e7d2-4fdf-bf00-3dfbb1e5e9c5'
		},
		{
			label: 'Microsoft: File formats supported in Excel for Mac',
			url: 'https://support.microsoft.com/en-us/excel/file-formats-supported-in-excel-for-mac'
		},
		{ label: 'Unicode FAQ: UTF-8, UTF-16, UTF-32 and BOM', url: 'https://www.unicode.org/faq/utf_bom.html' },
		{ label: 'SS64: macOS iconv', url: 'https://ss64.com/mac/iconv.html' },
		{ label: 'TabTool CSV format and detection', url: 'https://github.com/shurco/tabtool/blob/HEAD/docs/csv-format.md' }
	]
};
