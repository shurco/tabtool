@testable import TabTool
import Testing
import UIKit

@MainActor
@Suite struct GridEditorAccessoryTests {
    @Test func buttonsUseTheDocumentedIdentifiers() {
        let toolbar = GridEditorAccessory.make(onPrevious: {}, onNext: {}, onDone: {})
        let identifiers = toolbar.items?.compactMap(\.accessibilityIdentifier) ?? []
        #expect(identifiers == ["editor-previous", "editor-next", "editor-done"])
    }

    @Test func showInstallsTheAccessoryAndDisablesSmartInput() {
        let editor = GridEditor()
        let view = UICollectionView(frame: CGRect(x: 0, y: 0, width: 200, height: 200),
                                    collectionViewLayout: UICollectionViewFlowLayout())
        editor.show(in: view, frame: CGRect(x: 0, y: 0, width: 100, height: 44), text: "cell")
        #expect(editor.field.inputAccessoryView is UIToolbar)
        #expect(editor.field.autocapitalizationType == .none)
        #expect(editor.field.smartQuotesType == .no)
        editor.dismiss(commit: false)
    }
}
