//! Developer-facing spike UI for the Android feasibility checks.

use std::time::Duration;

use eframe::egui;

use crate::insets::Insets;
use crate::platform_bridge;
use crate::text_input::{self, TextInputOutcome, TextInputRequest};

const CLIPBOARD_SET_TEXT: &str = "TabTool clipboard";

#[derive(Default)]
pub struct SpikeApp {
    text: String,
    clipboard: Option<String>,
    document: Option<String>,
    insets: Insets,
    exception_contained: Option<bool>,
    editor_active: bool,
    last_outcome: String,
}

impl eframe::App for SpikeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if let Some(outcome) = text_input::poll() {
            self.editor_active = false;
            match outcome {
                TextInputOutcome::Committed(text) => {
                    self.text = text;
                    self.last_outcome = "committed".to_owned();
                }
                TextInputOutcome::Cancelled => {
                    self.last_outcome = "cancelled".to_owned();
                }
            }
        }

        self.insets = platform_bridge::insets();

        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("TabTool Android spike");

                ui.label("Current text:");
                ui.monospace(format!("{:?}", self.text));
                ui.monospace(format!(
                    "code points: {}",
                    self.text
                        .chars()
                        .map(|ch| format!("U+{:04X}", ch as u32))
                        .collect::<Vec<_>>()
                        .join(" ")
                ));

                let (editor_rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 56.0),
                    egui::Sense::hover(),
                );
                ui.painter()
                    .rect_filled(editor_rect, 6.0, ui.visuals().extreme_bg_color);
                ui.painter().text(
                    editor_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "native editor area",
                    egui::FontId::monospace(14.0),
                    ui.visuals().weak_text_color(),
                );

                ui.horizontal_wrapped(|ui| {
                    if ui.button("Edit text").clicked() {
                        let pixels_per_point = ui.ctx().pixels_per_point();
                        text_input::open(TextInputRequest {
                            rect: [
                                editor_rect.min.x * pixels_per_point,
                                editor_rect.min.y * pixels_per_point,
                                editor_rect.width() * pixels_per_point,
                                editor_rect.height() * pixels_per_point,
                            ],
                            text: self.text.clone(),
                            multiline: false,
                        });
                        self.editor_active = true;
                        self.last_outcome = "editing".to_owned();
                    }
                    if ui.button("Close editor").clicked() {
                        text_input::close();
                        self.editor_active = false;
                        self.last_outcome = "closed".to_owned();
                    }
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

                ui.label(format!(
                    "editor: {}",
                    if self.editor_active {
                        "active"
                    } else {
                        "closed"
                    }
                ));
                ui.label(format!("last outcome: {}", self.last_outcome));
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
