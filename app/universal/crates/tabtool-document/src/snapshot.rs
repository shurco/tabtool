//! The value captured when a document is saved.
//!
//! The snapshot is cheap to clone: the table is shared behind an [`Arc`], so
//! autosave can serialize a consistent value off-thread without copying the
//! cells or reading document state.

use std::sync::Arc;

use tabtool_core::{CsvConfiguration, CsvError, CsvTable, CsvWriter};

/// The table, format and byte-order-mark state a save writes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentSnapshot {
    /// The table at the moment the snapshot was taken.
    pub table: Arc<CsvTable>,
    /// The format used to write the table.
    pub configuration: CsvConfiguration,
    /// Whether the saved bytes start with a UTF-8 byte order mark.
    pub includes_utf8_bom: bool,
}

/// Serializes a snapshot with its captured table, format and BOM.
///
/// The writer applies the same ambiguity and encoding checks as the save gate,
/// so a snapshot that passes validation cannot fail for a different reason.
pub(crate) fn encode(snapshot: &DocumentSnapshot) -> Result<Vec<u8>, CsvError> {
    let header = if snapshot.configuration.first_row_as_header {
        snapshot.table.headers()
    } else {
        None
    };
    let mut writer = CsvWriter::new(&snapshot.table, header.as_deref(), &snapshot.configuration);
    writer.include_utf8_bom = snapshot.includes_utf8_bom;
    writer.data()
}
