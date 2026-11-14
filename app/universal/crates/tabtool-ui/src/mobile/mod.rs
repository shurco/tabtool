//! Touch-sized mobile layout primitives.
//!
//! These views layer the iOS layout on the shared [`crate::Theme`] tokens: a
//! navigation stack, a touch toolbar, the fold-aware contextual action bar,
//! sheets, list rows, the documents library and the editor accessory. They
//! depend on the platform only through [`TouchTextInput`] and the library's
//! file seams, so `tabtool-ui` stays neutral.

mod accessory;
mod action_bar;
mod library;
mod list;
mod metrics;
mod nav;
mod sheet;
mod toolbar;

pub use accessory::{AccessoryAction, editor_accessory};
pub use action_bar::{
    ContextualAction, ContextualActionId, FoldGap, FoldRegion, actions_for, actions_for_selection,
    contextual_action_bar, fold_gap, is_enabled,
};
pub use library::{
    DocumentEntry, DocumentFiles, DocumentMetadata, ERROR_GENERIC, ERROR_OPEN, ERROR_SAVE,
    FileMetadata, LibraryClock, LibraryError, LibraryErrors, LibraryResponse, LibrarySort,
    LibraryView, SystemMetadata, available_name, format_modified, format_size, new_document_name,
};
pub use list::{ListRow, ListRowResponse};
pub use metrics::{
    Insets, MOBILE_BODY_TEXT, MOBILE_DEFAULT_ROW_HEIGHT, MOBILE_MIN_TOUCH_TARGET,
    MOBILE_ROW_HEIGHTS, MOBILE_TABLET_SHEET_WIDTH, MobileMetrics, row_height,
};
pub use nav::{BackOutcome, DocumentId, MISSING_DOCUMENT_NOTICE, NavStack, Screen, SheetKind};
pub use sheet::{SheetOptions, SheetResponse, sheet};
pub use toolbar::{MobileToolbar, MobileToolbarResponse, OverflowItem};

use core::fmt;
use core::hash::Hash;

use egui::{Align2, Color32, CornerRadius, FontId, Rect, Response, Sense, Ui, WidgetInfo};

use crate::theme::current;

/// A request to open the platform's text editor over a control.
#[derive(Clone, Debug, PartialEq)]
pub struct TouchTextInputRequest {
    /// The control's rectangle in points, in screen coordinates.
    pub rect: Rect,
    /// The text the editor starts with.
    pub text: String,
    /// Whether the editor accepts line breaks.
    pub multiline: bool,
}

/// What the platform text editor returned.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TouchTextInputOutcome {
    /// The user committed `text`.
    Committed(String),
    /// The user dismissed the editor.
    Cancelled,
}

/// The native text editor the mobile views use instead of an egui text field.
///
/// The app adapts the platform's `TextInput` to this seam; the UI never opens
/// an in-window egui text field on Android.
pub trait TouchTextInput {
    /// Opens the editor over the request's rectangle.
    fn open(&mut self, request: TouchTextInputRequest);

    /// Closes the editor without an outcome.
    fn close(&mut self);

    /// Takes the one outcome the session produced.
    fn poll(&mut self) -> Option<TouchTextInputOutcome>;

    /// Whether an editing session is open.
    fn is_active(&self) -> bool;
}

/// A square touch cell painted from a symbol and an accessible name.
pub(crate) struct TouchCell<'a> {
    glyph: &'a str,
    name: &'a str,
    enabled: bool,
    destructive: bool,
}

impl<'a> TouchCell<'a> {
    /// An enabled, plain cell.
    pub(crate) fn new(glyph: &'a str, name: &'a str) -> Self {
        Self {
            glyph,
            name,
            enabled: true,
            destructive: false,
        }
    }

    /// Enable or disable the cell.
    pub(crate) fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Paint the symbol in the destructive color.
    pub(crate) fn destructive(mut self) -> Self {
        self.destructive = true;
        self
    }

    /// Show the cell in `rect`.
    pub(crate) fn show(self, ui: &Ui, rect: Rect, id_salt: impl Hash + fmt::Debug) -> Response {
        let theme = current(ui.ctx());
        let colors = theme.colors;
        let radius = CornerRadius::same(theme.metrics.small_corner_radius);
        let sense = if self.enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let response = ui.interact(rect, ui.scope_id().with(id_salt), sense);

        if response.hovered() && self.enabled {
            ui.painter()
                .rect_filled(rect, radius, colors.hover_background);
        }

        let color = if !self.enabled {
            colors.tertiary_label
        } else if self.destructive {
            colors.error
        } else {
            colors.accent
        };
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            self.glyph,
            FontId::proportional(theme.typography.body),
            color,
        );
        response.widget_info(|| WidgetInfo::labeled(egui::Role::Button, self.enabled, self.name));
        crate::widgets::focus_ring(ui, &response);
        response
    }
}

/// A hairline inside `rect` at `y`.
pub(crate) fn hairline(ui: &Ui, rect: Rect, y: f32) {
    let theme = current(ui.ctx());
    ui.painter().hline(
        rect.x_range(),
        y,
        egui::Stroke::new(1.0, theme.colors.separator),
    );
}

/// The bar background fill.
pub(crate) fn bar_fill(ui: &Ui) -> Color32 {
    current(ui.ctx()).colors.window_background
}
