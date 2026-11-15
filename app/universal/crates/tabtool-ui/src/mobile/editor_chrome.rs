//! The touch editor's bottom chrome: status line, banners and action bar.

use egui::{Frame, Margin, Panel, Rect, Ui, pos2};
use tabtool_document::{Document, LargeStatus};
use tabtool_i18n::Language;

use super::editor::{MobileEditor, MobileEditorResponse};
use super::editor_state::PendingWork;
use super::{ContextualAction, ContextualActionId};
use crate::document::{
    LargeUiAction, NeedsAttentionResponse, large_file_banner, needs_attention_banner,
    notice_banner, saving_text, stale_banner,
};
use crate::grid::{GridAction, GridAppearance, GridEditRequest, GridOptions};
use crate::theme::current;

/// The touch grid options for `document`.
pub(crate) fn touch_options(
    document: &Document,
    language: Language,
    appearance: GridAppearance,
    metrics: super::MobileMetrics,
) -> GridOptions {
    crate::document::grid_options(document, language, appearance, false)
        .for_touch(metrics.row_height)
}

impl MobileEditor<'_> {
    pub(super) fn show_status(
        &mut self,
        ui: &mut Ui,
        response: &mut MobileEditorResponse,
        saving: Option<f64>,
    ) {
        let summary = crate::document::document_summary(self.document, self.language, self.locale);
        let summary =
            summary.or_else(|| saving.map(|fraction| saving_text(self.language, fraction)));
        let Some(summary) = summary else {
            return;
        };
        let height = if saving.is_some() {
            self.metrics.control_height
        } else {
            self.metrics.status_line_height
        };
        let cancel = Panel::bottom("mobile-editor-status")
            .exact_size(height)
            .frame(Frame::NONE)
            .show_separator_line(false)
            .show(ui, |ui| {
                status_line(ui, self.metrics, self.language, &summary, saving)
            })
            .inner;
        if cancel {
            response.large = Some(LargeUiAction::CancelSave);
        }
    }

    pub(super) fn show_banners(
        &mut self,
        ui: &mut Ui,
        response: &mut MobileEditorResponse,
        options: &GridOptions,
    ) {
        if let Some(notice) = self.editor_state.notice {
            let dismissed = Panel::bottom("mobile-editor-notice")
                .frame(Frame::NONE)
                .show_separator_line(false)
                .show(ui, |ui| {
                    banner_frame()
                        .show(ui, |ui| notice_banner(ui, self.language, notice))
                        .inner
                })
                .inner;
            if dismissed {
                self.editor_state.notice = None;
            }
        }

        if let Some(issue) = self.document.write_issue() {
            let can_undo = self.document.can_undo();
            let banner = Panel::bottom("mobile-editor-attention")
                .frame(Frame::NONE)
                .show_separator_line(false)
                .show(ui, |ui| {
                    banner_frame()
                        .show(ui, |ui| {
                            needs_attention_banner(ui, self.language, issue, can_undo)
                        })
                        .inner
                })
                .inner;
            self.apply_needs_attention(banner, options, response);
        }

        if let LargeStatus::Stale { appended } = self.document.large_status() {
            let reload = Panel::bottom("mobile-editor-stale")
                .frame(Frame::NONE)
                .show_separator_line(false)
                .show(ui, |ui| {
                    banner_frame()
                        .show(ui, |ui| stale_banner(ui, self.language, appended))
                        .inner
                })
                .inner;
            if reload {
                response.large = Some(LargeUiAction::ReloadStale);
            }
        }

        if self.document.is_large_mode() && self.document.has_content() {
            let status = self.document.large_status();
            let rows = self
                .document
                .row_source()
                .map_or(0, |source| source.row_count());
            Panel::bottom("mobile-editor-large")
                .frame(Frame::NONE)
                .show_separator_line(false)
                .show(ui, |ui| {
                    large_file_banner(ui, self.language, status, rows, self.locale);
                });
        }
    }

    fn apply_needs_attention(
        &mut self,
        banner: NeedsAttentionResponse,
        options: &GridOptions,
        response: &mut MobileEditorResponse,
    ) {
        if banner.show_format_bar {
            response.show_format_bar = true;
        }
        if banner.undo {
            let work = PendingWork {
                undo: true,
                ..PendingWork::default()
            };
            self.with_session(options, |session| session.submit(work));
        }
    }

    pub(super) fn show_actions(
        &mut self,
        ui: &mut Ui,
        actions: &[ContextualAction],
        options: &GridOptions,
    ) {
        let chosen = Panel::bottom("mobile-editor-actions")
            .exact_size(self.metrics.action_bar_height)
            .frame(Frame::NONE)
            .show_separator_line(false)
            .show(ui, |ui| {
                super::contextual_action_bar(ui, self.metrics, self.language, actions, self.fold)
            })
            .inner;
        let Some(id) = chosen else {
            return;
        };
        let work = self.action_work(id);
        self.with_session(options, |session| session.submit(work));
    }

    fn action_work(&self, id: ContextualActionId) -> PendingWork {
        let column = self.state.selection.columns.first().copied().unwrap_or(0);
        let mut work = PendingWork::default();
        match id {
            ContextualActionId::AddRowAbove => work.actions.push(GridAction::AddRowAbove),
            ContextualActionId::AddRowBelow => work.actions.push(GridAction::AddRowBelow),
            ContextualActionId::AddColumnLeft => work.actions.push(GridAction::AddColumnLeft),
            ContextualActionId::AddColumnRight => work.actions.push(GridAction::AddColumnRight),
            ContextualActionId::Delete => work.actions.push(GridAction::DeleteSelection),
            ContextualActionId::SortAscending => work.actions.push(GridAction::Sort {
                column,
                direction: tabtool_core::SortDirection::Ascending,
            }),
            ContextualActionId::SortDescending => work.actions.push(GridAction::Sort {
                column,
                direction: tabtool_core::SortDirection::Descending,
            }),
            ContextualActionId::RemoveSort => work.actions.push(GridAction::RemoveSort),
            ContextualActionId::RenameColumn => {
                work.open = Some(GridEditRequest::Column(column));
            }
        }
        work
    }
}

