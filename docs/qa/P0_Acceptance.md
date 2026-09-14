# Fleqi P0 实施与验收记录

更新：2026-09-14。本轮按用户要求先完成不依赖账号的验证。**已实现应用骨架、文件核心、PI 接入及新界面；完整 P0 发布验收尚未通过。**

## 可以检查的交付

- [工作区截图](../previews/dashboard-overview.png)：shadcn `sidebar-07`、中性色、固定一屏概览、分区滚动、底部连接菜单和右下角设置。
- [设置截图](../previews/settings-general.png)：`sidebar-13` 独立弹窗，七个设置页面均接入实际后端接口；[黑色](../previews/settings-dark.png)与[紧凑概览](../previews/dashboard-compact.png)。
- 原生应用：`src-tauri/target/debug/bundle/macos/Fleqi.app`。已成功构建，重新启动才会加载新版原生代码。
- [架构与准备图](../engineering/P0_Architecture_and_Preparation.md)、[UI 规范](../design/P0_UI_Spec.md)、[开发说明](../../README.md)、[贡献指南](../../AGENTS.md)。

底部栏采用 56 pt 栏高、8 pt 间距、Finder 全宽；支持单色 Beam、按键录入与斜杠菜单。Codex 连接不要求手填地址或模型 ID；账号/API 模型目录、思考与速度通过实际服务返回值读取。相关服务真实可用性需要独立账号验证。

## 已通过的检查

| 检查 | 结果与证据 | 不覆盖的场景 |
| --- | --- | --- |
| 前端构建 | `pnpm build`，TypeScript + Vite 通过 | 原生窗口运行 |
| 界面 | `pnpm test`，18 项通过；深浅尺寸、IME、Beam、斜杠键盘、设置、快捷键录入、概览滚动边界，以及弹窗/菜单/侧栏/表单尺寸动效、统一字体和减少动态效果 | macOS 材质、真实账号 |
| 公共核心 | 9 项通过；去重/快照、SQLite 恢复/迁移、特殊路径、替换对象拒绝、文件夹原位改名、图片、ZIP、DOCX | 所有磁盘与云盘边界 |
| 实际媒体/PDF 引擎 | 2 项通过；随包 FFmpeg/ffprobe/qpdf 处理音频、体积目标、PDF 合并/提取/旋转/拆分 | 全部编码与异常媒体 |
| PI 运行时 | `pnpm test:runtime`，9 项通过；SDK 加载、凭据隔离、取消、模型列表及四类本地 HTTP 协议服务 | 真实 API 服务、Codex 登录/刷新 |
| Swift 布局 | 3 项通过；左右等宽/间距、菜单展开底边稳定、空间不足/全屏位置、拖动隐藏/释放稳定 | 真实 AX 事件与 Quick Look |
| 原生静态检查 | Rust Clippy `-D warnings` 与 Swift/Tauri 编译通过 | macOS 15 实机兼容 |
| 最终包运行时 | `pnpm test:package` 通过；从最终 `.app` 解包，保留目录链接，使用包内 Node 和仅系统 PATH 加载 PI SDK；缓存复用通过 | 干净 macOS 15 实机 |
| 应用打包 | 固定 Node/PI 与文件引擎随包；修复打包器遗漏 pnpm 目录链接，改为完整归档与版本缓存；引擎未链接 Homebrew 动态库 | 干净机器、签名与公证 |

PI 的四类协议测试使用本地测试服务和测试用凭据，没有调用用户账号，不作为真实账号成功证据。文件检查只使用可清理样本。

## PRD 对照

