import UIKit

/// A data cell. Vertical borders always show. Horizontal borders show when
/// alternating row colors are off, matching the Mac grid.
final class GridDataCell: UICollectionViewCell {
    static let reuseID = "data"

    private let label = UILabel()
    private let rightBorder = UIView()
    private let bottomBorder = UIView()

    override init(frame: CGRect) {
        super.init(frame: frame)
        label.translatesAutoresizingMaskIntoConstraints = false
        label.lineBreakMode = .byTruncatingTail
        contentView.addSubview(label)
        rightBorder.translatesAutoresizingMaskIntoConstraints = false
        bottomBorder.translatesAutoresizingMaskIntoConstraints = false
        contentView.addSubview(rightBorder)
        contentView.addSubview(bottomBorder)
        let border = UIColor.separator
        rightBorder.backgroundColor = border
        bottomBorder.backgroundColor = border
        NSLayoutConstraint.activate([
            label.leadingAnchor.constraint(equalTo: contentView.leadingAnchor, constant: 6),
            label.trailingAnchor.constraint(equalTo: contentView.trailingAnchor, constant: -6),
            label.topAnchor.constraint(equalTo: contentView.topAnchor),
            label.bottomAnchor.constraint(equalTo: contentView.bottomAnchor),
            rightBorder.trailingAnchor.constraint(equalTo: contentView.trailingAnchor),
            rightBorder.topAnchor.constraint(equalTo: contentView.topAnchor),
            rightBorder.bottomAnchor.constraint(equalTo: contentView.bottomAnchor),
            rightBorder.widthAnchor.constraint(equalToConstant: 1),
            bottomBorder.leadingAnchor.constraint(equalTo: contentView.leadingAnchor),
            bottomBorder.trailingAnchor.constraint(equalTo: contentView.trailingAnchor),
            bottomBorder.bottomAnchor.constraint(equalTo: contentView.bottomAnchor),
            bottomBorder.heightAnchor.constraint(equalToConstant: 1)
        ])
    }

    required init?(coder: NSCoder) { nil }

    func configure(text: String, alignment: NSTextAlignment, row: Int,
                   alternating: Bool, rowHeight: CGFloat, highlighted: Bool) {
        label.text = text
        label.textAlignment = alignment
        label.font = .systemFont(ofSize: min(15, rowHeight - 8))
        label.textColor = .label
        bottomBorder.isHidden = alternating
        if highlighted {
            contentView.backgroundColor = UIColor.systemBlue.withAlphaComponent(0.18)
        } else if alternating, row % 2 == 1 {
            contentView.backgroundColor = .secondarySystemFill
        } else {
            contentView.backgroundColor = .systemBackground
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
            label.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 6),
            label.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -4),
            label.topAnchor.constraint(equalTo: topAnchor),
            label.bottomAnchor.constraint(equalTo: bottomAnchor)
        ])
    }

    required init?(coder: NSCoder) { nil }

    func configure(text: String, centered: Bool, highlighted: Bool = false) {
        label.text = text
        label.textAlignment = centered ? .center : .left
        label.font = centered
            ? .monospacedDigitSystemFont(ofSize: 12, weight: .regular)
            : .systemFont(ofSize: 12, weight: .semibold)
        label.textColor = .secondaryLabel
        backgroundColor = highlighted
            ? UIColor.systemBlue.withAlphaComponent(0.18)
            : .secondarySystemBackground
    }
}
