import AppKit
import ApplicationServices
import Security
import WebKit

private var panelContentKey: UInt8 = 0

@_cdecl("fleqi_prepare_panel")
@MainActor
public func fleqiPreparePanel(_ pointer: UnsafeMutableRawPointer?) {
    guard let pointer else { return }
    let window=Unmanaged<NSWindow>.fromOpaque(pointer).takeUnretainedValue()
    if let content=window.contentView {
        // WebKit must remove its KVO observers before tauri-nspanel changes the
        // window's Objective-C class. Otherwise macOS 27 raises an exception.
        objc_setAssociatedObject(window,&panelContentKey,content,.OBJC_ASSOCIATION_RETAIN_NONATOMIC)
        // Tao requires a non-null contentView while obtaining the native handle.
        // A plain placeholder keeps that invariant without WebKit observers.
        let placeholder=NSView(frame:content.frame)
        placeholder.autoresizingMask=[.width,.height]
        window.contentView=placeholder
    }
}

@_cdecl("fleqi_finish_panel")
@MainActor
public func fleqiFinishPanel(_ pointer: UnsafeMutableRawPointer?) {
    guard let pointer else { return }
    let window=Unmanaged<NSWindow>.fromOpaque(pointer).takeUnretainedValue()
    if let content=objc_getAssociatedObject(window,&panelContentKey) as? NSView {
        window.contentView=content
        objc_setAssociatedObject(window,&panelContentKey,nil,.OBJC_ASSOCIATION_RETAIN_NONATOMIC)
    }
}

private func jsonResult(_ object: Any) -> UnsafeMutablePointer<CChar>? {
    guard let data = try? JSONSerialization.data(withJSONObject: object, options: [.fragmentsAllowed]),
          let text = String(data: data, encoding: .utf8) else { return strdup("{\"error\":\"原生响应编码失败\"}") }
    return strdup(text)
}

@_cdecl("fleqi_string_free")
public func fleqiStringFree(_ pointer: UnsafeMutablePointer<CChar>?) { free(pointer) }