| 编号 | 实现与已知证据 | 下一项验收 |
| --- | --- | --- |
| A0-01 / A0-02 | 随包 Node + 固定 PI；底部无外部 PI 切换或环境变量回退 | 干净环境真实模型文件任务 |
| A0-03 | 权限用途、独立请求、系统入口与复检已接入 | 新包拒绝/授权/撤销逐项验证 |
| A0-04 | PI 新建、读取、改名、清理的自检链已实现 | 配置账号后执行完整自检 |
| A0-05 | 未授权/未知/不支持分开显示 | 系统撤销后的真实访问行为 |
| A0-06 | NSPanel、位置观察、移动隐藏、稳定恢复、Quick Look 排除已实现 | 多窗口、标签、Quick Look、全屏和多屏实测 |
| A0-07 | 请求固定模型与文件快照；选区与窗口采样前后核对 | 真实 Finder 中途切换与替换对象 |
| A0-08 | 底部仅问题、结果、任务状态与简短错误；终端留待 P1 | 真实长任务界面 |
| A0-09 | 基础文件、ZIP、图片、媒体、PDF、文本与简单 DOCX 执行器 | 逐动作正常/失败/权限/体积样本矩阵 |
| A0-10 | 取消信号、已完成动作账本、未知进度无伪造百分比 | 长媒体任务取消及部分完成 |
| A0-11 | 自动模型目录、能力档位、四协议测试、Codex 登录桥接 | 真实登录、刷新、退出及账号调用分别验收 |
| A0-12 | 原生菜单、单实例、独立设置、后台关闭与恢复已实现 | 新版菜单/Dock/快捷键冲突复测 |
| A0-13 / A0-14 | PI 独立配置与工具集；macOS 桥独立于 P1 | 干净 macOS 15+ Apple Silicon |
| A0-15 | 中文候选确认、连点防护、请求去重测试通过 | 原生输入法与焦点切换 |
| A0-16 | 路径范围、链接/ZIP 越界、原件保护、正文外发确认；诊断脱敏 | 只读、满盘、网络盘、云占位文件；诊断导出实测 |
| A0-17 | 运行前记录动作；中断标记待核对，不自动重跑 | 强退及执行中异常的原生场景 |

## 当前原生验证限制

更新过程中，UI 工具报告 Mac 已锁定、无法自动解锁。新版独立设置、Finder 等宽挂靠、拖动隐藏及 Quick Look 修复尚未完成新包实测。旧包此前已成功启动，但不能用旧包结果替代新包验证。解锁后应先退出旧进程，再打开新包；不需要账号即可复测菜单、设置、快捷键、窗口绑定和权限状态。

用户要求先验证无账号部分，因此没有代填密钥或触发新的账号授权。真实服务调用与完整 PI 文件自检待用户配置后继续。

## 发布前与后续平台

首版以 macOS 15+ Apple Silicon 为目标，液态玻璃按 macOS 26+ 可用性启用。Windows/Linux 公共接口已有明确不可用分支；本轮不声称这些平台可挂靠。签名、公证、安装包更新策略、第三方许可复核与干净机器验收尚待发布阶段。

同名输出默认创建新名称；覆盖现有文件没有开放为可直接执行的工具。PDF 为普通未加密文件的页面/结构操作；简单 DOCX 不承诺复杂排版保真。完整支持范围应以文件设置和发布清单为准。

## 统一动效与字体补充

新增共享曲线配置、触发点记录、页面与弹窗进出、内容高度过渡、原生窗口及玻璃材质过渡。小窗口侧栏的隐藏 Tooltip 不再拦截 Esc。相关实现与演示见[动效与字体规范](../design/UI_Motion_and_Typography.md)。

本轮新增 7 项逐帧/交互动效检查。原生代码编译和 Swift 检查通过；Mac 仍锁定，原生窗口动画的画面与交互复测未完成。

## 底栏挂靠、选区与更新（本轮）

底栏改为“永远贴在 Finder 窗口下边缘下方、与窗口左右对齐”，面板只改变高度：
气泡在下方放得下时向下展开，否则向上展开，底栏本体不移动。实现不再调整用户
的 Finder 窗口大小或位置（此前会为腾出空间移动窗口，表现为窗口“到处乱飘”）。
窗口系统阴影已关闭，阴影完全由界面层绘制，磨砂边缘不再出现黑边。

选区捕获改为可访问性优先：通过 `AXSelectedRows` / `AXSelected` 直接读取 Finder
选中项，用窗口目录中的真实项目名校验后拼接路径；AppleScript 仅作为目录与选区的
补充来源，Finder 自动化因此变成可选权限。提交任务时重新捕获一次，仍为空的旧记录
只作为兜底。底栏会显示当前将要处理的文件名。

