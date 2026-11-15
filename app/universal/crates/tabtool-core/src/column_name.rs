//! Spreadsheet-style column names.
//!
//! Index `0` is `A`, `25` is `Z`, `26` is `AA`, and so on.

/// Returns the name for a zero-based column index.
///
/// The sequence is `A…Z`, `AA…AZ`, `BA…`, like a spreadsheet. Any `usize`
/// index has a name.
pub fn column_name(index: usize) -> String {
    let mut value = index;
    let mut name = Vec::new();

    loop {
        // The remainder is always below 26, so it fits a byte.
        name.push(b'A' + (value % 26) as u8);
        value /= 26;
        if value == 0 {
            break;
        }
        value -= 1;
    }

    name.reverse();
    String::from_utf8(name).expect("column names are ASCII")
}
