import type { Guide } from '../guides';

export const guide: Guide = {
	slug: 'semicolon-csv-mac',
	title: 'Open a semicolon-separated CSV on Mac',
	metaTitle: 'Open a semicolon-separated CSV on Mac: Excel, Numbers',
	metaDescription:
		'A CSV with semicolons may show up in one column. Why it happens, how to pick the separator in Excel and Numbers, and how to convert it to commas.',
	checked: '23 November 2026',
	lede: 'A CSV that uses semicolons may open with all its text in one column. The file is fine: the app may have assumed a comma. Here is why it happens and how to fix it.',
	image: {
		src: '/img/app-grid.webp',
		srcset: '/img/app-grid-1200.webp 1200w, /img/app-grid.webp 2400w',
		alt: 'TabTool for macOS showing a CSV with the detected format in the format bar.',
		width: 2400,
		height: 1500
	},
	facts: [
		{
			label: 'Why semicolons',
			value:
				'A community answer from a Microsoft MVP says Excel uses the list separator of the system region as the delimiter, and that where a comma is the decimal mark, the list separator is a semicolon.'
		},
		{
			label: 'Excel, import',
			value: 'On Excel for Mac, Data > From Text (Legacy) opens the Text Import Wizard, where you choose Delimited, pick the delimiter, and check a preview.'
		}
	],
	problem: [
		'A CSV is a text file, and the character between fields is a convention, not a rule. Files exported from Excel on a system where the comma is the decimal mark can use a semicolon, so a value like 3,5 can stay in one piece.',
		'An app that expects commas may then read each line as one field. Either tell the app which separator to use, or write a copy of the file with commas.'
	],
	options: [
		{
			name: 'Excel: import with a chosen delimiter',
			summary:
				'Choose Data > From Text (Legacy) and select the file. In the Text Import Wizard, choose Delimited, then under Delimiters pick the separator, or tick Other and type it. Check the Preview of selected data, then Finish.',
			good: 'Stays in Excel, and the preview shows the result before anything loads.',
			cost: 'The file is loaded into a worksheet, with its row limit.',
			link: {
				label: 'Microsoft: Import and shape data in Excel for Mac',
				url: 'https://support.microsoft.com/en-us/office/import-and-shape-data-in-excel-for-mac-power-query-7b2f337d-e7d2-4fdf-bf00-3dfbb1e5e9c5'
			}
		},
		{
			name: 'Numbers: Import Settings',
			summary:
				'Open the file in Numbers, select the table, then in the Format sidebar choose the Table tab and Adjust Import Settings. On the Delimited tab, pick a delimiter under Separate Values Using or enter your own. The same window has a Text Encoding menu. Then click Update Table.',
			good: 'Can set both the delimiter and the encoding after the file is open.',
			cost: 'Apple documents this for delimited text files and does not name CSV on that page. You cannot adjust import settings after you edit the imported table.',
			link: {
				label: 'Apple: Import an Excel or text file in Numbers',
				url: 'https://support.apple.com/guide/numbers/import-an-excel-or-text-file-tan9f3c54bdc/mac'
			}
		},
		{
			name: 'qsv',
			summary:
				'qsv fmt reads with one delimiter and writes with another. -d is the input delimiter and -t the output delimiter.',
			good: 'A CSV parser, so it treats the file as records and fields, not as raw text.',
			cost: 'A command-line tool to install. Values are kept; quoting in the output follows the quoting options of qsv.',
			code: `qsv fmt -d ';' -t ',' semicolon.csv -o comma.csv`,
			link: { label: 'qsv fmt', url: 'https://github.com/dathere/qsv/blob/master/docs/help/fmt.md' }
		},
		{
			name: 'DuckDB',
			summary:
				'read_csv takes delim for the input separator, and COPY writes a new file with the delimiter you give it. all_varchar keeps every column as text, so values such as 007 are not turned into numbers.',
			good: 'Also lets you filter or select columns in the same command.',
			cost: 'You write SQL, and the output is a new file.',
			code: `duckdb -c "COPY (SELECT * FROM read_csv('semicolon.csv', delim=';', header=true, all_varchar=true)) TO 'comma.csv' (FORMAT CSV, DELIMITER ',', HEADER);"`,
			link: { label: 'DuckDB CSV import', url: 'https://duckdb.org/docs/2.0/data/csv/overview.html' }
		},
		{
			name: 'sed or tr',
			summary:
				"Replacing every ; with , in a text stream is quick, but it is not a CSV conversion. Do not use it on files with quoted fields.",
			good: 'Nothing to install.',
			cost: 'It also changes semicolons inside quoted fields, and a decimal comma then splits a value into two fields. Use qsv or DuckDB when the file has quotes.'
		},
		{
			name: 'TabTool',
			summary:
				'A free, MIT-licensed CSV editor. It reads and writes comma, semicolon, tab, or pipe separated files, detects the separator when you open a file, and shows it in the format bar. Choosing another separator re-reads the file.',
			good: 'No import step. Export can write a different delimiter, encoding, quote style, and decimal mark. Saving keeps the file as it was opened.',
			cost: 'No formulas, styles, or charts. Changing the format of an open file asks before it discards unsaved edits. Export writes no BOM and uses LF line endings.',
			link: { label: 'TabTool format detection', url: 'https://github.com/shurco/tabtool/blob/HEAD/docs/csv-format.md' }
		}
	],
	sources: [
		{
			label: 'Microsoft: Import and shape data in Excel for Mac',
			url: 'https://support.microsoft.com/en-us/office/import-and-shape-data-in-excel-for-mac-power-query-7b2f337d-e7d2-4fdf-bf00-3dfbb1e5e9c5'
		},
		{
			label: 'Microsoft Q&A: saving as comma delimited ends up as semicolon (community answers)',
			url: 'https://learn.microsoft.com/en-us/answers/questions/5148933/saving-as-cvs-comma-delimited-but-ends-up-as-a-sem'
		},
		{
			label: 'Apple: Import an Excel or text file in Numbers',
			url: 'https://support.apple.com/guide/numbers/import-an-excel-or-text-file-tan9f3c54bdc/mac'
		},
		{ label: 'qsv fmt', url: 'https://github.com/dathere/qsv/blob/master/docs/help/fmt.md' },
		{ label: 'DuckDB CSV import', url: 'https://duckdb.org/docs/2.0/data/csv/overview.html' },
		{ label: 'DuckDB COPY', url: 'https://duckdb.org/docs/2.0/sql/statements/copy.html' },
		{ label: 'TabTool CSV format and detection', url: 'https://github.com/shurco/tabtool/blob/HEAD/docs/csv-format.md' }
	]
};