自动更新改用 `tauri-plugin-updater`：启动后自动检查 GitHub Releases 的
`latest.json`，确认后下载、签名校验、替换应用并重启。仓库地址与公钥在
`src-tauri/src/update.rs` 中填写，步骤见 [GitHub 与自动更新](../engineering/GitHub_and_Updates.md)。

需要在真机上复验的项目：Finder 列表/图标两种视图的选中读取、不同屏幕位置的底栏
方向切换、快速连按快捷键的动效回落、文件选择器是否贴合底栏而不是抬起主窗口。

### 本轮自动化验证

- `pnpm build`（tsc + vite）通过。
- `pnpm test` Playwright 19 项通过（含新增的设置搜索用例）。
- `pnpm test:runtime` 9 项通过。
- `cargo clippy -D warnings` 与 `swift test`（5 项）通过。

## 仓库工程化、GitHub 登录与更新通道（本轮）

仓库落到 `Open-Less/fleqi`，项目团队 `@Open-Less/fleqi` 拥有 Write 权限。分支模型
固定为 `main`（正式版，只接收 `release/*` 与 `hotfix/*`）与 `beta`（开发主干），
两者都启用了规则集：必须走 PR、必须通过门禁检查、禁止强推与删除。

CI 由四个工作流组成：质量门禁（前端构建与类型检查、Playwright、核心库、PI 运行时、
Rust fmt/clippy、Swift 单测）、macOS 打包、Linux 与 Windows 构建、tag 发版。
提交与 PR 规范由 `scripts/check-commits.mjs` 统一执行，本地可用 `.githooks/commit-msg`
预检，PR 标题、区间内每个提交与目标分支都会被校验。

GitHub 登录新增令牌方式：OAuth 应用尚未注册时，界面提供「打开 GitHub 令牌页」与
令牌输入框，令牌先经 `api.github.com/user` 校验再写入系统钥匙串；注册 OAuth 应用并
填入 Client ID 后自动切换到设备码流程。自动更新填入仓库常量 `Open-Less/fleqi`、签名
公钥与通道选择（正式版读 `latest.json`，`-beta` 版本读常驻 `beta` 通道的
`latest-beta.json`），签名私钥与口令已写入仓库 Secrets，位置见
[发布与分发](../engineering/Release_and_Distribution.md)。

需要在真机上复验的项目：令牌登录后重启应用仍保持登录、真实 GitHub 账号的设备码流程
（需先注册 OAuth 应用）、从旧版本升级到新版本的完整更新流程（需先发出第一个 Release）。

### 本轮自动化验证

- `pnpm build`（tsc + vite）通过。
- `pnpm test` Playwright 19 项通过；修复了令牌输入框字体破坏「单一字体系统」约束的问题。
- `pnpm test:core`、`pnpm test:runtime`（9 项）、`swift test --package-path src-tauri/native`（5 项）通过。
- `cargo clippy --all-targets -- -D warnings` 与 rustfmt 检查通过（此前仓库内有未格式化文件，已一并格式化）。
- `node scripts/check-commits.mjs` 正例与反例、`node scripts/check-version.mjs v0.0.1` 通过。
- GitHub Actions（PR #1，全部通过）：`PR conventions`；`CI` 的五项任务——前端构建与类型检查、
  界面行为测试、核心库测试、PI 运行时测试、桌面端静态检查（rustfmt + 两份 clippy + Swift 单测）；
  `Build Linux and Windows` 的 Linux（deb/AppImage）与 Windows（NSIS）任务均产出安装包。
  首次运行暴露的问题与修法见[仓库与持续集成](../engineering/Repository_and_CI.md#五首次运行暴露并修好的问题)。
- `Build macOS`（合并进 `beta` 后触发）通过，产出 84 MB 的 debug 应用包；Windows 56 MB NSIS
  安装包与 Linux 234 MB 的 deb/AppImage 同样上传成功。修正了 `APPLE_SIGNING_IDENTITY`
  为空导致 codesign 失败的问题。
- `Release` 以 dry run（不创建 Release）验证流水线：完整引擎构建、更新包签名与清单生成见
  下一节记录。首次正式发版仍需用户决定 tag。
