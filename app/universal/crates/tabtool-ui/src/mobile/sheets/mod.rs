//! The touch sheets: Format, Filters, Settings, About and Export.
//!
//! Every sheet composes the mobile [`super::sheet`] container and edits the
//! document or the settings directly, through the same model as the desktop
//! views. The app owns navigation, persistence and the platform seams; these
//! views only draw and report the close request their host needs.

mod about;
mod controls;
mod export;
mod export_file;
mod filters;
mod format;
mod layout;
mod save_copy;
mod settings;

pub use about::{AboutSheetResponse, RATE_URL, about_sheet};
pub use export::ExportSheet;
pub use filters::FiltersSheet;
pub use format::FormatSheet;
pub use layout::{SheetScreenResponse, SheetStyle};
pub use save_copy::{CopyResult, SAVED_COPY_KEY, SaveCopy, saved_copy_message};
pub use settings::{SettingsSheetResponse, settings_sheet};

pub(crate) use controls::{
    PickerTouch, touch_button, touch_checkbox, touch_field, touch_icon_button, touch_picker,
    touch_picker_sized, touch_toggle, touch_wide_button,
};
pub(crate) use layout::{caption, form_row, section_title};
