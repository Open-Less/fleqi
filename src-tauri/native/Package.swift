// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "FleqiGlass",
    platforms: [.macOS("15.0")],
    products: [.library(name: "FleqiGlass", type: .static, targets: ["FleqiGlass"])],
    targets: [.target(name: "FleqiGlass"), .testTarget(name: "FleqiGlassTests", dependencies: ["FleqiGlass"])]
)
