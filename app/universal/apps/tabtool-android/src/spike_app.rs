//! Developer-facing spike UI for the Android feasibility checks.

use std::time::Duration;

use eframe::egui;

use crate::insets::Insets;
use crate::platform_bridge;

const CLIPBOARD_SET_TEXT: &str = "TabTool clipboard";

#[derive(Default)]
pub struct SpikeApp {
    text: String,
    clipboard: Option<String>,
    document: Option<String>,
    insets: Insets,
    exception_contained: Option<bool>,
}

impl eframe::App for SpikeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.insets = platform_bridge::insets();

        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("TabTool Android spike");

                ui.label("Single-line field:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.text)
                        .desired_width(f32::INFINITY)
                        .hint_text("Type here"),
                );
                ui.monospace(format!("value: {:?}", self.text));

                ui.separator();

                ui.horizontal_wrapped(|ui| {
                    if ui.button("Clipboard get").clicked() {
                        self.clipboard = platform_bridge::clipboard_get();
                    }
                    if ui.button("Clipboard set").clicked() {
                        platform_bridge::clipboard_set(CLIPBOARD_SET_TEXT);
                    }
                    if ui.button("Open document").clicked() {
                        self.document = platform_bridge::open_document()
                            .or_else(|| Some("picker opened; tap again after picking".to_owned()));
                    }
                    if ui.button("Test JNI exception").clicked() {
                        self.exception_contained =
                            Some(platform_bridge::throw_test_security_exception());
                    }
                });

                ui.label(format!("clipboard: {:?}", self.clipboard));
                ui.label(format!("document: {:?}", self.document));
                if let Some(contained) = self.exception_contained {
                    ui.label(format!("SecurityException contained: {contained}"));
                }

                ui.separator();

                ui.monospace(format!("insets (dp): {:?}", self.insets));
                ui.monospace(self.insets.to_json());
            });
        });

        ui.ctx().request_repaint_after(Duration::from_millis(250));
    }
}
