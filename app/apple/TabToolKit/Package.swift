// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "TabToolKit",
    platforms: [
        .macOS(.v14),
        .iOS(.v17)
    ],
    products: [
        .library(name: "TabToolKit", targets: ["TabToolKit"])
    ],
    targets: [
        .target(name: "TabToolKit"),
        .testTarget(
            name: "TabToolKitTests",
            dependencies: ["TabToolKit"],
            resources: [.copy("Fixtures")]
        )
    ],
    swiftLanguageModes: [.v6]
)
