import UIKit

/// Presents a controller on the key window's topmost controller.
enum KeyWindow {
    /// Returns false when no window is available, so a continuation can resume.
    @MainActor
    @discardableResult
    static func present(_ controller: UIViewController, animated: Bool = true) -> Bool {
        let scenes = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }
        guard let root = scenes.flatMap(\.windows).first(where: \.isKeyWindow)?.rootViewController
            ?? scenes.first?.windows.first?.rootViewController else {
            return false
        }
        var presenter = root
        while let presented = presenter.presentedViewController {
            presenter = presented
        }
        presenter.present(controller, animated: animated)
        return true
    }
}
