import UIKit

/// Hardware-keyboard navigation for the grid. Arrow keys, Tab, Return and Esc.
final class GridCollectionView: UICollectionView {
    weak var controller: GridController?

    override var canBecomeFirstResponder: Bool { true }

    override var keyCommands: [UIKeyCommand]? {
        [arrow(UIKeyCommand.inputUpArrow, #selector(moveUp)),
         arrow(UIKeyCommand.inputDownArrow, #selector(moveDown)),
         arrow(UIKeyCommand.inputLeftArrow, #selector(moveLeft)),
         arrow(UIKeyCommand.inputRightArrow, #selector(moveRight)),
         command("\t", [], #selector(moveNext)),
         command("\t", .shift, #selector(movePrevious)),
         command("\r", [], #selector(startEditing)),
         command(UIKeyCommand.inputEscape, [], #selector(clearSelection))]
    }

    @objc private func moveUp() { controller?.moveFocus(rowDelta: -1, columnDelta: 0) }
    @objc private func moveDown() { controller?.moveFocus(rowDelta: 1, columnDelta: 0) }
    @objc private func moveLeft() { controller?.moveFocus(rowDelta: 0, columnDelta: -1) }
    @objc private func moveRight() { controller?.moveFocus(rowDelta: 0, columnDelta: 1) }
    @objc private func moveNext() { controller?.moveFocus(rowDelta: 0, columnDelta: 1) }
    @objc private func movePrevious() { controller?.moveFocus(rowDelta: 0, columnDelta: -1) }

    @objc private func startEditing() {
        controller?.beginEditing()
    }

    @objc private func clearSelection() {
        controller?.editor.dismiss(commit: false)
        controller?.focus = nil
        controller?.document?.selection = GridSelection()
        reloadData()
    }

    private func arrow(_ input: String, _ action: Selector) -> UIKeyCommand {
        command(input, [], action)
    }

    private func command(_ input: String, _ flags: UIKeyModifierFlags, _ action: Selector) -> UIKeyCommand {
        let command = UIKeyCommand(input: input, modifierFlags: flags, action: action)
        command.wantsPriorityOverSystemBehavior = true
        return command
    }
}
