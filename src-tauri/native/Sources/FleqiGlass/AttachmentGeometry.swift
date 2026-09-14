import Foundation

/// 底栏的定位规则：底栏是绝对定位的——底边永远钉在屏幕底部（Dock 之上），
/// 不随访达窗口的高度、位置或是否贴底而改变；横向范围与访达窗口对齐。
/// 面板只改变高度，底栏本体的位置由 `barOffset`（底栏顶边到面板顶边的距离）决定，
/// 气泡一律向上展开。这里从不修改 Finder 窗口的框架。
struct AttachmentGeometry {
    struct Placement {
        var panel: CGRect
        /// 底栏下方没有可用空间：气泡永远向上展开，保留字段供界面判断。
        var spaceBelow: CGFloat
        /// 底栏上方到屏幕可见区域顶部的空间，气泡向上展开时可用。
        var spaceAbove: CGFloat
    }
    static func layout(
        host: CGRect,
        visible: CGRect,
        barHeight: CGFloat,
        contentHeight: CGFloat,
        barOffset: CGFloat,
        fullscreen: Bool
    ) -> Placement? {
        guard host.width > 200, host.height > 100 else { return nil }
        let height = max(barHeight, contentHeight)
        let width = min(host.width, visible.width)
        let x = min(max(host.minX, visible.minX), visible.maxX - width)
        // AppKit 中 y 轴向上：面板底边贴屏幕可见区域底部，顶边随内容增高。
        let panel = CGRect(x: x, y: visible.minY, width: width, height: height)
        let barTop = panel.maxY - barOffset
        let spaceAbove = max(0, visible.maxY - barTop)
        return Placement(panel: panel, spaceBelow: 0, spaceAbove: spaceAbove)
    }
}

struct WindowMotionGate {
    private var previous: CGRect?
    private var wasPressed = false
    private var dragging = false
    private var settleUntil: TimeInterval = 0
    mutating func reset() { self = Self() }
    mutating func noteApplicationFrame(_ frame: CGRect) { previous = frame }
    mutating func hide(frame: CGRect, pressed: Bool, now: TimeInterval, applicationChange: Bool = false) -> Bool {
        let changed = previous.map { abs($0.minX - frame.minX) > 0.5 || abs($0.minY - frame.minY) > 0.5 || abs($0.width - frame.width) > 0.5 || abs($0.height - frame.height) > 0.5 } ?? false
        previous = frame
        if changed && !applicationChange {
            dragging = dragging || pressed
            settleUntil = now + 0.14
        }
        if dragging && wasPressed && !pressed { settleUntil = now + 0.14 }
        wasPressed = pressed
        if dragging && pressed { return true }
        if now < settleUntil { return true }
        dragging = false
        return false
    }
}