/// The inset frame banners use inside their bottom panels.
fn banner_frame() -> Frame {
    Frame::new().inner_margin(Margin::symmetric(12, 6))
}

/// Draws the touch status line; returns whether Cancel was pressed.
pub(crate) fn status_line(
    ui: &mut Ui,
    metrics: super::MobileMetrics,
    language: Language,
    summary: &str,
    saving: Option<f64>,
) -> bool {
    let theme = current(ui.ctx());
    let height = if saving.is_some() {
        metrics.control_height
    } else {
        metrics.status_line_height
    };
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        egui::Sense::hover(),
    );
    ui.painter()
        .rect_filled(rect, 0.0, theme.colors.header_background);
    ui.painter().hline(
        rect.x_range(),
        rect.top(),
        egui::Stroke::new(1.0, theme.colors.separator),
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, summary));
    ui.painter().text(
        pos2(rect.left() + 10.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        summary,
        egui::FontId::proportional(theme.typography.caption),
        theme.colors.secondary_label,
    );

    let Some(fraction) = saving else {
        return false;
    };
    let cancel_width = 64.0;
    let cancel = Rect::from_min_size(
        pos2(rect.right() - cancel_width, rect.top()),
        egui::vec2(cancel_width, rect.height()),
    );
    let bar = Rect::from_min_max(
        pos2(rect.left() + 10.0, rect.bottom() - 10.0),
        pos2(
            (cancel.left() - 10.0).max(rect.left() + 10.0),
            rect.bottom() - 6.0,
        ),
    );
    let filled = (fraction.clamp(0.0, 1.0) as f32) * bar.width();
    ui.painter().rect_filled(bar, 2, theme.colors.separator);
    if filled > 0.0 {
        ui.painter().rect_filled(
            Rect::from_min_size(bar.min, egui::vec2(filled, bar.height())),
            2,
            theme.colors.accent,
        );
    }
    let cancel_label = tabtool_i18n::tr(language, "Cancel");
    super::TouchCell::new(cancel_label, cancel_label)
        .show(ui, cancel, "mobile-status-cancel")
        .clicked()
}
