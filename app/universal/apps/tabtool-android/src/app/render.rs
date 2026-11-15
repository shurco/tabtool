//! The single-window composition: library, editor, sheets and banners.

use super::*;

impl AndroidApp {
    /// Draws one frame of the app.
    pub fn show(&mut self, ui: &mut Ui) {
        self.poll(Instant::now());
        self.apply_system_theme(ui.ctx());
        let language = self.refresh_language();
        let metrics = self.metrics(ui.available_width());
        self.ensure_library(metrics, language);

        let background = current(ui.ctx()).colors.window_background;
        ui.painter().rect_filled(ui.max_rect(), 0.0, background);
        let content_rect = metrics.content_rect(ui.max_rect());
        let mut content =
            ui.new_child(UiBuilder::new().max_rect(content_rect).layout(*ui.layout()));
        content.set_clip_rect(content_rect.intersect(ui.clip_rect()));

        self.draw_notice(&mut content, language);
        match self.nav.screen() {
            Screen::Library => self.show_library(&mut content, metrics, language),
            Screen::Editor(_) => self.show_editor(&mut content, metrics, language),
        }
        // A background request that arrived while a native edit was open saves
        // after the editor has applied the commit.
        self.flush_background();
        self.show_sheets(&mut content, metrics, language);
        self.show_prompt(&mut content, language);
    }

    /// Follows the platform's night mode when it reports one.
    pub fn apply_system_theme(&self, ctx: &eframe::egui::Context) {
        let Some(dark) = self.host.dark_mode() else {
            return;
        };
        let preference = if dark {
            eframe::egui::ThemePreference::Dark
        } else {
            eframe::egui::ThemePreference::Light
        };
        if ctx.options(|options| options.theme_preference) != preference {
            ctx.set_theme(preference);
        }
    }

    fn metrics(&self, width: f32) -> MobileMetrics {
        let base = if width >= 600.0 {
            MobileMetrics::tablet()
        } else {
            MobileMetrics::phone()
        };
        metrics_for(
            base,
            &self.settings,
            tabtool_ui::mobile::Insets::from_array(self.host.safe_area()),
        )
    }

    fn appearance(&self, metrics: MobileMetrics) -> GridAppearance {
        GridAppearance {
            row_height: metrics.row_height,
            alternating_rows: self.settings.grid_alternating_rows,
            shows_row_numbers: self.settings.grid_shows_row_numbers,
        }
    }

    fn refresh_language(&mut self) -> Language {
        self.language = self.settings.interface_language(&self.languages);
        self.language
    }

    fn ensure_library(&mut self, metrics: MobileMetrics, language: Language) {
        let key = LibraryKey {
            language,
            metrics,
            configuration: self.settings.new_document_configuration(),
        };
        if key == self.library_key {
            return;
        }
        self.library = library::new_view(
            Rc::clone(&self.host),
            &self.settings,
            self.locale.clone(),
            self.clock,
            metrics,
            language,
        );
        self.library_key = key;
    }

    fn show_library(&mut self, ui: &mut Ui, metrics: MobileMetrics, language: Language) {
        self.ensure_library(metrics, language);
        let response = self.library.show(ui, self.text_input.as_mut());
        self.apply_library_response(&response);
    }

    fn show_editor(&mut self, ui: &mut Ui, metrics: MobileMetrics, language: Language) {
        let keyboard_height = self.host.keyboard_height();
        let appearance = self.appearance(metrics);
        let response = {
            let Some(editor) = self.editor.as_mut() else {
                return;
            };
            MobileEditor {
                document: &mut editor.document,
                state: &mut editor.grid,
                editor_state: &mut editor.state,
                text_input: self.text_input.as_mut(),
                clipboard: self.clipboard.as_mut(),
                metrics,
                language,
                title: &editor.title,
                locale: &self.locale,
                decimal_separator: &self.decimal_separator,
                appearance,
                keyboard_height,
                fold: None,
            }
            .show(ui)
        };
        self.apply_editor_response(&response);
    }

