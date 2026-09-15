import AppKit
import AuthenticationServices

/// 使用系统浏览器授权会话，支持 GitHub 的登录、Passkey 和组织 SSO。
/// 凭据不会经过 Fleqi WebView；设备授权结果由 Rust 向 GitHub 查询。
@MainActor
final class GitHubAuthorization: NSObject, ASWebAuthenticationPresentationContextProviding {
    static let shared = GitHubAuthorization()
    private var session: ASWebAuthenticationSession?
    private var flowID: String?
    private weak var anchor: NSWindow?

    func open(flowID: String) -> Bool {
        close(flowID: self.flowID ?? "")
        guard let url = URL(string: "https://github.com/login/device"),
              let window = NSApp.keyWindow ?? NSApp.windows.first(where: { $0.isVisible }) else { return false }
        self.flowID = flowID
        anchor = window
        let session = ASWebAuthenticationSession(url: url, callbackURLScheme: nil) { [weak self] _, _ in
            Task { @MainActor in
                guard self?.flowID == flowID else { return }
                self?.session = nil
            }
        }
        session.presentationContextProvider = self
        session.prefersEphemeralWebBrowserSession = false
        self.session = session
        guard session.start() else { self.session = nil; return false }
        return true
    }

    func isOpen(flowID: String) -> Bool { self.flowID == flowID && session != nil }

    func close(flowID: String) {
        guard self.flowID == flowID else { return }
        self.flowID = nil
        let old = session
        session = nil
        old?.cancel()
    }

    func presentationAnchor(for session: ASWebAuthenticationSession) -> ASPresentationAnchor {
        anchor ?? NSApp.keyWindow ?? ASPresentationAnchor()
    }
}
