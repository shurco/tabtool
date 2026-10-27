import AppKit

/// A table cell with a single, borderless text field.
@MainActor
final class CSVGridCellView: NSTableCellView {
    var row = 0
    var column = 0
    var isColumnSelected = false {
        didSet {
            if isColumnSelected != oldValue {
                needsDisplay = true
            }
        }
    }

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        let textField = CSVGridTextField()
        textField.translatesAutoresizingMaskIntoConstraints = false
        textField.isBordered = false
        textField.drawsBackground = false
        textField.usesSingleLineMode = true
        textField.lineBreakMode = .byTruncatingTail
        textField.focusRingType = .none
        addSubview(textField)
        self.textField = textField
        NSLayoutConstraint.activate([
            textField.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 4),
            textField.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -4),
            textField.centerYAnchor.constraint(equalTo: centerYAnchor),
        ])
    }

    required init?(coder: NSCoder) {
        nil
    }

    override func draw(_ dirtyRect: NSRect) {
        super.draw(dirtyRect)
        guard isColumnSelected else { return }
        NSColor.selectedContentBackgroundColor.withAlphaComponent(0.16).setFill()
        dirtyRect.fill()
    }
}

/// The cell's text field.
///
/// A text field answers a right click with its own text menu; the grid's row
/// menu comes from the table view, so the click is routed there instead.
private final class CSVGridTextField: NSTextField {
    override func menu(for event: NSEvent) -> NSMenu? {
        var ancestor = superview
        while let view = ancestor {
            if let table = view as? NSTableView { return table.menu(for: event) }
            ancestor = view.superview
        }
        return super.menu(for: event)
    }
}
