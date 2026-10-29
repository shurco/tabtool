import UIKit

/// The bar above the keyboard while a cell is edited: previous, next, Done.
@MainActor
enum GridEditorAccessory {
    static func make(onPrevious: @escaping () -> Void,
                     onNext: @escaping () -> Void,
                     onDone: @escaping () -> Void) -> UIToolbar {
        let toolbar = UIToolbar()
        toolbar.items = [
            .flexibleSpace(),
            imageButton("chevron.left", identifier: "editor-previous",
                        label: String(localized: "Previous Cell"), action: onPrevious),
            imageButton("chevron.right", identifier: "editor-next",
                        label: String(localized: "Next Cell"), action: onNext),
            doneButton(String(localized: "Done"), identifier: "editor-done", action: onDone)
        ]
        toolbar.sizeToFit()
        return toolbar
    }

    private static func imageButton(_ systemImage: String,
                                    identifier: String,
                                    label: String,
                                    action: @escaping () -> Void) -> UIBarButtonItem {
        let item = UIBarButtonItem(title: nil,
                                   image: UIImage(systemName: systemImage),
                                   primaryAction: UIAction { _ in action() },
                                   menu: nil)
        item.accessibilityIdentifier = identifier
        item.accessibilityLabel = label
        return item
    }

    private static func doneButton(_ title: String,
                                   identifier: String,
                                   action: @escaping () -> Void) -> UIBarButtonItem {
        let item = UIBarButtonItem(title: title,
                                   image: nil,
                                   primaryAction: UIAction { _ in action() },
                                   menu: nil)
        item.style = .done
        item.accessibilityIdentifier = identifier
        return item
    }
}
