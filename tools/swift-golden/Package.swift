// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "swift-golden",
    platforms: [.macOS(.v14)],
    dependencies: [
        .package(path: "../../app/apple/TabToolKit")
    ],
    targets: [
        .executableTarget(
            name: "swift-golden",
            dependencies: [.product(name: "TabToolKit", package: "TabToolKit")]
        )
    ],
    swiftLanguageModes: [.v6]
)
