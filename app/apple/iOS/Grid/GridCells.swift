import UIKit

/// How a grid cell is highlighted. Focus wins over a selected row or column.
enum GridCellHighlight: Equatable {
    case none
    case focused
    case selected
}

/// A data cell. Only a hairline row separator shows, so the table reads as a
/// sheet of rows instead of a boxed grid.
final class GridDataCell: UICollectionViewCell {
    static let reuseID = "data"

    private let label = UILabel()
    private let bottomBorder = UIView()

    override init(frame: CGRect) {
        super.init(frame: frame)
        label.translatesAutoresizingMaskIntoConstraints = false
        label.lineBreakMode = .byTruncatingTail
        contentView.addSubview(label)
        bottomBorder.translatesAutoresizingMaskIntoConstraints = false
        bottomBorder.backgroundColor = UIColor.separator.withAlphaComponent(0.35)
        contentView.addSubview(bottomBorder)
        NSLayoutConstraint.activate([
            label.leadingAnchor.constraint(equalTo: contentView.leadingAnchor, constant: 10),
            label.trailingAnchor.constraint(equalTo: contentView.trailingAnchor, constant: -10),
            label.topAnchor.constraint(equalTo: contentView.topAnchor),
            label.bottomAnchor.constraint(equalTo: contentView.bottomAnchor),
            bottomBorder.leadingAnchor.constraint(equalTo: contentView.leadingAnchor, constant: 10),
            bottomBorder.trailingAnchor.constraint(equalTo: contentView.trailingAnchor),
            bottomBorder.bottomAnchor.constraint(equalTo: contentView.bottomAnchor),
            bottomBorder.heightAnchor.constraint(equalToConstant: 1 / UIScreen.main.scale)
        ])
    }

    required init?(coder: NSCoder) { nil }

    func configure(text: String, alignment: NSTextAlignment, row: Int,
                   alternating: Bool, rowHeight: CGFloat, highlight: GridCellHighlight) {
        label.text = text
        label.textAlignment = alignment
        label.adjustsFontForContentSizeCategory = true
        label.font = UIFontMetrics(forTextStyle: .body)
            .scaledFont(for: .systemFont(ofSize: 15), maximumPointSize: rowHeight - 8)
        label.textColor = .label
        bottomBorder.isHidden = alternating
        contentView.layer.cornerRadius = 0
        contentView.layer.borderWidth = 0
        switch highlight {
        case .focused:
            contentView.backgroundColor = .systemBackground
            contentView.layer.cornerRadius = 4
            contentView.layer.borderWidth = 2
            contentView.layer.borderColor = UIColor.tintColor.cgColor
        case .selected:
            contentView.backgroundColor = UIColor.tintColor.withAlphaComponent(0.12)
        case .none:
            if alternating, row % 2 == 1 {
                contentView.backgroundColor = .secondarySystemBackground
            } else {
                contentView.backgroundColor = .systemBackground
            }
        }
    }
}

/// A column header or a row-number cell.
final class GridChromeCell: UICollectionReusableView {
    static let reuseID = "chrome"

    private let label = UILabel()

    override init(frame: CGRect) {
        super.init(frame: frame)
        label.translatesAutoresizingMaskIntoConstraints = false
        label.lineBreakMode = .byTruncatingTail
        addSubview(label)
        backgroundColor = .secondarySystemBackground
        NSLayoutConstraint.activate([
            label.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 10),
            label.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -6),
            label.topAnchor.constraint(equalTo: topAnchor),
            label.bottomAnchor.constraint(equalTo: bottomAnchor)
        ])
    }

    required init?(coder: NSCoder) { nil }

    func configure(text: String, centered: Bool, highlighted: Bool = false) {
        label.text = text
        label.textAlignment = centered ? .center : .left
        if centered {
            label.font = .monospacedDigitSystemFont(ofSize: 11, weight: .regular)
            label.textColor = .tertiaryLabel
        } else {
            label.font = .systemFont(ofSize: 13, weight: .semibold)
            label.textColor = .label
        }
        backgroundColor = highlighted
            ? UIColor.tintColor.withAlphaComponent(0.12)
            : .secondarySystemBackground
    }
}
