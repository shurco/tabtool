//! The sheet style, response and form-level layout helpers.

use egui::{Align, Label, Layout, RichText, Ui, Vec2};
use tabtool_i18n::Language;

use crate::fonts::heading_font;
use crate::mobile::MobileMetrics;
use crate::theme::current;

/// The width of a form row's caption column.
const FORM_LABEL_WIDTH: f32 = 118.0;

/// The geometry, language and keyboard inset every sheet draws with.
#[derive(Clone, Copy, Debug)]
pub struct SheetStyle {
    /// The touch metrics, including the safe area.
    pub metrics: MobileMetrics,
    /// The interface language.
    pub language: Language,
    /// The platform keyboard height, or zero when it is hidden.
    pub keyboard_height: f32,
}

impl SheetStyle {
    /// A style at the phone metrics with no keyboard.
    pub fn new(metrics: MobileMetrics, language: Language) -> Self {
        Self {
            metrics,
            language,
            keyboard_height: 0.0,
        }
    }

    /// Lift the sheet above the keyboard.
    pub fn keyboard_height(mut self, keyboard_height: f32) -> Self {
        self.keyboard_height = keyboard_height;
        self
    }
}

/// What a sheet reports to the screen below it.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SheetScreenResponse {
    /// The close button, Escape or a drag asked to dismiss the sheet.
    pub close_requested: bool,
}

/// A caption above a touch control.
pub(crate) fn form_row<R>(
    ui: &mut Ui,
    style: SheetStyle,
    label: &str,
    add: impl FnOnce(&mut Ui) -> R,
) -> R {
    let theme = current(ui.ctx());
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            Vec2::new(FORM_LABEL_WIDTH, style.metrics.control_height),
            Layout::left_to_right(Align::Center),
            |ui| {
                ui.add(
                    Label::new(
                        RichText::new(label)
                            .size(style.metrics.body)
                            .color(theme.colors.label),
                    )
                    .wrap(),
                );
            },
        );
        add(ui)
    })
    .inner
}

/// A section heading inside a scrollable sheet.
pub(crate) fn section_title(ui: &mut Ui, style: SheetStyle, label: &str) {
    let theme = current(ui.ctx());
    ui.add_space(10.0);
    ui.label(
        RichText::new(label)
            .font(heading_font(style.metrics.body))
            .color(theme.colors.secondary_label),
    );
    ui.add_space(4.0);
}

/// A secondary caption under a group of controls.
pub(crate) fn caption(ui: &mut Ui, style: SheetStyle, text: &str) {
    let theme = current(ui.ctx());
    ui.add(
        Label::new(
            RichText::new(text)
                .size(style.metrics.caption)
                .color(theme.colors.secondary_label),
        )
        .wrap(),
    );
}
