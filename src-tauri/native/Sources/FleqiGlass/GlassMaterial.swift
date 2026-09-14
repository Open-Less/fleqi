import AppKit
import ObjectiveC
import WebKit

// This module owns only Fleqi's backdrop views. It never reads or changes Finder.
private var materialCoordinatorKey: UInt8 = 0

@MainActor
private func findWebView(in view: NSView) -> WKWebView? {
    if let webView = view as? WKWebView { return webView }
    for child in view.subviews {
        if let webView = findWebView(in: child) { return webView }
    }
    return nil
}

private final class PassiveView: NSView {
    override func hitTest(_ point: NSPoint) -> NSView? { nil }
}

private final class WallpaperView: NSView {
    override func hitTest(_ point: NSPoint) -> NSView? { nil }
    override func draw(_ dirtyRect: NSRect) {
        let gradient = NSGradient(colors: [
            NSColor(srgbRed: 0.68, green: 0.80, blue: 0.97, alpha: 1),
            NSColor(srgbRed: 0.83, green: 0.90, blue: 1.0, alpha: 1),
            NSColor(srgbRed: 0.71, green: 0.83, blue: 0.98, alpha: 1),
        ])
        gradient?.draw(in: bounds, angle: 35)
    }
}

@MainActor
private final class MaterialCoordinator: NSObject {
    weak var container: NSView?
    let wallpaper = WallpaperView()
    let holder = PassiveView()
    var effect: NSView?
    var themeIsDark = false
    var wantsLiquid = false
    var radius: CGFloat = 20
    var renderKey = ""

    init(container: NSView) {
        self.container = container
        super.init()
        wallpaper.frame = container.bounds
        wallpaper.autoresizingMask = [.width, .height]
        holder.wantsLayer = true
        container.addSubview(wallpaper, positioned: .below, relativeTo: nil)
        container.addSubview(holder, positioned: .above, relativeTo: wallpaper)
        NSWorkspace.shared.notificationCenter.addObserver(
            self, selector: #selector(accessibilityChanged),
            name: NSWorkspace.accessibilityDisplayOptionsDidChangeNotification, object: nil
        )
    }

    deinit { NSWorkspace.shared.notificationCenter.removeObserver(self) }

    @objc private func accessibilityChanged() { _ = render() }

    func render() -> Int32 {
        let reduce = NSWorkspace.shared.accessibilityDisplayShouldReduceTransparency
        var liquidAvailable = false
        if #available(macOS 26.0, *) { liquidAvailable = true }
        let useLiquid = wantsLiquid && liquidAvailable && !reduce
        let key = "\(themeIsDark)-\(useLiquid)-\(reduce)"
        if renderKey != key {
            let previous = effect
            let view: NSView
            if reduce {
                let opaque = PassiveView()
                opaque.wantsLayer = true
                opaque.layer?.backgroundColor = (themeIsDark
                    ? NSColor(white: 0.07, alpha: 1)
                    : NSColor(white: 0.97, alpha: 1)).cgColor
                view = opaque
            } else if #available(macOS 26.0, *), useLiquid {
                let glass = NSGlassEffectView()
                glass.style = .regular
                glass.cornerRadius = radius
                glass.tintColor = themeIsDark
                    ? NSColor.black.withAlphaComponent(0.78)
                    : NSColor.white.withAlphaComponent(0.25)
                glass.appearance = NSAppearance(named: themeIsDark ? .darkAqua : .aqua)
                let content = PassiveView(frame: holder.bounds)
                content.autoresizingMask = [.width, .height]
                content.wantsLayer = true
                // Preserve readable black glass even over a very bright desktop.
                content.layer?.backgroundColor = themeIsDark
                    ? NSColor.black.withAlphaComponent(0.84).cgColor
                    : NSColor.clear.cgColor
                glass.contentView = content
                view = glass
            } else {
                let frosted = NSVisualEffectView()
                frosted.material = themeIsDark ? .hudWindow : .popover
                frosted.blendingMode = wallpaper.isHidden ? .behindWindow : .withinWindow
                frosted.state = .active
                frosted.appearance = NSAppearance(named: themeIsDark ? .darkAqua : .aqua)
                let tint = PassiveView(frame: holder.bounds)
                tint.autoresizingMask = [.width, .height]
                tint.wantsLayer = true
                tint.layer?.backgroundColor = (themeIsDark
                    ? NSColor.black.withAlphaComponent(0.82)
                    : NSColor.white.withAlphaComponent(0.20)).cgColor
                frosted.addSubview(tint)
                view = frosted
            }
            view.frame = holder.bounds
            view.autoresizingMask = [.width, .height]
            let animated = previous != nil && !FleqiMotion.reduced
            view.alphaValue = animated ? 0 : 1
            holder.addSubview(view)
            effect = view
            renderKey = key
            if animated {
                NSAnimationContext.runAnimationGroup({ context in
                    context.duration = FleqiMotion.layoutDuration
                    context.timingFunction = FleqiMotion.timing
                    view.animator().alphaValue = 1
                    previous?.animator().alphaValue = 0
                }, completionHandler: { previous?.removeFromSuperview() })
            } else { previous?.removeFromSuperview() }
        }
        holder.layer?.cornerRadius = radius
        holder.layer?.masksToBounds = true
        if #available(macOS 26.0, *), let glass = effect as? NSGlassEffectView {
            glass.cornerRadius = radius
        }
        return useLiquid ? 1 : 0
    }
}

