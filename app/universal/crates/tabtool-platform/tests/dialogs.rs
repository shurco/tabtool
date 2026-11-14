use std::cell::RefCell;
use std::path::PathBuf;

use tabtool_platform::{
    BackHandler, Clipboard, DesktopPlatform, DocumentFiles, FileDialogs, Insets, OpenUrl,
    PlatformErrors, Share, SystemInfo, TextInput,
};

#[derive(Default)]
struct FakeDialogs {
    opened: Vec<PathBuf>,
    saved: Option<PathBuf>,
    folder: Option<PathBuf>,
    saved_requests: RefCell<Vec<(String, String)>>,
}

impl FileDialogs for FakeDialogs {
    fn open_files(&self) -> Vec<PathBuf> {
        self.opened.clone()
    }

    fn save_file(&self, suggested_name: &str, extension: &str) -> Option<PathBuf> {
        self.saved_requests
            .borrow_mut()
            .push((suggested_name.to_string(), extension.to_string()));
        self.saved.clone()
    }

    fn choose_folder(&self) -> Option<PathBuf> {
        self.folder.clone()
    }
}

#[derive(Default)]
struct FakeClipboard(RefCell<Option<String>>);

impl Clipboard for FakeClipboard {
    fn get(&self) -> Option<String> {
        self.0.borrow().clone()
    }

    fn set(&self, text: &str) {
        *self.0.borrow_mut() = Some(text.to_string());
    }
}

#[derive(Default)]
struct FakeOpenUrl(RefCell<Vec<String>>);

impl OpenUrl for FakeOpenUrl {
    fn open(&self, url: &str) {
        self.0.borrow_mut().push(url.to_string());
    }
}

#[test]
fn the_file_dialog_trait_can_be_faked() {
    let fake = FakeDialogs {
        opened: vec![PathBuf::from("/tmp/a.csv"), PathBuf::from("/tmp/b.csv")],
        saved: Some(PathBuf::from("/tmp/saved.csv")),
        folder: Some(PathBuf::from("/tmp")),
        ..FakeDialogs::default()
    };
    let dialogs: &dyn FileDialogs = &fake;

    assert_eq!(
        dialogs.open_files(),
        vec![PathBuf::from("/tmp/a.csv"), PathBuf::from("/tmp/b.csv")]
    );
    assert_eq!(
        dialogs.save_file("export.csv", "csv"),
        Some(PathBuf::from("/tmp/saved.csv"))
    );
    assert_eq!(dialogs.choose_folder(), Some(PathBuf::from("/tmp")));
    assert_eq!(
        fake.saved_requests.borrow().as_slice(),
        &[("export.csv".to_string(), "csv".to_string())]
    );
}

#[test]
fn the_clipboard_trait_can_be_faked() {
    let clipboard = FakeClipboard::default();
    assert_eq!(clipboard.get(), None);
    clipboard.set("a\tb");
    assert_eq!(clipboard.get().as_deref(), Some("a\tb"));
}

#[test]
fn the_open_url_trait_can_be_faked() {
    let open = FakeOpenUrl::default();
    open.open("https://example.com");
    assert_eq!(
        open.0.borrow().as_slice(),
        &["https://example.com".to_string()]
    );
}

#[test]
fn the_desktop_platform_implements_every_service() {
    fn assert_dialogs<T: FileDialogs>() {}
    fn assert_clipboard<T: Clipboard>() {}
    fn assert_open_url<T: OpenUrl>() {}
    fn assert_system_info<T: SystemInfo>() {}
    fn assert_text_input<T: TextInput>() {}
    fn assert_document_files<T: DocumentFiles>() {}
    fn assert_insets<T: Insets>() {}
    fn assert_share<T: Share>() {}
    fn assert_back_handler<T: BackHandler>() {}
    fn assert_platform_errors<T: PlatformErrors>() {}

    assert_dialogs::<DesktopPlatform>();
    assert_clipboard::<DesktopPlatform>();
    assert_open_url::<DesktopPlatform>();
    assert_system_info::<DesktopPlatform>();
    assert_text_input::<DesktopPlatform>();
    assert_document_files::<DesktopPlatform>();
    assert_insets::<DesktopPlatform>();
    assert_share::<DesktopPlatform>();
    assert_back_handler::<DesktopPlatform>();
    assert_platform_errors::<DesktopPlatform>();
}
