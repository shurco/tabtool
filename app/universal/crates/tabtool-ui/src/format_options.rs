//! The separator, decimal and quote choices shared by the format bar and the
//! export options.

/// The separators the bars offer, with their catalog keys.
pub(crate) const SEPARATORS: [(&str, &str); 4] = [
    (",", "Comma (,)"),
    (";", "Semicolon (;)"),
    ("\t", "Tab (⇥)"),
    ("|", "Pipe (|)"),
];

/// The decimal marks the bars offer, with their catalog keys.
pub(crate) const DECIMALS: [(&str, &str); 2] = [(".", "Dot (.)"), (",", "Comma (,)")];
