//! Developer-facing smoke UI for the Android platform services.

use std::path::PathBuf;
use std::time::Duration;

use eframe::egui;
use tabtool_platform::android::{AndroidPlatform, throw_test_security_exception};
use tabtool_platform::{
    BackHandler, Clipboard, DocumentFiles, Insets, OpenUrl, PlatformErrors, Share, SystemInfo,
    TextInput, TextInputOutcome, TextInputRequest,
};

const CLIPBOARD_TEXT: &str = "TabTool clipboard";
const OPEN_URL: &str = "https://example.com";
const SAMPLE_NAME: &str = "sample.csv";
const EXPORT_NAME: &str = "TabTool export.csv";
const SAMPLE_CONTENT: &str = "name,value\nalpha,1\n";

/// A SAF operation that is waiting for the picker and its copy.
enum Pending {
    Import,
    Export { source: PathBuf, name: String },
}

pub struct SpikeApp {
    platform: AndroidPlatform,
    text: String,
    clipboard: Option<String>,
    document: Option<String>,
    safe_area: [f32; 4],
    keyboard_height: f32,
    languages: Vec<String>,
    status: String,
    error_banner: Option<String>,
    last_outcome: String,
    editor_active: bool,
    pending: Option<Pending>,
    exception_contained: Option<bool>,
}

impl Default for SpikeApp {
    fn default() -> Self {
        Self {
            platform: AndroidPlatform,
            text: String::new(),
            clipboard: None,
            document: None,
            safe_area: [0.0; 4],
            keyboard_height: 0.0,
            languages: Vec::new(),
            status: "ready".to_owned(),
            error_banner: None,
            last_outcome: String::new(),
            editor_active: false,
            pending: None,
            exception_contained: None,
        }
    }
}

impl SpikeApp {
    fn start_import(&mut self) {
        self.status = "import: picker open".to_owned();
        self.pending = Some(Pending::Import);
        // Starts the picker; its result is drained while `pending` is set.
        self.platform.import_from_picker();
    }

    fn start_export(&mut self) {
        let Some(source) = self.export_source() else {
            self.status = "export: no source document".to_owned();
            return;
        };
        self.status = "export: picker open".to_owned();
        self.pending = Some(Pending::Export {
            source: source.clone(),
            name: EXPORT_NAME.to_owned(),
        });
        self.platform.export_copy(&source, EXPORT_NAME);
    }

    /// The imported document, or a small sample written into the documents dir.
    fn export_source(&self) -> Option<PathBuf> {
        if let Some(document) = &self.document {
            return Some(PathBuf::from(document));
        }
        let path = self.platform.documents_dir().join(SAMPLE_NAME);
        std::fs::write(&path, SAMPLE_CONTENT).ok()?;
        Some(path)
    }

    fn drain_pending(&mut self) {
        let Some(pending) = self.pending.take() else {
            return;
        };
        if self.platform.document_operation_pending() {
            self.pending = Some(pending);
            return;
        }

        match pending {
            Pending::Import => {
                self.status = match self.platform.import_from_picker() {
                    Some(path) => {
                        let shown = path.display().to_string();
                        self.document = Some(shown.clone());
                        format!("imported: {shown}")
                    }
                    None => "import: no file (cancel or error; see logcat)".to_owned(),
                };
            }
            Pending::Export { source, name } => {
                self.status = if self.platform.export_copy(&source, &name) {
                    "export: copy written".to_owned()
                } else {
                    "export: no copy (cancel or error; see logcat)".to_owned()
                };
            }
        }
    }
}

impl eframe::App for SpikeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if let Some(outcome) = self.platform.poll() {
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
        self.editor_active = self.platform.is_active();
        if self.platform.take_back() {
            self.status = "back: taken".to_owned();
        }
        if let Some(error) = self.platform.take_error() {
            let language = tabtool_i18n::language_from_system(&self.platform.languages());
            self.error_banner = Some(tabtool_i18n::tr(language, error.message_key).to_owned());
        }
        self.safe_area = self.platform.safe_area();
        self.keyboard_height = self.platform.keyboard_height();
        self.drain_pending();

        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("TabTool Android platform services");

                ui.label("Current text:");
                ui.monospace(format!("{:?}", self.text));

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
                        TextInput::open(
                            &self.platform,
                            TextInputRequest {
                                rect: [
                                    editor_rect.min.x,
                                    editor_rect.min.y,
                                    editor_rect.width(),
                                    editor_rect.height(),
                                ],
                                text: self.text.clone(),
                                multiline: false,
                            },
                        );
                        self.last_outcome = "editing".to_owned();
                    }
                    if ui.button("Close editor").clicked() {
                        self.platform.close();
                        self.last_outcome = "closed".to_owned();
                    }
                    if ui.button("Clipboard get").clicked() {
                        self.clipboard = self.platform.get();
                    }
                    if ui.button("Clipboard set").clicked() {
                        self.platform.set(CLIPBOARD_TEXT);
                    }
                    if ui.button("Import").clicked() {
                        self.start_import();
                    }
                    if ui.button("Export").clicked() {
                        self.start_export();
                    }
                    if ui.button("Share").clicked() {
                        match self.export_source() {
                            Some(source) => {
                                self.platform.share(&source);
                                self.status = format!("share: {}", source.display());
                            }
                            None => self.status = "share: no source document".to_owned(),
                        }
                    }
                    if ui.button("Languages").clicked() {
                        self.languages = self.platform.languages();
                    }
                    if ui.button("Open URL").clicked() {
                        OpenUrl::open(&self.platform, OPEN_URL);
                        self.status = format!("open: {OPEN_URL}");
                    }
                    if ui.button("Test JNI exception").clicked() {
                        self.exception_contained = Some(throw_test_security_exception());
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
                ui.label(format!("status: {}", self.status));
                if let Some(error) = &self.error_banner {
                    ui.label(format!("error: {error}"));
                }
                ui.label(format!("clipboard: {:?}", self.clipboard));
                ui.label(format!("document: {:?}", self.document));
                ui.label(format!(
                    "documents dir: {}",
                    self.platform.documents_dir().display()
                ));
                ui.label(format!(
                    "documents: {}",
                    self.platform.list_documents().len()
                ));
                ui.label(format!("languages: {:?}", self.languages));
                if let Some(contained) = self.exception_contained {
                    ui.label(format!("SecurityException contained: {contained}"));
                }

                ui.separator();

                ui.monospace(format!("safe area (dp): {:?}", self.safe_area));
                ui.monospace(format!("keyboard (dp): {}", self.keyboard_height));
            });
        });

        ui.ctx().request_repaint_after(Duration::from_millis(250));
    }
}
