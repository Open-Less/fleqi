# Fleqi

Fleqi 在 Finder 下方提供文件任务输入栏，并通过 macOS 菜单栏常驻。当前代码包含 Tauri 2 / React 界面、Rust 文件核心、Swift 原生挂靠、独立设置窗口、内置 PI 与随包文件引擎。

主页使用 shadcn `sidebar-07`，设置使用 `sidebar-13` 的弹窗结构。核心配色为灰、黑、白，图标来自 `src/assets/icons/`（由 `src/icons.tsx` 统一引用），输入框使用单色 Border Beam。概览固定一屏，任务等分区独立滚动。

代码在 <https://github.com/Open-Less/fleqi>，开发在 `beta` 分支，`main` 只承载正式版本；提交与 PR 规范由 CI 强制校验，细则见[贡献指南](CONTRIBUTING.md)。

## 运行

```sh
pnpm install --frozen-lockfile
pnpm --dir runtime/pi install --frozen-lockfile
pnpm dev                       # http://127.0.0.1:1420
pnpm prepare:runtime           # 获取固定版本 Node，复制独立 PI
pnpm prepare:engines           # macOS 文件引擎；首次需要编译
pnpm desktop                   # 原生开发预览
pnpm desktop:build --debug     # 按顺序准备资源并生成 .app
pnpm desktop:build             # Release 构建
```

开发工具需要 Node 22.19+、pnpm、Rust 和 Xcode 26+。媒体引擎源码构建还需 CMake、Ninja、NASM 与 pkg-config。PI 依赖作为完整归档随包携带，首次启动校验后解包到应用自身 `runtime-cache/`；不写用户 PI 目录。最终应用随包提供 Node 24.21.0、PI 0.85.0、FFmpeg 8.0.1、LAME 4.0、qpdf 12.4.1 与 libjpeg-turbo 3.2.0，运行时不依赖 Homebrew 或全局 Node/PI。macOS 15 最低版本声明、干净机器运行、签名和公证仍须完成发布验收。

开发应用位于 `src-tauri/target/debug/bundle/macos/Fleqi.app`。新构建完成后退出并重新打开应用，以加载新版原生代码。

浏览器 `/` 是工作区，`?surface=preview` 是使用静态 Finder 图片的控制栏参考场景，`?surface=bar` 是实际栏布局，`?surface=settings` 是独立设置布局。浏览器预览不能执行原生文件任务或保存凭据。

## 使用流程

1. 从应用或菜单栏打开设置。通用页录入快捷键；默认 `Control+Option+Space`，默认仅手动唤起。
2. 在“权限与自检”逐项授权，并检查实际 Finder 窗口与选区。
3. 在“模型与账号”登录 Codex，或填写 API 地址和密钥。自动获取模型列表后选择模型、思考强度和服务提供的速度档位，再保存。
4. 在 Finder 选中文件并唤起底部栏。`/model`、`/thinking`、`/speed` 与 `/settings` 用于快捷调整。任务提交后固定文件对象及模型配置。
5. 必要确认、进度与产物显示在底部栏及任务页。取消停止后续动作；不确定的中断结果标记“待核对”，不会自动重跑。

设置窗口、主窗口和菜单使用同一份 SQLite 状态；macOS 凭据保存在 Keychain。底部齿轮始终打开独立设置。栏与 Finder 等宽，间隔 8 pt，移动时隐藏、放下稳定后显示；Quick Look 不作为挂靠目标。

## 动效与字体

所有界面共用一套系统字体栈和曲线动效。侧栏、弹窗、菜单、表单尺寸与底部面板均支持展开和反向收回；详见[动效与字体规范](docs/design/UI_Motion_and_Typography.md)。

## 验证

```sh
pnpm build
pnpm test
pnpm test:core
pnpm test:runtime
pnpm test:engines
pnpm test:package              # 检查最终 .app 的运行时解包与 SDK 加载
swift test --package-path src-tauri/native
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```

本机若 Homebrew Rust 与未配置的 rustup 并存，可使用 `/opt/homebrew/opt/rust/bin/cargo-clippy clippy …` 和 `/opt/homebrew/opt/rust/bin/rustfmt`，无需修改全局工具链。

真实服务授权与本地协议测试分开记录。当前自动化通过情况、未完成的原生复测和发布条件见[验收记录](docs/qa/P0_Acceptance.md)。

## 文档与代码

- [文档索引](docs/README.md)
- [贡献指南](CONTRIBUTING.md)
- [需求基线](docs/product/Fleqi_PRD_v0.0.1_Final.md)
- [架构与准备图](docs/engineering/P0_Architecture_and_Preparation.md)
- [仓库与持续集成](docs/engineering/Repository_and_CI.md)
- [发布与分发](docs/engineering/Release_and_Distribution.md)
- [GitHub 与自动更新](docs/engineering/GitHub_and_Updates.md)
- [UI 规范与截图](docs/design/P0_UI_Spec.md)
- [P0 验收记录](docs/qa/P0_Acceptance.md)
- [贡献指南与 API 索引](AGENTS.md)
- [图标资产说明](Icon/README.md)

`src/` 为界面，`src/assets/icons/` 为界面 SVG 图标，`crates/fleqi-core/` 为无窗口依赖的任务与文件核心，`src-tauri/` 为原生宿主，`runtime/pi/` 为隔离的模型运行时，`scripts/` 负责运行时与引擎打包，`Icon/` 为应用图标设计源。Windows/Linux 已有公共接口和明确的不可用分支，真实宿主挂靠与 P1 侧边终端后续实现。
