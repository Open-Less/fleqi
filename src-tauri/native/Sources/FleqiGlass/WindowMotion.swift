import AppKit
import CoreImage
import QuartzCore
import WebKit

@MainActor
struct FleqiMotion {
    static var curve:[Float] = [0.2,0.8,0.2,1]
    static var windowDuration:Double = 0.32
    static var feedbackDuration:Double = 0.14
    static var layoutDuration:Double = 0.28
    static var collapsedScale:CGFloat = 0.96
    static var reduced:Bool {NSWorkspace.shared.accessibilityDisplayShouldReduceMotion}
    static func configure(_ value:[String:Any]) {
        if let points=value["curve"] as? [Double],points.count==4,points.allSatisfy({$0.isFinite&&$0>=0&&$0<=1}) {curve=points.map(Float.init)}
        if let durations=value["duration"] as? [String:Double] {
            if let time=durations["window"],time>0&&time<=1 {windowDuration=time}
            if let time=durations["layout"],time>0&&time<=1 {layoutDuration=time}
            if let time=durations["feedback"],time>0&&time<=1 {feedbackDuration=time}
        }
        if let scale=value["collapsedScale"] as? Double,scale>=0.8&&scale<=1 {collapsedScale=CGFloat(scale)}
    }
    static var timing:CAMediaTimingFunction {CAMediaTimingFunction(controlPoints:curve[0],curve[1],curve[2],curve[3])}
    static func contracted(_ frame:CGRect,around anchor:CGPoint)->CGRect {
        let s=collapsedScale
        return CGRect(x:anchor.x+(frame.minX-anchor.x)*s,y:anchor.y+(frame.minY-anchor.y)*s,width:frame.width*s,height:frame.height*s)
    }
}

