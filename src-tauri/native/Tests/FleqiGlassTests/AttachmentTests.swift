import XCTest
@testable import FleqiGlass

final class AttachmentTests: XCTestCase {
    let screen = CGRect(x: 0, y: 24, width: 1440, height: 850)

    func testBarIsPinnedToTheScreenBottomRegardlessOfWindow() throws {
        // 窗口高、窗口矮、窗口贴底：底栏面板必须完全一致地钉在屏幕底部。
        let tall = CGRect(x: 140, y: 40, width: 1000, height: 800)
        let short = CGRect(x: 140, y: 700, width: 1000, height: 200)
        let tight = CGRect(x: 140, y: 24, width: 1000, height: 120)
        for host in [tall, short, tight] {
            let placement = try XCTUnwrap(AttachmentGeometry.layout(host: host, visible: screen, barHeight: 56, contentHeight: 56, barOffset: 0, fullscreen: false))
            XCTAssertEqual(placement.panel.minY, screen.minY, "面板底边必须贴屏幕可见区域底部")
            XCTAssertEqual(placement.panel.maxY, screen.minY + 56, "底栏顶边位置必须唯一确定")
            XCTAssertEqual(placement.panel.minX, host.minX)
            XCTAssertEqual(placement.panel.width, host.width)
            XCTAssertEqual(placement.spaceBelow, 0)
        }
        // Finder 窗口本身从不被移动或缩放：以上只读几何，不产生任何窗口操作。
    }

    func testBubbleAboveKeepsTheBarInPlace() throws {
        let host = CGRect(x: 0, y: 120, width: 1000, height: 600)
        let bar = try XCTUnwrap(AttachmentGeometry.layout(host: host, visible: screen, barHeight: 56, contentHeight: 56, barOffset: 0, fullscreen: false))
        // 气泡向上展开：面板向上长高，底栏顶边仍然钉在同一个位置。
        let above = try XCTUnwrap(AttachmentGeometry.layout(host: host, visible: screen, barHeight: 56, contentHeight: 320, barOffset: 264, fullscreen: false))
        XCTAssertEqual(above.panel.minY, bar.panel.minY)
        // barOffset 是面板顶边到底栏顶边的距离：320 - 264 = 56，与无气泡时一致。
        XCTAssertEqual(above.panel.maxY - 264, bar.panel.maxY, accuracy: 0.001)
    }

    func testSpaceReportingForBubbleDirection() throws {
        let host = CGRect(x: 0, y: 700, width: 1000, height: 400)
        let placement = try XCTUnwrap(AttachmentGeometry.layout(host: host, visible: screen, barHeight: 56, contentHeight: 56, barOffset: 0, fullscreen: false))
        // 底栏下方永远没有空间（贴着屏幕底），气泡只能向上。
        XCTAssertEqual(placement.spaceBelow, 0)
        XCTAssertEqual(placement.spaceAbove, screen.maxY - (screen.minY + 56), accuracy: 0.001)
        XCTAssertGreaterThan(placement.spaceAbove, 700)
    }

    func testPanelClampedToScreenWidth() throws {
        // 窗口比屏幕还宽、或偏出屏幕左右边缘时，面板收进可见区域。
        let wide = CGRect(x: -200, y: 400, width: 3000, height: 600)
        let placement = try XCTUnwrap(AttachmentGeometry.layout(host: wide, visible: screen, barHeight: 56, contentHeight: 56, barOffset: 0, fullscreen: false))
        XCTAssertGreaterThanOrEqual(placement.panel.minX, screen.minX)
        XCTAssertLessThanOrEqual(placement.panel.maxX, screen.maxX)
        XCTAssertEqual(placement.panel.width, screen.width)
    }

    func testFullscreenUsesTheSameScreenBottomAnchor() throws {
        let host = CGRect(x: 0, y: 24, width: 1440, height: 850)
        let placement = try XCTUnwrap(AttachmentGeometry.layout(host: host, visible: screen, barHeight: 56, contentHeight: 56, barOffset: 0, fullscreen: true))
        XCTAssertEqual(placement.panel.minY, screen.minY)
        XCTAssertEqual(placement.spaceBelow, 0)
    }

    func testDragStaysHiddenUntilReleaseAndSettle() {
        var gate = WindowMotionGate()
        let initial = CGRect(x: 10, y: 10, width: 800, height: 600)
        let moved = initial.offsetBy(dx: 20, dy: 0)
        XCTAssertFalse(gate.hide(frame: initial, pressed: false, now: 0))
        XCTAssertFalse(gate.hide(frame: initial, pressed: true, now: 1))
        XCTAssertTrue(gate.hide(frame: moved, pressed: true, now: 1.1))
        XCTAssertTrue(gate.hide(frame: moved, pressed: true, now: 3))
        XCTAssertTrue(gate.hide(frame: moved, pressed: false, now: 3.1))
        XCTAssertFalse(gate.hide(frame: moved, pressed: false, now: 3.25))
    }
}
