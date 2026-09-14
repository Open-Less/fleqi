import Foundation

/// 底栏的定位规则：底栏永远贴在 Finder 窗口下边缘下方，与窗口左右对齐。
/// 面板只改变高度，底栏本体的位置由 `barOffset`（底栏顶边到面板顶边的距离）决定，
/// 因此气泡向上或向下展开都不会让底栏移动。这里从不修改 Finder 窗口的框架。
struct AttachmentGeometry {
    static let gap: CGFloat = 8
    struct Placement {
        var panel: CGRect
        /// 底栏下方到屏幕可见区域底部的空间，气泡向下展开时可用。
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
        // 全屏窗口没有窗口外的空间：把底栏放在窗口内侧底部，气泡只能向上展开。
        let barTop = fullscreen ? host.minY + gap + barHeight : host.minY - gap
        let spaceBelow = max(0, barTop - barHeight - visible.minY)
        let spaceAbove = max(0, visible.maxY - barTop)
        // barOffset 是面板顶边到底栏顶边的距离：面板只向上（气泡在上）或向下
        // （气泡在下）长高，底栏本身不动。AppKit 中面板顶边是 maxY。
        var topEdge = barTop + barOffset
        let lowest = visible.minY + height
        topEdge = min(max(topEdge, lowest), visible.maxY)
        return Placement(
            panel: CGRect(x: host.minX, y: topEdge - height, width: host.width, height: height),
            spaceBelow: spaceBelow,
            spaceAbove: spaceAbove
        )
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