@_cdecl("fleqi_credential")
public func fleqiCredential(_ request: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>? {
    guard let request,
          let data = String(cString: request).data(using: .utf8),
          let value = (try? JSONSerialization.jsonObject(with: data)) as? [String: Any],
          let profile = value["profile"] as? String, let operation = value["operation"] as? String else { return jsonResult(["error": "凭据请求无效"]) }
    let query: [String: Any] = [kSecClass as String: kSecClassGenericPassword,
        kSecAttrService as String: "app.fleqi.desktop", kSecAttrAccount as String: profile]
    if operation == "delete" {
        let status = SecItemDelete(query as CFDictionary)
        return status == errSecSuccess || status == errSecItemNotFound ? jsonResult(["ok": true]) : jsonResult(["error": "无法删除 Fleqi 凭据（\(status)）"])
    }
    if operation == "write" {
        guard let credential = value["credential"], let bytes = try? JSONSerialization.data(withJSONObject: credential) else { return jsonResult(["error": "凭据格式无效"]) }
        let update = [kSecValueData as String: bytes]
        var status = SecItemUpdate(query as CFDictionary, update as CFDictionary)
        if status == errSecItemNotFound {
            var insert = query; insert[kSecValueData as String] = bytes
            insert[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
            status = SecItemAdd(insert as CFDictionary, nil)
        }
        return status == errSecSuccess ? jsonResult(["ok": true]) : jsonResult(["error": "Keychain 保存失败（\(status)）"])
    }
    var read = query
    read[kSecMatchLimit as String] = kSecMatchLimitOne
    read[operation == "has" ? kSecReturnAttributes as String : kSecReturnData as String] = true
    var result: CFTypeRef?
    let status = SecItemCopyMatching(read as CFDictionary, &result)
    if status == errSecItemNotFound { return jsonResult(["value": NSNull()]) }
    guard status == errSecSuccess else { return jsonResult(["error": "Keychain 读取失败（\(status)）"]) }
    if operation == "has" { return jsonResult(["value": true]) }
    guard let bytes = result as? Data, let credential = try? JSONSerialization.jsonObject(with: bytes) else { return jsonResult(["error": "已保存的凭据无法读取，请重新登录"]) }
    return jsonResult(["value": credential])
}

@MainActor
private final class FinderHost {
    static let shared = FinderHost()
    weak var panel: NSWindow?
    var requested = false
    /// 面板内容高度（底栏 + 气泡），由前端测量后传入。
    var height: CGFloat = 56
    var barHeight: CGFloat = 56
    /// 底栏顶边到面板顶边的距离：气泡向下展开时为 0，向上展开时等于气泡高度。
    var barOffset: CGFloat = 0
    var spaceBelow: CGFloat = 0
    var spaceAbove: CGFloat = 0
    var motion=WindowMotionGate()
    var timer: Timer?
    var observer: AXObserver?
    var observedWindow: AXUIElement?
    // 底栏跟随“唤起时所在”的那个 Finder 窗口；切换空间或其他窗口不改绑。
    var pinnedWindow: AXUIElement?
    var lastWindow: AXUIElement?
    var windowKinds: [(AXUIElement, Bool, TimeInterval)] = []
    var lastFrame: NSRect?
    var automatic = false
    var workspaceToken: NSObjectProtocol?
    /// 上一次命中选中项的那个容器（outline/table/list）。Finder 的选区挂在很深的
    /// 子视图上，每次都走整棵树太贵（最多 2500 次 AX 调用）。轮询时先问缓存下来的
    /// 容器，命中就只要一两次属性读取，用来做 200ms 级的选区跟随。
    var selectionAnchor: AXUIElement?

    init() {
        workspaceToken = NSWorkspace.shared.notificationCenter.addObserver(forName: NSWorkspace.didActivateApplicationNotification, object: nil, queue: .main) { _ in
            Task { @MainActor in
                let host = FinderHost.shared
                if host.automatic && NSWorkspace.shared.frontmostApplication?.bundleIdentifier == "com.apple.finder" {
                    host.requested = true; host.startTimer(); host.layout()
                }
            }
        }
    }
    func startTimer() {
        guard timer == nil else {return}
        let timer=Timer(timeInterval:0.033,repeats:true) { _ in Task { @MainActor in FinderHost.shared.layout() } }
        RunLoop.main.add(timer,forMode:.common);self.timer=timer
    }

    func finder() -> NSRunningApplication? {
        NSRunningApplication.runningApplications(withBundleIdentifier: "com.apple.finder").first
    }
    func attribute(_ element: AXUIElement, _ name: String) -> CFTypeRef? {
        var result: CFTypeRef?
        guard AXUIElementCopyAttributeValue(element, name as CFString, &result) == .success else { return nil }
        return result
    }
    func window() -> AXUIElement? {
        guard AXIsProcessTrusted(), let app = finder() else { return nil }
        let element = AXUIElementCreateApplication(app.processIdentifier)
        AXUIElementSetMessagingTimeout(element, 0.2)
        let windows = attribute(element, kAXWindowsAttribute) as? [AXUIElement] ?? []
        // 已锁定唤起时的窗口：只要它仍存在就继续跟随，不切换到其他窗口。
        if let pinnedWindow, windows.contains(where: { CFEqual($0, pinnedWindow) }) { return pinnedWindow }
        pinnedWindow = nil
        windowKinds.removeAll { entry in !windows.contains { CFEqual($0, entry.0) } }
        func inspectBrowser(_ candidate: AXUIElement) -> Bool {
            guard attribute(candidate, kAXSubroleAttribute) as? String == kAXStandardWindowSubrole,
                  attribute(candidate, kAXModalAttribute) as? Bool != true,
                  frame(candidate) != nil else { return false }
            // Both file and folder Quick Look previews may expose AXDocument.
            // Check the browser hierarchy and Finder window set, never the title.
            var remaining = 100
            func hasRole(_ root: AXUIElement, _ role: String, _ depth: Int) -> Bool {
                guard depth > 0, remaining > 0 else { return false }
                remaining -= 1
                if attribute(root, kAXRoleAttribute) as? String == role { return true }
                return (attribute(root, kAXChildrenAttribute) as? [AXUIElement] ?? []).contains { hasRole($0, role, depth - 1) }
            }
            guard hasRole(candidate, kAXSplitGroupRole, 5) else { return false }
            guard automation(false) == noErr, let candidateFrame = frame(candidate) else { return false }
            // Finder's scripting dictionary excludes Quick Look, Info and other
            // auxiliary windows. Verify against that set once per new AX window.
            var error: NSDictionary?
            guard let program = NSAppleScript(source: "tell application \"Finder\" to return bounds of every Finder window") else { return false }
            let bounds = program.executeAndReturnError(&error)
            guard error == nil, bounds.numberOfItems > 0 else { return false }
            for index in 1...bounds.numberOfItems {
                guard let row = bounds.atIndex(index), row.numberOfItems == 4 else { continue }
                let x = row.atIndex(1)?.int32Value ?? 0
                let y = row.atIndex(2)?.int32Value ?? 0
                let rect = CGRect(x: Int(x), y: Int(y), width: Int((row.atIndex(3)?.int32Value ?? 0)-x), height: Int((row.atIndex(4)?.int32Value ?? 0)-y))
                if close(candidateFrame, rect) { return true }
            }
            return false
        }
        func eligible(_ candidate: AXUIElement) -> Bool {
            let now = ProcessInfo.processInfo.systemUptime
            if let known = windowKinds.first(where: { CFEqual($0.0, candidate) }), known.1 || now-known.2 < 0.5 { return known.1 }
            let browser = inspectBrowser(candidate)
            windowKinds.removeAll { CFEqual($0.0, candidate) }
            windowKinds.append((candidate, browser, now))
            return browser
        }
        if let focused = attribute(element, kAXFocusedWindowAttribute), CFGetTypeID(focused) == AXUIElementGetTypeID() {
            let candidate = unsafeBitCast(focused, to: AXUIElement.self)
            if eligible(candidate) { return candidate }
        }
        // Space / Quick Look must never replace the previously associated
        // Finder browser. A closed window cannot survive this membership check.
        if let lastWindow, windows.contains(where: { CFEqual($0, lastWindow) }), eligible(lastWindow) { return lastWindow }
        return windows.first(where: eligible)
    }
    func frame(_ element: AXUIElement) -> CGRect? {
        guard let position = attribute(element, kAXPositionAttribute), let size = attribute(element, kAXSizeAttribute),
              CFGetTypeID(position) == AXValueGetTypeID(), CFGetTypeID(size) == AXValueGetTypeID() else { return nil }
        var origin = CGPoint.zero; var dimensions = CGSize.zero
        guard AXValueGetValue(unsafeBitCast(position, to: AXValue.self), .cgPoint, &origin),
              AXValueGetValue(unsafeBitCast(size, to: AXValue.self), .cgSize, &dimensions) else { return nil }
        return CGRect(origin: origin, size: dimensions)
    }
    func setFrame(_ element: AXUIElement, _ rect: CGRect) -> Bool {
        var origin = rect.origin; var size = rect.size
        guard let position = AXValueCreate(.cgPoint, &origin), let dimensions = AXValueCreate(.cgSize, &size) else { return false }
        let moved = AXUIElementSetAttributeValue(element, kAXPositionAttribute as CFString, position)
        let resized = AXUIElementSetAttributeValue(element, kAXSizeAttribute as CFString, dimensions)
        return moved == .success && resized == .success
    }
    func automation(_ request: Bool) -> OSStatus {
        var target = AEAddressDesc()
        let name = "com.apple.finder"
        let created = name.withCString { AECreateDesc(DescType(typeApplicationBundleID), $0, name.utf8.count, &target) }
        guard created == noErr else { return OSStatus(created) }
        defer { AEDisposeDesc(&target) }
        return AEDeterminePermissionToAutomateTarget(&target, AEEventClass(typeWildCard), AEEventID(typeWildCard), request)
    }
    func permissions() -> [[String: Any]] {
        let accessibility = AXIsProcessTrusted()
        let automationStatus = automation(false)
        return [
            ["id": "accessibility", "name": "辅助功能", "purpose": "跟随和调整 Finder 窗口", "status": accessibility ? "granted" : "not_granted", "detail": accessibility ? "系统已授权；请运行 Finder 检查验证窗口访问" : "在系统设置中允许 Fleqi 控制窗口", "required": true],
            ["id": "automation", "name": "Finder 自动化", "purpose": "读取 Finder 目录的补充来源", "status": automationStatus == noErr ? "granted" : "not_granted", "detail": automationStatus == noErr ? "已授权；选区默认由辅助功能读取，此项仅作补充" : "可选：未授权时仍可通过辅助功能读取目录与选区", "required": false],
            ["id": "files", "name": "文件与文件夹", "purpose": "读取与处理选中的文件", "status": "unknown", "detail": "点击检查后逐项验证桌面、文稿与下载；普通项目目录不受系统限制，不会出现在系统列表", "required": true],
            ["id": "full_disk", "name": "完全磁盘访问", "purpose": "处理受保护位置中的文件", "status": "unknown", "detail": "系统开关不会自动反馈到应用；点击检查验证授权是否生效", "required": false],
        ]
    }
    /// 窗口里所有行。Finder 内容列表的行在名称单元格上带 file:// 的 AXURL，
    /// 边栏的行没有 URL，据此过滤即可区分文件与位置。
    func rows(_ window: AXUIElement) -> [AXUIElement] {
        var result: [AXUIElement] = []
        var visited = 0
        func walk(_ element: AXUIElement, _ depth: Int) {
            guard depth > 0, visited < 2500 else { return }
            visited += 1
            let role = attribute(element, kAXRoleAttribute) as? String
            if role == kAXRowRole { result.append(element) }
            for child in attribute(element, kAXChildrenAttribute) as? [AXUIElement] ?? [] { walk(child, depth - 1) }
        }
        walk(window, 12)
        return result
    }
    /// 行内的第一个文件地址：列表视图在名称单元格上，图标视图通常在图标本身。
    func rowURL(_ row: AXUIElement) -> URL? {
        var found: URL?
        var visited = 0
        func walk(_ element: AXUIElement, _ depth: Int) {
            guard depth > 0, visited < 60, found == nil else { return }
            visited += 1
            if let value = attribute(element, kAXURLAttribute) as? URL, value.isFileURL { found = value; return }
            if let text = attribute(element, kAXURLAttribute) as? String, let url = URL(string: text), url.isFileURL { found = url; return }
            for child in attribute(element, kAXChildrenAttribute) as? [AXUIElement] ?? [] { walk(child, depth - 1) }
        }
        walk(row, 4)
        return found
    }
    /// 当前选中的行：列表与分栏视图用 AXSelectedRows，图标视图用 AXSelected。
    /// 同一行可能同时命中两条路径，按元素去重，避免同一文件被算作多次。
    func selectedRows(_ window: AXUIElement) -> [AXUIElement] {
        var rows: [AXUIElement] = []
        var visited = 0
        func known(_ candidate: AXUIElement) -> Bool { rows.contains { CFEqual($0, candidate) } }
        func walk(_ element: AXUIElement, _ depth: Int) {
            guard depth > 0, visited < 2500 else { return }
            visited += 1
            let role = attribute(element, kAXRoleAttribute) as? String
            if role == kAXOutlineRole || role == kAXTableRole || role == kAXListRole {
                let selected = attribute(element, kAXSelectedRowsAttribute) as? [AXUIElement] ?? []
                if !selected.isEmpty { selectionAnchor = element }
                for row in selected where !known(row) {
                    rows.append(row)
                }
            } else if role == kAXRowRole || role == kAXImageRole || role == kAXCellRole,
                      attribute(element, kAXSelectedAttribute) as? Bool == true, !known(element) {
                rows.append(element)
            }
            for child in attribute(element, kAXChildrenAttribute) as? [AXUIElement] ?? [] { walk(child, depth - 1) }
        }
        walk(window, 12)
        return rows
    }
    /// 候选选中项：先用缓存容器做一两次 AX 读取，未命中才退回整棵树。
    func candidateRows(_ window: AXUIElement) -> [AXUIElement] {
        if let anchor = selectionAnchor,
           (attribute(anchor, kAXRoleAttribute) as? String).map({ $0 == kAXOutlineRole || $0 == kAXTableRole || $0 == kAXListRole }) == true,
           let selected = attribute(anchor, kAXSelectedRowsAttribute) as? [AXUIElement], !selected.isEmpty {
            return selected
        }
        return selectedRows(window)
    }
    /// 选中路径：按行取地址、去重、丢掉“列视图里被顺带选中的父目录”。
    /// 在分栏视图里选中文件时，上一层分栏的当前目录也会处于选中态，
    /// 不清理就会把整个文件夹一起当成待处理对象。
    func selectionPaths(_ window: AXUIElement) -> [String] {
        var files: [String] = []
        for row in candidateRows(window) {
            guard let path = rowURL(row)?.path, FileManager.default.fileExists(atPath: path), !files.contains(path) else { continue }
            files.append(path)
        }
        if files.isEmpty { files = scriptSelection() }
        let directories = Set(files.filter { path in
            var isDirectory: ObjCBool = false
            return FileManager.default.fileExists(atPath: path, isDirectory: &isDirectory) && isDirectory.boolValue
        })
        guard !directories.isEmpty else { return files }
        return files.filter { path in
            guard directories.contains(path) else { return true }
            return !files.contains { other in other != path && (other as NSString).deletingLastPathComponent == path }
        }
    }
    /// 轻量选区读取：不含目录解析与窗口校验，供底栏高频轮询使用。
    func selection() -> [String: Any] {
        guard AXIsProcessTrusted() else { return ["error": "请先授予辅助功能权限"] }
        guard let window = window() else { return ["error": "没有可关联的 Finder 窗口"] }
        return ["files": selectionPaths(window)]
    }
    /// 目录来源：优先选中文件所在目录，其次窗口内任意项目的所在目录，
    /// 最后才回退到 AppleScript（因此 Finder 自动化权限是可选的）。
    func windowDirectory(_ window: AXUIElement, files: [String]) -> String? {
        if let first = files.first { return (first as NSString).deletingLastPathComponent }
        // 逐个试到第一个带地址的行即可；大目录不会把整棵树都读完。
        for row in rows(window) {
            if let url = rowURL(row) { return url.deletingLastPathComponent().path }
        }
        guard automation(false) == noErr else { return nil }
        var error: NSDictionary?
        guard let program = NSAppleScript(source: "tell application \"Finder\" to return POSIX path of (target of front Finder window as alias)") else { return nil }
        let result = program.executeAndReturnError(&error)
        guard error == nil, let directory = result.stringValue else { return nil }
        var isDirectory: ObjCBool = false
        guard FileManager.default.fileExists(atPath: directory, isDirectory: &isDirectory), isDirectory.boolValue else { return nil }
        return directory
    }
    func scriptSelection() -> [String] {
        guard automation(false) == noErr else { return [] }
        let script = """
        tell application "Finder"
            if (count of Finder windows) is 0 then return {}
            set selectedPaths to {}
            repeat with chosen in (selection of front Finder window)
                set end of selectedPaths to POSIX path of (chosen as alias)
            end repeat
            return selectedPaths
        end tell
        """
        var error: NSDictionary?
        guard let program = NSAppleScript(source: script) else { return [] }
        let result = program.executeAndReturnError(&error)
        guard error == nil, result.numberOfItems > 0 else { return [] }
        var paths: [String] = []
        for index in 1...result.numberOfItems {
            guard let path = result.atIndex(index)?.stringValue else { continue }
            if FileManager.default.fileExists(atPath: path) { paths.append(path) }
        }
        return paths
    }
    func context() -> [String: Any] {
        guard AXIsProcessTrusted() else { return ["error": "请先授予辅助功能权限"] }
        guard let before = window(), let beforeFrame = frame(before) else { return ["error": "没有可关联的 Finder 窗口，请先打开 Finder"] }
        let files = selectionPaths(before)
        guard let directory = windowDirectory(before, files: files) else { return ["error": "无法读取 Finder 当前目录，请先在窗口中选中文件或授予 Finder 自动化权限"] }
        guard let after = window(), CFEqual(before, after), let afterFrame = frame(after), close(beforeFrame, afterFrame) else { return ["error": "Finder 窗口或选区在捕获期间发生变化，请重新提交"] }
        let windowID = attribute(before, kAXIdentifierAttribute) as? String ?? String(CFHash(before))
        return ["id": UUID().uuidString, "hostWindow": "finder:\(windowID):\(CFHash(before))", "directory": directory, "files": files, "capturedAt": Int(Date().timeIntervalSince1970 * 1000)]
    }
    func observe(_ window: AXUIElement) {
        if let old = observedWindow, CFEqual(old, window) { return }
        if let observer { CFRunLoopRemoveSource(CFRunLoopGetMain(), AXObserverGetRunLoopSource(observer), .commonModes) }
        guard let app = finder() else { return }
        var newObserver: AXObserver?
        let callback: AXObserverCallback = { _, _, _, _ in
            DispatchQueue.main.async { FinderHost.shared.layout() }
        }
        guard AXObserverCreate(app.processIdentifier, callback, &newObserver) == .success, let newObserver else { return }
        for notification in [kAXMovedNotification, kAXResizedNotification, kAXUIElementDestroyedNotification, kAXWindowMiniaturizedNotification, kAXWindowDeminiaturizedNotification] {
            AXObserverAddNotification(newObserver, window, notification as CFString, nil)
        }
        observer = newObserver; observedWindow = window
        CFRunLoopAddSource(CFRunLoopGetMain(), AXObserverGetRunLoopSource(newObserver), .commonModes)
    }
    func close(_ a: CGRect, _ b: CGRect) -> Bool {
        abs(a.minX-b.minX)<2 && abs(a.minY-b.minY)<2 && abs(a.width-b.width)<2 && abs(a.height-b.height)<2
    }
    func layout() {
        guard requested, let panel else { return }
        guard let running = finder(), !running.isHidden, let window = window(), let quartz = frame(window),
              (attribute(window, kAXMinimizedAttribute) as? Bool) != true else { NativeWindowMotion.forWindow(panel).setVisible(false,compact:true); return }
        let front = NSWorkspace.shared.frontmostApplication?.bundleIdentifier
        let ours = Bundle.main.bundleIdentifier
        // A nonactivating panel can accept typing while Finder remains the host.
        guard front == "com.apple.finder" || (front == ours && panel.isKeyWindow) else { NativeWindowMotion.forWindow(panel).setVisible(false,compact:true); return }
        if let lastWindow, !CFEqual(lastWindow, window) { lastFrame = nil;motion.reset() }
        lastWindow = window; observe(window)
        // 拖动 Finder 窗口时先收起底栏，松手后重新显示。
        if motion.hide(frame:quartz,pressed:NSEvent.pressedMouseButtons&1 != 0,now:ProcessInfo.processInfo.systemUptime) {NativeWindowMotion.forWindow(panel).setVisible(false,compact:true);return}
        let primaryTop = NSScreen.screens.first?.frame.maxY ?? 0
        let rect = NSRect(x: quartz.minX, y: primaryTop-quartz.maxY, width: quartz.width, height: quartz.height)
        let screen = NSScreen.screens.max { a,b in a.frame.intersection(rect).width*a.frame.intersection(rect).height < b.frame.intersection(rect).width*b.frame.intersection(rect).height }
        guard let screen else { NativeWindowMotion.forWindow(panel).setVisible(false,compact:true); return }
        let fullscreen = (attribute(window, "AXFullScreen") as? Bool) == true
        // Finder 窗口本身从不被移动或缩放：底栏只在窗口下方，空间不足时面板留在屏幕内。
        guard let placement=AttachmentGeometry.layout(host:rect,visible:screen.visibleFrame,barHeight:barHeight,contentHeight:height,barOffset:barOffset,fullscreen:fullscreen) else{NativeWindowMotion.forWindow(panel).setVisible(false,compact:true);return}
        spaceBelow=placement.spaceBelow;spaceAbove=placement.spaceAbove
        let target=placement.panel
        if lastFrame != target {
            NativeWindowMotion.forWindow(panel).followFrame(target)
            panel.setFrame(target,display:true)
            if let content=panel.contentView {
                content.setFrameSize(target.size)
                func fillWebView(_ view:NSView) {
                    for child in view.subviews {
                        if let web=child as? WKWebView {web.frame=view.bounds;web.autoresizingMask=[.width,.height]}
                        else {fillWebView(child)}
                    }
                }
                fillWebView(content)
            }
            lastFrame=target
            panel.invalidateShadow()
        }
        NativeWindowMotion.forWindow(panel).setVisible(true,compact:true)
    }
    func show() -> [String: Any] {
        guard AXIsProcessTrusted() else { return ["error": "请先在权限设置中授予辅助功能权限"] }
        guard let host = window() else { return ["error": "请先打开 Finder 文件窗口"] }
        pinnedWindow = host
        requested = true
        finder()?.activate(options: [])
        startTimer()
        layout(); panel?.orderFrontRegardless()
        return ["ok": true]
    }
    func hide() {
        let wasKey = panel?.isKeyWindow == true
        requested=false; pinnedWindow=nil
        if let panel {NativeWindowMotion.forWindow(panel).setVisible(false,compact:true)}
        timer?.invalidate(); timer=nil; lastFrame=nil;motion.reset()
        // 面板在键时收起，把焦点还给 Finder，避免落回 Fleqi 主窗口。
        if wasKey { finder()?.activate(options: []) }
    }
}

@_cdecl("fleqi_attach_panel")
@MainActor
public func fleqiAttachPanel(_ pointer: UnsafeMutableRawPointer?) {
    guard let pointer else { return }
    let panel = Unmanaged<NSWindow>.fromOpaque(pointer).takeUnretainedValue()
    // 不加入所有空间：底栏只出现在唤起时的那个桌面。
    panel.collectionBehavior = [.fullScreenAuxiliary, .ignoresCycle]
    // 阴影完全由网页层绘制：透明窗口的系统阴影会在磨砂玻璃边缘留下黑边。
    panel.level = .floating; panel.isOpaque=false; panel.backgroundColor = .clear; panel.hasShadow = false
    let motion=NativeWindowMotion.forWindow(panel)
    motion.attachment = true
    motion.baseLevel = .floating
    FinderHost.shared.panel = panel
}

@_cdecl("fleqi_host_command")
public func fleqiHostCommand(_ pointer: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>? {
    guard let pointer, let data=String(cString:pointer).data(using:.utf8),
          let request=(try? JSONSerialization.jsonObject(with:data)) as? [String:Any] else {return jsonResult(["error":"宿主请求无效"])}
    let work = { @MainActor () -> UnsafeMutablePointer<CChar>? in
        let host=FinderHost.shared
        switch request["operation"] as? String {
        case "github_auth_open":
            guard let id=request["flowId"] as? String,UUID(uuidString:id) != nil else{return jsonResult(["error":"登录请求无效"])}
            return GitHubAuthorization.shared.open(flowID:id) ? jsonResult(["ok":true]) : jsonResult(["error":"无法打开 GitHub 授权窗口，请重试"])
        case "github_auth_status":
            return jsonResult(["open":GitHubAuthorization.shared.isOpen(flowID:request["flowId"] as? String ?? "")])
        case "github_auth_close":
            GitHubAuthorization.shared.close(flowID:request["flowId"] as? String ?? "")
            return jsonResult(["ok":true])
        case "motion_config": FleqiMotion.configure(request["config"] as? [String:Any] ?? [:]);return jsonResult(["ok":true])
        case "permissions": return jsonResult(host.permissions())
        case "context": return jsonResult(host.context())
        case "selection": return jsonResult(host.selection())
        case "show": return jsonResult(host.show())
        case "hide": host.hide(); return jsonResult(["ok":true])
        case "status": return jsonResult(["requested":host.requested,"visible":host.panel?.isVisible ?? false])
        case "copy_text":
            guard let text=request["text"] as? String,text.utf8.count<=1_000_000 else{return jsonResult(["error":"复制内容无效"])}
            NSPasteboard.general.clearContents()
            return NSPasteboard.general.setString(text,forType:.string) ? jsonResult(["ok":true]) : jsonResult(["error":"无法写入剪贴板"])
        case "automatic":
            host.automatic = request["enabled"] as? Bool ?? false
            if host.automatic {host.requested=true;host.startTimer();host.layout()}
            else {host.hide()}
            return jsonResult(["ok":true])
        case "height":
            if let value=request["height"] as? Double,value.isFinite {host.height=CGFloat(min(1600,max(50,value)))}
            if let value=request["barHeight"] as? Double,value.isFinite {host.barHeight=CGFloat(min(80,max(50,value)))}
            if let value=request["barOffset"] as? Double,value.isFinite {host.barOffset=CGFloat(min(1600,max(0,value)))}
            if let panel=host.panel {
                let motion=NativeWindowMotion.forWindow(panel)
                // 收起时向屏幕下方滑出“面板高度 - 底栏顶边偏移 + 2”，把底栏整条推出屏幕底边；
                // 面板底边本就贴屏幕底部，气泡在上方不会露出来。
                motion.tuckDistance = -(host.height - host.barOffset) - 2
            }
            host.layout()
            return jsonResult(["ok":true,"spaceBelow":Double(host.spaceBelow),"spaceAbove":Double(host.spaceAbove)])
        case "refocus": if host.requested {host.finder()?.activate(options:[])};return jsonResult(["ok":true])
        case "request_permission":
            if request["id"] as? String == "accessibility" {let options=[kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String:true];_ = AXIsProcessTrustedWithOptions(options as CFDictionary)}
            else if request["id"] as? String == "automation" {_ = host.automation(true)}
            return jsonResult(host.permissions())
        default:return jsonResult(["error":"未知宿主操作"])
        }
    }
    // Only the owned strdup response crosses this boundary, never AppKit state.
    let address: Int
    if Thread.isMainThread { address = MainActor.assumeIsolated { Int(bitPattern: work()) } }
    else { address = DispatchQueue.main.sync { MainActor.assumeIsolated { Int(bitPattern: work()) } } }
    return UnsafeMutablePointer<CChar>(bitPattern: address)
}
