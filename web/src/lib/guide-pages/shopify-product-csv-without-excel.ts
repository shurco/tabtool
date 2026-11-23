import type { Guide } from '../guides';

export const guide: Guide = {
	slug: 'shopify-product-csv-without-excel',
	title: 'Edit a Shopify product CSV without Excel breaking it',
	metaTitle: 'Edit a Shopify product CSV without Excel breaking it',
	metaDescription:
		'Excel can drop leading zeros and round long numbers in a product CSV. What Shopify and Microsoft document, and ways to edit the file safely on a Mac.',
	checked: '23 November 2026',
	lede: 'A product CSV for Shopify holds SKUs, barcodes, and descriptions that a spreadsheet may read as numbers. Here is what Shopify and Microsoft say, and ways to edit the file on a Mac without changing values.',
	image: {
		src: '/img/app-grid.webp',
		srcset: '/img/app-grid-1200.webp 1200w, /img/app-grid.webp 2400w',
		alt: 'TabTool for macOS showing a CSV where every cell is kept as text.',
		width: 2400,
		height: 1500
	},
	facts: [
		{
			label: 'Shopify file format',
			value: 'Save the CSV in UTF-8 format using LF-style linefeeds.'
		},
		{
			label: 'Excel number precision',
			value: '15 digits. Microsoft says Excel removes leading zeros and converts large numbers to scientific notation.'
		},
		{
			label: 'Excel numbers of 16 digits or more',
			value: 'Microsoft says to use a text format for them.'
		},
		{
			label: 'Smart quotes',
			value: 'Shopify says Excel or another spreadsheet program can add curly quotes, which must be replaced with straight quotes.'
		}
	],
	problem: [
		'A product CSV is plain text: every value, including a SKU or a barcode, is a string of characters. A spreadsheet that opens the file may read a value as a number. Microsoft documents that Excel removes leading zeros, converts large numbers to scientific notation, and keeps 15 digits of precision. A barcode or SKU written back from such a sheet may not match what you started with.',
		"Shopify's current page says the first line must be the column headers it describes, and that an older template may use different column names. Shopify also says to save the file as UTF-8, and to replace curly quotes added by a spreadsheet. The options below differ in how much of that they leave to you."
	],
	options: [
		{
			name: 'Excel: import columns as text',
			summary:
				'Microsoft says that Get & Transform (Power Query) can format individual columns as text when you import data, and that numbers of 16 digits or more need a text format. Excel for Microsoft 365 and Excel 2024 also have settings for Automatic Data Conversion that can be turned off; see the Microsoft page for where they are.',
			good: 'Text is the format Microsoft documents for keeping leading zeros and numbers of 16 digits or more.',
			cost: "You set each affected column by hand every time you open a file. Microsoft's page gives no Mac-specific steps, so check the menus in your version.",
			link: {
				label: 'Microsoft: Keeping leading zeros and large numbers',
				url: 'https://support.microsoft.com/en-us/excel/keeping-leading-zeros-and-large-numbers'
			}
		},
		{
			name: 'Google Sheets',
			summary:
				'Shopify recommends using Google Sheets to view a formatted version of your CSV files, and its example file is meant to be opened in Google Sheets or another spreadsheet program.',
			good: "It is the tool Shopify's own help page points to for viewing the file.",
			cost: 'The Shopify page does not say how Google Sheets treats leading zeros or long numbers, so check a sample of SKUs and barcodes after you save. Your file goes to a web service.',
			link: {
				label: 'Shopify: Using CSV files to import and export products',
				url: 'https://help.shopify.com/en/manual/products/import-export/using-csv'
			}
		},
		{
			name: 'TabTool',
			summary:
				'A free, MIT-licensed CSV editor. Cells are text and are never converted: leading zeros, long numbers such as UPCs, and values like 1e3 stay as typed. A quoted field that contains line breaks, such as an HTML description, is read as one cell.',
			good: 'No import step and no number conversion. Saving keeps the file’s encoding, delimiter, UTF-8 BOM, and line endings (LF, CRLF, or CR), so an LF file stays LF.',
			cost: 'No formulas, styles, or charts. With quotes on, every field is written quoted, and Shopify’s page does not say whether that matters for import, so test with a small file first. TabTool does not check the file against Shopify’s column list.',
			link: { label: 'TabTool download notes', url: '/download/' }
		}
	],
	sources: [
		{
			label: 'Shopify: Using CSV files to import and export products',
			url: 'https://help.shopify.com/en/manual/products/import-export/using-csv'
		},
		{
			label: 'Shopify: Common import issues',
			url: 'https://help.shopify.com/en/manual/products/import-export/common-import-issues'
		},
		{
			label: 'Microsoft: Keeping leading zeros and large numbers',
			url: 'https://support.microsoft.com/en-us/excel/keeping-leading-zeros-and-large-numbers'
		},
		{
			label: 'Microsoft: Excel specifications and limits',
			url: 'https://support.microsoft.com/en-us/excel/excel-specifications-and-limits'
		},
		{ label: 'TabTool CSV format and behavior', url: 'https://github.com/shurco/tabtool/blob/HEAD/docs/csv-format.md' },
		{ label: 'TabTool README', url: 'https://github.com/shurco/tabtool#readme' }
	]
};