    fn show_sheets(&mut self, ui: &mut Ui, metrics: MobileMetrics, language: Language) {
        let Some(sheet) = self.nav.sheet() else {
            return;
        };
        let style = SheetStyle::new(metrics, language).keyboard_height(self.host.keyboard_height());
        match sheet {
            SheetKind::Settings => {
                let response = settings_sheet(ui, style, &mut self.settings);
                if response.changed {
                    self.host.save_settings(&self.settings);
                }
                if response.about {
                    self.open_sheet(SheetKind::About);
                } else if response.close_requested {
                    self.close_sheet();
                }
            }
            SheetKind::About => {
                let response = about_sheet(ui, style, tabtool_core::VERSION, LICENSE);
                if let Some(action) = response.action {
                    let url = match action {
                        AboutAction::Website => WEBSITE_URL,
                        AboutAction::Rate => tabtool_ui::mobile::RATE_URL,
                    };
                    self.host.open_url(url);
                }
                if response.close_requested {
                    self.close_sheet();
                }
            }
            SheetKind::Format => {
                let close = match self.editor.as_mut() {
                    Some(editor) => {
                        editor
                            .format
                            .show(
                                ui,
                                style,
                                &mut editor.document,
                                &StdStorage,
                                self.prefers_chinese,
                            )
                            .close_requested
                    }
                    None => false,
                };
                if close {
                    self.close_sheet();
                }
            }
            SheetKind::Filters => {
                let close = match self.editor.as_mut() {
                    Some(editor) => {
                        editor
                            .filters
                            .show(
                                ui,
                                style,
                                &mut editor.document,
                                self.text_input.as_mut(),
                                &self.locale,
                            )
                            .close_requested
                    }
                    None => false,
                };
                if close {
                    self.close_sheet();
                }
            }
            SheetKind::Export => {
                let close = match self.editor.as_mut() {
                    Some(editor) => match editor.export.as_mut() {
                        Some(export) => {
                            export
                                .show(ui, style, &editor.document, &self.files)
                                .close_requested
                        }
                        None => true,
                    },
                    None => true,
                };
                if close {
                    self.close_sheet();
                }
            }
        }
    }

    fn show_prompt(&mut self, ui: &mut Ui, language: Language) {
        let Some(prompt) = self.prompt.clone() else {
            return;
        };
        let title = tr(language, "Do you want to save the changes made to “%@”?")
            .replace("%@", &prompt.name);
        let message = tr(
            language,
            "Your changes will be lost if you don’t save them.",
        );
        let actions = [
            AlertAction::new(tr(language, "Save"), ButtonKind::Prominent),
            AlertAction::new(tr(language, "Don’t Save"), ButtonKind::Destructive),
            AlertAction::new(tr(language, "Cancel"), ButtonKind::Bordered),
        ];
        match modal(ui, "android-save-prompt", &title, message, &actions) {
            Some(0) => self.answer_prompt(PromptChoice::Save),
            Some(1) => self.answer_prompt(PromptChoice::DontSave),
            Some(_) => self.answer_prompt(PromptChoice::Cancel),
            None => {}
        }
    }

    fn draw_notice(&mut self, ui: &mut Ui, language: Language) {
        let Some(notice) = self.notice.clone() else {
            return;
        };
        let (kind, text) = match notice {
            Notice::Error(key) => (BannerKind::Error, tr(language, key).to_owned()),
            Notice::Message(message) => (BannerKind::Info, message),
        };
        let pressed = Panel::top("android-notice")
            .frame(Frame::NONE)
            .show_separator_line(false)
            .show(ui, |ui| {
                banner(ui, kind, &text, "", Some(tr(language, "OK"))).action_clicked
            })
            .inner;
        if pressed {
            self.notice = None;
        }
    }
}
