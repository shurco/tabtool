# CSV format and editor behavior

TabTool stores cells as raw text. Numeric classification is used only for alignment, the header, and decimal mark conversion; `Decimal` does not become the data model and does not normalize values on save.

## Configuration

`CSVConfiguration`:

- `encoding` — file encoding;
- `columnSeparator` — `,`, `;`, tab, or `|`;
- `quoteCharacter` — `"` or an empty string;
- `escapeCharacter` — `"`, `\`, or an empty string;
- `decimalMark` — `.` or `,`;
- `firstRowAsHeader` — the first row is a header.

## Auto-detection

The heuristic uses 11 configurations in a fixed order, up to five rows per configuration, scores for rows read, equal length, numeric cells, and an empty/numeric first row. On a tie, the configuration checked earlier wins.

Encodings: UTF-8, Windows CP1252, MacRoman, GBK. If the system language starts with `zh`, GBK is checked before CP1252.

A header is considered a header if the first row is not empty, not numeric, and not shorter/longer than the rest according to the heuristic rules. An empty first-row cell also clears the header flag. Detection never throws: if every quoted candidate fails to parse, a no-quote candidate can still be selected, and a later parse can report the exact error.

Malformed quoted input such as `"1","""` is rejected by quoted configurations with `malformedQuotedField`; it may still open in no-quote mode, where quotes are literal text. The user can then choose the right quote style or detect automatically.

## Reading

- Line separators: CRLF, CR, LF, U+0085, U+2028, U+2029.
- An empty line is a row with one empty cell.
- An empty file is one empty row (a 1×1 table).
- A trailing newline does not create an extra row.
- Trailing empty fields are preserved.
- Quoted fields may contain the delimiter and line breaks.
- A doubled quote (`""`) and escaping via `\` are supported.
- A quote in an unquoted field is an error.
- An unterminated quote/escape at the end is an error.
- A UTF-8 BOM is stripped from the text and remembered; it is restored when saving as UTF-8.
- Invalid bytes for the selected encoding are a decoding error.

Typed errors: `cannotDecodeData`, `malformedQuotedField`, `quoteInUnquotedField`, `unencodableText`, `ambiguousOutput`.

## Writing

- With quotes enabled, every field is quoted.
- A quote is doubled or escaped with `\`.
- Rows are joined with `\n`; there is no trailing newline.
- The header is written first if it exists in the document.
- Column order is the visual table order.
- Text is not normalized: `001,50`, `+5`, `1e3` remain text.
- If text cannot be represented in the encoding, it is an error and the file is not written partially. Undo the edit that added the text, or export with an encoding that can represent it.
- No-quote mode: if a cell contains the delimiter or a line break, the write is ambiguous; autosave/Save are blocked. Undo the edit, or export with quotes on; Export requires confirmation.

## Decimal mark

- Changing the mark in the format panel replaces the old mark with the new one only in cells that are numeric by the old mark: `001,50` → `001.50`.
- Save adds no conversions: it writes the current text.
- Export converts a copy of the table from the document mark to the export mark.
- Paste converts numeric cells from the current locale's decimal separator to the document mark.
- Copy localizes numeric cells into the current locale.

## Header

- Header in the document format is semantics: whether the table has a header.
- Header in export is output only: the document headers, otherwise generated A, B, C…; disabled — no header is written.
- Enabling/disabling the header moves the first row between headers and data and is undone via undo.
- Rename Column changes one header name. It is available when exactly one column is selected. Without a header it turns Header on: every column gets its generated name (A, B, C…), the renamed column gets the new name, and all rows stay data rows (unlike the Header toggle, the first row is not consumed). Renaming to the column's own generated name without a header does nothing. Start it with a double-click on the column title, Rename Column in the header context menu, or Rename Column in the Table menu. Return or focus loss commits, Escape cancels.
- A header name may be empty (an empty header cell is valid CSV); names containing the separator, quotes, or line breaks are quoted on export like any cell.
- Renaming is a single undo step ("Rename Column"), keeps the active sort and column identities, and renaming to the same name records nothing.

## Format change and safety

- Encoding, delimiter, quotes, escape, and auto-detection re-read the file from disk.
- If there are unsaved edits — Cancel/Re-read confirmation; after success, a snapshot of the old state goes into undo.
- Parsing is transactional: on error the previous table and format remain, data is not erased.
- An Untitled document has nothing to re-read: settings affect writing only.
- Decimal/header change in memory and are undone via undo.

## Copy and paste

- Copy: rows or columns in visual order, TSV, numbers in the current locale, tabs/newlines inside cells replaced with spaces.
- Paste: TSV without quotes, insertion after the last selected row (or at the end), extra fields add columns, pasted rows are selected, the whole operation is one undo.
- Row drag & drop is only within the table; TSV is provided outward.

## Sorting and moving

- Sort Ascending and Sort Descending order the rows by one column. They are in the column header's context menu and in the Table menu (one column selected). Every column header also shows a sort arrow at its right edge; clicking it cycles that column through ascending, descending, and no sort without selecting the column.
- Add Column Left/Right are also in the column header's context menu, and Add Row Above/Below in the context menu of a data row. A right click selects the row or column under the pointer unless it is already selected, then inserts relative to the selection exactly like the Table menu.
- Comparison: empty cells (including whitespace-only ones) go last in both directions; numbers are compared by value (using the document decimal mark) and come before text; surrounding whitespace is ignored when comparing; text uses natural order (`item2` before `item10`). Descending is the exact reverse of the non-empty cells, so text comes before numbers. Equal cells keep their current relative order in both directions.
- Sorting rewrites the row order in the table, is a single undo step, and is saved like any other edit.
- Remove Sort restores the row order from before the first sort, keeping any cell edits made in between.
- Repeating a sort always recomputes the order, so Sort Ascending after editing cells re-sorts. If the order does not change, no undo step is recorded.
- Column moves (header drag, Move Column Left/Right) keep the sort. Every other structural or format change clears it (cell edits keep it): moving, adding, deleting, or pasting rows; adding, deleting, or pasting columns; format changes (such as the decimal mark); toggling the header; and re-reading the file.
- Move Row Up/Down and Move Column Left/Right (`⌥⌘` + arrow) move the selection one step, keeping gaps between selected items, and are disabled when the selection touches the edge.

## Large file mode

Files of 100 MiB (104,857,600 bytes) and more open in large file mode; the threshold is a setting (default 100 MiB, options 25, 50, 100, 250, 500, and 1024 MiB): the file is memory-mapped and indexed in the background instead of being parsed into memory. The rules above apply with these differences.

- Encodings: UTF-8, Windows Latin 1/2 (CP1252/CP1250), MacRoman, ISO Latin 2, Cyrillic (CP1251), Greek (CP1253), Turkish (CP1254), and GBK. UTF-16/32, Shift_JIS, EUC-JP, and ISO-2022-JP are refused with "This encoding cannot be opened in large file mode"; convert the file to UTF-8. The same applies when one of them is chosen in the format bar.
- The separator, quote, and escape must each be one ASCII character (or empty for quote and escape).
- Rows break at LF, CR, and CRLF only. U+0085, U+2028, and U+2029 are ordinary characters in large file mode (the in-memory reader treats them as line breaks).
- Format detection reads the first 1 MiB, cut back to the last row break outside quotes, and that sample must parse strictly or the file does not open. Later rows are indexed by quote parity and never stop the scan: a group of 64 rows that does not parse is shown with each row as one raw-text cell, and rows with bytes that are invalid in the encoding are shown with replacement characters. Neither can be edited.
- At most 2,048 columns are shown; wider rows keep their extra fields in the file untouched.
- Cells can be edited once indexing is complete, not while saving or after the file changed on disk. Header cells cannot be edited. The Header toggle only changes which row is the header. Undo and redo work as usual.
- Saving rewrites only the edited rows. Every other byte is copied unchanged: original quoting, line endings, ragged rows, and the BOM stay. An edited row is written again by the writer (every field quoted when quotes are on, short rows padded to the edited column) and keeps its own line terminator. Saving needs free disk space of about twice the file size. The file is never autosaved; closing or quitting with unsaved edits asks Save / Don't Save / Cancel.
- Not available: Cut, Paste, Delete; adding, deleting, moving, sorting, and renaming rows or columns; changing the decimal mark; Export; Duplicate. Copy is limited to 1,000,000 cells.
- Changing the encoding, separator, quote, or escape indexes the file again and drops unsaved edits and the undo history after confirmation.
- When the file changes on disk, a banner offers Reload. Rows already read stay visible; edits are off until the reload.

## Limitations

- No formulas, styles, charts, XLSX/ODS, or cloud sync.
- Files below the large file threshold (100 MiB by default) are held in memory completely; larger files use large file mode.
- External file changes are last-writer-wins; there is no separate conflict UI.
- CSV/TSV/TXT are opened, not arbitrary extensions.
