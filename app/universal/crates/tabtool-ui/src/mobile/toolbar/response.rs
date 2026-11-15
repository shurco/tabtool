//! What the navigation bar reports: pressed controls and overflow items.

/// An item in the overflow menu.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OverflowItem {
    /// Format settings.
    Format,
    /// App settings.
    Settings,
    /// About TabTool.
    About,
    /// Share the current document.
    Share,
    /// Save a copy of a large document.
    SaveAs,
}

impl OverflowItem {
    /// The catalog key for the visible label.
    pub fn label_key(self) -> &'static str {
        match self {
            Self::Format => "Format",
            Self::Settings => "Settings",
            Self::About => "About",
            Self::Share => "Share",
            Self::SaveAs => "Save As",
        }
    }
}

/// What the toolbar asked for this frame.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MobileToolbarResponse {
    /// Back was pressed.
    pub back: bool,
    /// Undo was pressed.
    pub undo: bool,
    /// Redo was pressed.
    pub redo: bool,
    /// Filter was pressed.
    pub filter: bool,
    /// Export was pressed.
    pub export: bool,
    /// The chosen overflow item, if any.
    pub overflow: Option<OverflowItem>,
}

impl MobileToolbarResponse {
    /// Whether no control was pressed.
    pub fn is_empty(&self) -> bool {
        !self.back
            && !self.undo
            && !self.redo
            && !self.filter
            && !self.export
            && self.overflow.is_none()
    }

    /// Folds another frame's response into this one.
    ///
    /// A deferred response from a commit frame and a tap in the same frame
    /// both matter, so a press in either is kept.
    pub fn merge(&mut self, other: Self) {
        self.back |= other.back;
        self.undo |= other.undo;
        self.redo |= other.redo;
        self.filter |= other.filter;
        self.export |= other.export;
        if other.overflow.is_some() {
            self.overflow = other.overflow;
        }
    }
}