@_cdecl("fleqi_material_support")
@MainActor
public func fleqiMaterialSupport() -> Int32 {
    var result: Int32 = 0
    let reduce = NSWorkspace.shared.accessibilityDisplayShouldReduceTransparency
    if #available(macOS 26.0, *), !reduce { result |= 1 }
    if reduce { result |= 2 }
    return result
}

@_cdecl("fleqi_apply_material")
@MainActor
public func fleqiApplyMaterial(
    _ pointer: UnsafeMutableRawPointer?, _ material: Int32, _ dark: Int32,
    _ x: Double, _ y: Double, _ width: Double, _ height: Double,
    _ radius: Double, _ scene: Int32
) -> Int32 {
    guard let pointer else { return -1 }
    let window = Unmanaged<NSWindow>.fromOpaque(pointer).takeUnretainedValue()
    guard let container = window.contentView,
          let webView = findWebView(in: container) else { return -1 }
    window.isOpaque = false
    window.backgroundColor = .clear

    let coordinator: MaterialCoordinator
    if let existing = objc_getAssociatedObject(window, &materialCoordinatorKey) as? MaterialCoordinator {
        coordinator = existing
    } else {
        coordinator = MaterialCoordinator(container: container)
        objc_setAssociatedObject(window, &materialCoordinatorKey, coordinator, .OBJC_ASSOCIATION_RETAIN_NONATOMIC)
    }
    // Extend two radii behind the Finder reference, hiding the upper material corners
    // while preserving the same radius for all four corners of the combined outline.
    let overlap = scene == 1 ? 2 * radius : 0
    // WKWebView's default CSS viewport excludes the safe-area insets, including
    // the title bar. Convert from that viewport before converting view spaces.
    let top = y + webView.safeAreaInsets.top - overlap
    let fullHeight = height + overlap
    let originY = webView.isFlipped ? top : webView.bounds.height - top - fullHeight
    let webRect = NSRect(x: x + webView.safeAreaInsets.left, y: originY, width: width, height: fullHeight)
    coordinator.holder.frame = webView.convert(webRect, to: container)
    if coordinator.wallpaper.isHidden != (scene != 1) { coordinator.renderKey = "" }
    coordinator.wallpaper.isHidden = scene != 1
    coordinator.themeIsDark = dark == 1
    coordinator.wantsLiquid = material == 1
    coordinator.radius = radius
    return coordinator.render()
}
