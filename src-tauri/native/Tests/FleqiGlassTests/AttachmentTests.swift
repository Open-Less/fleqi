import XCTest
@testable import FleqiGlass

final class AttachmentTests: XCTestCase {
    let screen = CGRect(x: 0, y: 24, width: 1440, height: 850)

    func testBarStaysAlignedUnderTheWindowAndNeverMovesTheWindow() throws {
        let host = CGRect(x: 140, y: 400, width: 1000, height: 600)
        let bar = try XCTUnwrap(AttachmentGeometry.layout(host: host, visible: screen, barHeight: 56, contentHeight: 56, barOffset: 0, fullscreen: false))
        XCTAssertEqual(bar.panel.minX, host.minX)
        XCTAssertEqual(bar.panel.width, host.width)
        // 底栏顶边在窗口下边缘下方 8pt，且面板整体位于窗口之外。
        XCTAssertEqual(host.minY - bar.panel.maxY, 8)
        // 气泡向下展开：面板向屏幕下方长高，顶边（底栏位置）不动。
        let popup = try XCTUnwrap(AttachmentGeometry.layout(host: host, visible: screen, barHeight: 56, contentHeight: 320, barOffset: 0, fullscreen: false))
        XCTAssertEqual(popup.panel.maxY, bar.panel.maxY)
        XCTAssertLessThan(popup.panel.minY, bar.panel.minY)
    }

    func testBubbleAboveKeepsTheBarInPlace() throws {
        let host = CGRect(x: 0, y: 120, width: 1000, height: 600)
        let bar = try XCTUnwrap(AttachmentGeometry.layout(host: host, visible: screen, barHeight: 56, contentHeight: 56, barOffset: 0, fullscreen: false))
        // 气泡向上展开：面板向上长高，底栏顶边仍然固定在窗口下方 8pt。
        let above = try XCTUnwrap(AttachmentGeometry.layout(host: host, visible: screen, barHeight: 56, contentHeight: 320, barOffset: 264, fullscreen: false))
        XCTAssertEqual(above.panel.maxY, bar.panel.maxY + 264)
        XCTAssertEqual(above.panel.minY, bar.panel.minY)
        XCTAssertEqual(above.panel.minY + 56, bar.panel.minY + 56)
    }

    func testSpaceReportingForBubbleDirection() throws {
        let roomy = CGRect(x: 0, y: 700, width: 1000, height: 400)
        let roomyPlacement = try XCTUnwrap(AttachmentGeometry.layout(host: roomy, visible: screen, barHeight: 56, contentHeight: 56, barOffset: 0, fullscreen: false))
        XCTAssertGreaterThan(roomyPlacement.spaceBelow, 200)
        XCTAssertGreaterThan(roomyPlacement.spaceAbove, 150)
        // 窗口贴着屏幕底部时，下方没有空间，气泡必须向上。
        let tight = CGRect(x: 0, y: 24, width: 1000, height: 400)
        let tightPlacement = try XCTUnwrap(AttachmentGeometry.layout(host: tight, visible: screen, barHeight: 56, contentHeight: 56, barOffset: 0, fullscreen: false))
        XCTAssertEqual(tightPlacement.spaceBelow, 0)
        XCTAssertGreaterThan(tightPlacement.spaceAbove, 200)
        // 面板始终留在屏幕可见区域内，Finder 窗口本身不被改动。
        XCTAssertGreaterThanOrEqual(tightPlacement.panel.minY, screen.minY)
        XCTAssertLessThanOrEqual(tightPlacement.panel.maxY, screen.maxY)
    }

    func testFullscreenKeepsTheBarInsideTheWindow() throws {
        let host = CGRect(x: 0, y: 24, width: 1440, height: 850)
        let placement = try XCTUnwrap(AttachmentGeometry.layout(host: host, visible: screen, barHeight: 56, contentHeight: 56, barOffset: 0, fullscreen: true))
        XCTAssertEqual(placement.panel.minY, host.minY + 8)
        XCTAssertLessThan(placement.spaceBelow, 20)
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
