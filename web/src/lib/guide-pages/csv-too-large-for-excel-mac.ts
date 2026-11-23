import type { Guide } from '../guides';

export const guide: Guide = {
	slug: 'csv-too-large-for-excel-mac',
	title: 'CSV file too large for Excel on Mac',
	metaTitle: 'CSV too large for Excel on Mac: 6 ways to handle it',
	metaDescription:
		'Excel for Mac stops at 1,048,576 rows. Split, filter, query, or open the whole CSV with commands you can copy, and the limits of each way.',
	checked: '23 November 2026',
	lede: 'Excel for Mac stops at 1,048,576 rows per worksheet. Here is what to do with a CSV that has more, with every option and what it costs.',
	image: {
		src: '/img/app-large.webp',
		srcset: '/img/app-large-1200.webp 1200w, /img/app-large.webp 2400w',
		alt: 'TabTool for macOS in large-file mode showing a 1,000,000-row emoji CSV.',
		width: 2400,
		height: 1500
	},
	facts: [
		{ label: 'Rows per worksheet', value: '1,048,576' },
		{ label: 'Columns per worksheet', value: '16,384' },
		{
			label: 'Power Query on Mac',
			value:
				'Imports CSV, but cannot load into the Data Model, so the result goes to a sheet with the same row limit.'
		}
	],
	problem: [
		'Excel loads the first 1,048,576 rows of a CSV and drops the rest after a warning. The data is not lost on disk, but anything you save from that sheet contains only the part Excel kept.',
		'The limit comes from the worksheet, not from the file size or from your Mac. Every fix below either works on a smaller piece of the file or avoids the worksheet.'
	],
	options: [
		{
			name: 'Split the file',
			summary:
				'The built-in split command cuts the file into parts that each fit in Excel. The header row is written once, so each part gets it added back.',
			good: 'Needs no install. Every part opens in Excel or any other app.',
			cost: 'You work on parts, not on one file. A row with a quoted line break can be cut in half, because split counts lines, not records.',
			code: `head -n 1 big.csv > header.csv
tail -n +2 big.csv | split -l 1000000 - part_
for f in part_*; do cat header.csv "$f" > "$f.csv" && rm "$f"; done`,
			link: { label: 'SS64: macOS split', url: 'https://ss64.com/mac/split.html' }
		},
		{
			name: 'csvkit',
			summary:
				'A Python toolkit. csvgrep filters rows whose column matches a regular expression, csvstat counts rows and summarizes columns.',
			good: 'Readable commands and many other tools in the same kit, such as csvcut and csvsort.',
			cost: 'Written in Python, so it is slow on multi-gigabyte files.',
			code: `csvstat --count big.csv
csvgrep -c country -r '^Germany$' big.csv > germany.csv`,
			link: { label: 'csvkit csvgrep', url: 'https://csvkit.readthedocs.io/en/latest/scripts/csvgrep.html' }
		},
		{
			name: 'qsv',
			summary: 'A fast command-line tool for CSV written in Rust. It counts, filters, and selects without loading the whole file.',
			good: 'Fast on large files. One binary, no runtime.',
			cost: 'A command-line tool with many commands; the filter takes a regular expression, not a plain value.',
			code: `qsv count big.csv
qsv search -s country '^Germany$' big.csv -o germany.csv`,
			link: { label: 'qsv search', url: 'https://github.com/dathere/qsv/blob/master/docs/help/search.md' }
		},
		{
			name: 'DuckDB',
			summary:
				'An embedded SQL database. It reads a CSV file directly, and COPY writes any query result to a new, smaller CSV.',
			good: 'Full SQL, including grouping and joins, with no import step.',
			cost: 'You need to know SQL. The output is a new file, not an edit of the original.',
			code: `duckdb -c "SELECT count(*) FROM 'big.csv';"
duckdb -c "COPY (SELECT * FROM 'big.csv' WHERE country = 'Germany') TO 'germany.csv' (FORMAT CSV, HEADER);"`,
			link: { label: 'DuckDB CSV import', url: 'https://duckdb.org/docs/2.0/data/csv/overview.html' }
		},
		{
			name: 'Power Query in Excel for Mac',
			summary:
				'Excel for Mac can import a CSV through Power Query, so you can filter or remove columns before the data reaches the sheet.',
			good: 'Stays inside Excel, with steps you can refresh.',
			cost: 'Excel for Mac cannot load into the Data Model, so the result goes into a worksheet and the 1,048,576-row limit applies to it.',
			link: {
				label: 'Microsoft: Power Query in Excel for Mac',
				url: 'https://support.microsoft.com/en-us/office/import-and-shape-data-in-excel-for-mac-power-query-7b2f337d-e7d2-4fdf-bf00-3dfbb1e5e9c5'
			}
		},
		{
			name: 'TabTool',
			summary:
				'A free, MIT-licensed CSV editor. In large file mode it memory-maps the file and builds a row index in the background, so rows appear at once and you can scroll, jump, and edit cells with undo.',
			good: 'Opens the whole file and edits it in place. Save streams the file and rewrites only the edited rows. Nothing is autosaved.',
			cost: 'Large file mode has no structural commands, sorting, export, Duplicate, or decimal mark. Saving needs free disk space of about twice the file size.',
			link: { label: 'TabTool large files', url: 'https://github.com/shurco/tabtool#large-files' }
		}
	],
	benchmark: [
		{
			label: 'Benchmark script, 5.00 GiB file with 68.2 million rows',
			value: '45 ms to the first 64 rows, 1.97 s to a full index',
			note: './scripts/bench-large 5, release build, Apple M4 Pro, 24 GB, APFS SSD, file in the page cache. Generated CSV, 8 columns.'
		},
		{
			label: 'Release app, 5.0 GiB file with 77.7 million rows',
			value: 'Fully indexed 2.7 s after launch',
			note: 'The sandboxed app from the release build, not the script.'
		}
	],
	sources: [
		{
			label: 'Microsoft: Excel specifications and limits',
			url: 'https://support.microsoft.com/en-us/excel/excel-specifications-and-limits'
		},
		{
			label: 'Microsoft: Power Query in Excel for Mac',
			url: 'https://support.microsoft.com/en-us/office/import-and-shape-data-in-excel-for-mac-power-query-7b2f337d-e7d2-4fdf-bf00-3dfbb1e5e9c5'
		},
		{
			label: 'Microsoft Q&A: Excel for Mac and the Data Model (community answers)',
			url: 'https://learn.microsoft.com/en-us/answers/questions/5169428/importing-csvs-into-a-query-table-to-clean-using-p'
		},
		{ label: 'SS64: macOS split', url: 'https://ss64.com/mac/split.html' },
		{ label: 'csvkit csvgrep', url: 'https://csvkit.readthedocs.io/en/latest/scripts/csvgrep.html' },
		{ label: 'csvkit csvstat', url: 'https://csvkit.readthedocs.io/en/latest/scripts/csvstat.html' },
		{ label: 'qsv count', url: 'https://github.com/dathere/qsv/blob/master/docs/help/count.md' },
		{ label: 'qsv search', url: 'https://github.com/dathere/qsv/blob/master/docs/help/search.md' },
		{ label: 'DuckDB CSV import', url: 'https://duckdb.org/docs/2.0/data/csv/overview.html' },
		{ label: 'DuckDB COPY', url: 'https://duckdb.org/docs/2.0/sql/statements/copy.html' },
		{ label: 'TabTool large files and benchmark', url: 'https://github.com/shurco/tabtool#large-files' }
	]
};
