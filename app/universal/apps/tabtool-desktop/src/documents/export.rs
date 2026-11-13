//! The export panel and its confirmation and failure alerts.

use egui::Ui;
use tabtool_core::CsvError;
use tabtool_document::DocumentError;
use tabtool_i18n::tr;
use tabtool_ui::document::alert_message;
use tabtool_ui::export::{ExportPanelAction, export_panel};
use tabtool_ui::widgets::{AlertAction, ButtonKind, modal};

use crate::documents::WindowState;
use crate::files::{ExportOutcome, ExportSession};

/// The export UI state of one window.
#[derive(Default)]
pub(crate) struct ExportState {
    /// The options panel is open.
    pub(crate) session: Option<ExportSession>,
    /// The ambiguous-output confirmation is showing.
    pub(crate) confirmation: bool,
    /// The failure alert is showing.
    pub(crate) alert: Option<ExportAlert>,
}

/// A failure alert with a catalog title and a localized message.
pub(crate) struct ExportAlert {
    pub(crate) title: &'static str,
    pub(crate) message: String,
}

impl WindowState {
    /// Opens the export panel for the visible table.
    pub(crate) fn open_export_panel(&mut self) {
        let Some(session) = self.file_ops.export_session(&self.document) else {
            return;
        };
        self.export = ExportState {
            session: Some(session),
            confirmation: false,
            alert: None,
        };
    }

    /// Draws the export panel, its confirmation or its failure alert.
    pub(crate) fn render_export(&mut self, ui: &mut Ui) {
        if self.export.confirmation {
            self.render_export_confirmation(ui);
            return;
        }
        if self.export.alert.is_some() {
            self.render_export_alert(ui);
            return;
        }
        let Some(mut session) = self.export.session.take() else {
            return;
        };
        match export_panel(
            ui,
            ("export-panel", self.viewport_id),
            &mut session.options,
            self.language,
        ) {
            Some(ExportPanelAction::Cancel) => {}
            Some(ExportPanelAction::Export(configuration)) => {
                session.options.configuration = configuration;
                self.run_export(session, false);
            }
            None => self.export.session = Some(session),
        }
    }

    fn run_export(&mut self, session: ExportSession, allow_ambiguous: bool) {
        match self.file_ops.export(&session, allow_ambiguous) {
            ExportOutcome::Saved => {}
            ExportOutcome::Cancelled => {} // Return to the document, like Apple.
            ExportOutcome::NeedsConfirmation => {
                self.export.session = Some(session);
                self.export.confirmation = true;
            }
            ExportOutcome::BlockedUnencodable => {
                self.export.alert = Some(ExportAlert {
                    title: "Export Not Completed",
                    message: alert_message(
                        self.language,
                        DocumentError::Csv(CsvError::UnencodableText),
                    ),
                });
            }
            ExportOutcome::Failed(error) => {
                self.export.alert = Some(ExportAlert {
                    title: "Export Not Completed",
                    message: alert_message(self.language, error),
                });
            }
        }
    }

    fn render_export_confirmation(&mut self, ui: &mut Ui) {
        let actions = [
            AlertAction::new(tr(self.language, "Export Anyway"), ButtonKind::Destructive),
            AlertAction::new(tr(self.language, "Cancel"), ButtonKind::Bordered),
        ];
        let chosen = modal(
            ui,
            ("export-ambiguous", self.viewport_id),
            tr(self.language, "Export Anyway?"),
            tr(
                self.language,
                "Quotes are off and a header or cell contains the separator or a line break. The exported file may not read back as the same table.",
            ),
            &actions,
        );
        let Some(index) = chosen else {
            return;
        };
        self.export.confirmation = false;
        if index != 0 {
            self.export.session = None;
            return;
        }
        if let Some(session) = self.export.session.take() {
            self.run_export(session, true);
        }
    }

    fn render_export_alert(&mut self, ui: &mut Ui) {
        let Some(alert) = self.export.alert.as_ref() else {
            return;
        };
        let title = tr(self.language, alert.title);
        let message = alert.message.clone();
        let actions = [AlertAction::new(
            tr(self.language, "OK"),
            ButtonKind::Bordered,
        )];
        if modal(
            ui,
            ("export-alert", self.viewport_id),
            title,
            &message,
            &actions,
        )
        .is_some()
        {
            self.export.alert = None;
        }
    }
}