private var motionKey:UInt8=0
@MainActor
final class NativeWindowMotion {
    weak var window:NSWindow?
    weak var source:NSWindow?
    var origin:CGPoint?
    var fullFrame:CGRect?
    var requested:Bool?
    var revision=0
    // 底栏面板专属：后方滑入滑出 + 高斯模糊 + 动画期间临时降层级。
    var attachment=false
    /// 底栏隐藏时向上收起的距离：等于“底栏相对面板顶边的偏移 + 底栏高度”，
    /// 使底栏刚好完全藏到 Finder 窗口下边缘之后。
    var tuckDistance:CGFloat=0
    /// 底栏正常显示时应该处的层级；动画期间会临时降级，结束后必须回到这里。
    var baseLevel:NSWindow.Level = .floating
    init(window:NSWindow){self.window=window}
    static func forWindow(_ window:NSWindow)->NativeWindowMotion {
        if let existing=objc_getAssociatedObject(window,&motionKey) as? NativeWindowMotion {return existing}
        let motion=NativeWindowMotion(window:window)
        objc_setAssociatedObject(window,&motionKey,motion,.OBJC_ASSOCIATION_RETAIN_NONATOMIC)
        return motion
    }
    func rememberSource(_ source:NSWindow?,rect:CGRect?) {
        self.source=source
        if let source,let rect,let root=source.contentView {
            func webView(_ view:NSView)->WKWebView? {
                if let web=view as? WKWebView {return web}
                return view.subviews.lazy.compactMap{webView($0)}.first
            }
            if let web=webView(root) {
                let point=CGPoint(x:rect.midX,y:web.isFlipped ? rect.midY:web.bounds.height-rect.midY)
                origin=source.convertPoint(toScreen:web.convert(point,to:nil));return
            }
        }
        origin=NSEvent.mouseLocation
    }
    func setVisible(_ visible:Bool,compact:Bool=false) {
        guard let window else{return}
        if requested==visible {return}
        let wasAnimating=requested != nil
        requested=visible;revision+=1
        window.ignoresMouseEvents = !visible
        if !visible && window.isKeyWindow {window.resignKey()}
        let currentRevision=revision
        if visible && window.isVisible && !wasAnimating {window.makeKeyAndOrderFront(nil);return}
        if visible {
            if fullFrame==nil || !window.isVisible {fullFrame=window.frame}
        }else if !window.isVisible {return}
        else if fullFrame==nil {fullFrame=window.frame}
        let target=fullFrame ?? window.frame
        let anchor=origin ?? CGPoint(x:target.midX,y:target.maxY)
        let contracted=compact ? target.offsetBy(dx:0,dy:8):FleqiMotion.contracted(target,around:anchor)
        // 仅底栏（attachment）使用“滑入/滑出 + 高斯模糊”：底栏钉在屏幕底部，
        // 收起距离为负（向屏幕下方滑出），出现时从底边下方滑回原位。
        let tuck=attachment ? target.offsetBy(dx:0,dy:tuckDistance != 0 ? tuckDistance : target.height):contracted
        let startFrame=tuck
        let time=FleqiMotion.reduced ? 0:(compact ? FleqiMotion.feedbackDuration:FleqiMotion.windowDuration)
        if visible && !window.isVisible {
            // 重新出现时先清掉上次消失残留的模糊，快速连续操作也不会带回毛玻璃。
            window.contentView?.layer?.filters=nil
            if attachment {window.level = NSWindow.Level(Int(kCGNormalWindowLevel) - 1)}
            window.setFrame(time==0 ? target:startFrame,display:false)
            window.orderFrontRegardless()
            // 底栏完全靠位置与层级隐藏，不做淡入，避免半透明叠加显得发灰。
            window.alphaValue=attachment ? 1:0
        } else if !visible && attachment && time>0 {
            // 收起时同样降到窗口下方，让底栏“滑回”窗口后面而不是盖在窗口上。
            window.level = NSWindow.Level(Int(kCGNormalWindowLevel) - 1)
        }
        if time==0 && !visible && attachment {
            window.contentView?.layer?.filters=nil
        }
        NSAnimationContext.runAnimationGroup({context in
            context.duration=time;context.timingFunction=FleqiMotion.timing
            if !attachment {window.animator().alphaValue=visible ? 1:0}
            if !compact {window.animator().setFrame(visible ? target:(time==0 ? target:startFrame),display:true)}
        },completionHandler:{[weak self,weak window] in
            MainActor.assumeIsolated {
                guard let self,let window else{return}
                // 无论动画是否被新的调用打断，层级与滤镜都必须恢复干净，
                // 否则底栏会永久留在窗口下方或残留模糊。
                if self.attachment {
                    window.level = self.baseLevel
                    if self.requested != true {window.contentView?.layer?.filters=nil}
                }
                guard self.revision==currentRevision else{return}
                if !visible {
                    window.orderOut(nil);window.setFrame(self.fullFrame ?? target,display:false);window.alphaValue=1
                    if !self.attachment,window.level == NSWindow.Level(Int(kCGNormalWindowLevel) - 1) {window.level = self.baseLevel}
                    if let source=self.source,source.isVisible,NativeWindowMotion.forWindow(source).requested != false {source.makeKeyAndOrderFront(nil)}
                }
                if visible && !compact {window.setFrame(self.fullFrame ?? target,display:true)}
                self.fullFrame=nil
            }
        })
        if attachment && !visible && time>0 && !FleqiMotion.reduced {
            // 消失：上滑同时步进加大高斯模糊，收起后清除。
            let steps=6
            for step in 1...steps {
                DispatchQueue.main.asyncAfter(deadline:.now()+time*Double(step)/Double(steps+1)) {[weak window] in
                    MainActor.assumeIsolated {
                        guard let window else{return}
                        let motion=NativeWindowMotion.forWindow(window)
                        if motion.requested==false {self.applyBlur(window,radius:12*Double(step)/Double(steps))}
                    }
                }
            }
        }
    }
    fileprivate func applyBlur(_ window:NSWindow,radius:Double) {
        guard let content=window.contentView else{return}
        content.wantsLayer=true
        guard let layer=content.layer else{return}
        guard radius>0 else {layer.filters=nil;return}
        let blur=CIFilter(name:"CIGaussianBlur")
        blur?.setValue(radius,forKey:"inputRadius")
        layer.filters=blur.map{[$0]} ?? nil
    }
    // A layout resize while visible is driven by React's animated measured height.
    func followFrame(_ frame:CGRect) {
        if fullFrame != nil {fullFrame=frame}
    }
}

@_cdecl("fleqi_window_visibility")
@MainActor
public func fleqiWindowVisibility(_ pointer:UnsafeMutableRawPointer?,_ sourcePointer:UnsafeMutableRawPointer?,_ x:Double,_ y:Double,_ width:Double,_ height:Double,_ visible:Int32) {
    guard let pointer else{return}
    let window=Unmanaged<NSWindow>.fromOpaque(pointer).takeUnretainedValue()
    let motion=NativeWindowMotion.forWindow(window)
    if visible != 0 {
        let source=sourcePointer.map{Unmanaged<NSWindow>.fromOpaque($0).takeUnretainedValue()}
        motion.rememberSource(source,rect:width>0&&height>0 ? CGRect(x:x,y:y,width:width,height:height):nil)
        window.deminiaturize(nil)
    }
    motion.setVisible(visible != 0)
    // 底栏不主动抢键：保持 Finder 为活动应用；主窗口/设置窗口仍按普通窗口置前。
    if visible != 0 && !motion.attachment {window.makeKeyAndOrderFront(nil)}
    if visible != 0 && motion.attachment {window.orderFrontRegardless()}
}
