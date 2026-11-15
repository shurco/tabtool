//! Crate-layering guards: the UI must not reach desktop services.

#[test]
fn the_ui_depends_on_the_neutral_settings_model_not_the_platform() {
    let manifest = include_str!("../Cargo.toml");
    assert!(
        manifest.contains("tabtool-settings"),
        "the UI uses the platform-neutral settings model"
    );
    assert!(
        !manifest.contains("tabtool-platform"),
        "tabtool-ui must not depend on rfd/open/directories through tabtool-platform"
    );
}

#[test]
fn the_neutral_settings_model_has_no_os_services() {
    let manifest = include_str!("../../tabtool-settings/Cargo.toml");
    for forbidden in [
        "rfd",
        "open",
        "directories",
        "sys-locale",
        "tabtool-platform",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "tabtool-settings must stay free of `{forbidden}`"
        );
    }
}
