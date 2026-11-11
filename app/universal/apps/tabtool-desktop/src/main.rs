use eframe::egui;

fn main() -> eframe::Result {
    if smoke_requested(std::env::args().skip(1)) {
        println!("tabtool {}", tabtool_core::VERSION);
        return Ok(());
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_title("TabTool"),
        ..Default::default()
    };

    eframe::run_native("TabTool", options, Box::new(|_cc| Ok(Box::new(TabToolApp))))
}

fn smoke_requested(args: impl IntoIterator<Item = String>) -> bool {
    args.into_iter().any(|arg| arg == "--smoke")
}

struct TabToolApp;

impl eframe::App for TabToolApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.centered_and_justified(|ui| {
                ui.label("TabTool");
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::smoke_requested;

    #[test]
    fn detects_smoke_flag_among_other_arguments() {
        let args = ["ignored".to_owned(), "--smoke".to_owned()];

        assert!(smoke_requested(args.into_iter()));
    }

    #[test]
    fn ignores_arguments_without_smoke_flag() {
        let args = ["--windowed".to_owned()];

        assert!(!smoke_requested(args.into_iter()));
    }
}
