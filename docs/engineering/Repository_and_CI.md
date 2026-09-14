# 仓库与持续集成

Fleqi 的代码、CI 与发布都在 GitHub 上完成，本地负责开发与真机验证。

- 仓库：<https://github.com/Open-Less/fleqi>（公开，自动更新需要匿名可下载的 Release 资产）
- 项目团队：`@Open-Less/fleqi`（Write 权限，覆盖本仓库）
- 分支：`main` 正式版、`beta` 开发主干；工作分支命名与提交规范见 [CONTRIBUTING.md](../../CONTRIBUTING.md)

## 一、工作流一览

| 工作流 | 触发 | 作用 | 产物 |
| --- | --- | --- | --- |
| `.github/workflows/ci.yml` | 推送到 `beta`/`main`、指向二者的 PR、手动 | 前端构建与类型检查、界面行为测试（Playwright + Chrome）、核心库测试、PI 运行时测试、桌面端静态检查（fmt + clippy + Swift 单测） | `fleqi-web-dist`、失败时的 Playwright 现场 |
| `.github/workflows/build-macos.yml` | 推送到 `beta`/`main`、手动 | macOS 应用打包（debug `.app`），验证 bundle 资源清单、Info.plist、Swift 链接、图标 | `fleqi-macos-debug-<sha>` |
| `.github/workflows/build-cross-platform.yml` | 推送、PR、手动 | Linux（deb/AppImage）与 Windows（NSIS）构建，确保共享代码没有引入 macOS 专属写法 | `fleqi-linux-<sha>`、`fleqi-windows-<sha>` |
| `.github/workflows/pr.yml` | PR 打开/编辑/同步 | PR 标题、区间内每个提交、目标分支规则 | — |
| `.github/workflows/release.yml` | 推 `v*` tag、手动 | 完整引擎构建、更新包签名、生成 `latest.json` / `latest-beta.json`、创建 Release、刷新 beta 通道 | DMG、`.app.tar.gz` 与 `.sig`、更新清单 |

同一分支的新推送会取消上一次未完成的 `CI` 与跨平台构建；发版不取消（避免发布到一半被打断）。

### 已验证的首次运行

`beta` 上的第一次完整运行结果：

| 工作流 | 结果 | 产物 |
| --- | --- | --- |
| `CI` | 五项任务全部通过 | `fleqi-web-dist` |
| `PR conventions` | 通过 | — |
| `Build macOS` | 通过（修正签名身份后） | `fleqi-macos-debug-<sha>`，84 MB |
| `Build Linux and Windows` | 通过 | Linux 234 MB（deb/AppImage）、Windows 56 MB（NSIS） |
| `Release`（dry run） | 通过（不创建 Release） | `fleqi-release-dry-run-v0.0.1`，196 MB：DMG、`Fleqi_aarch64.app.tar.gz` 与 `.sig`、`latest.json` |

`main` 仍是空的正式分支：第一次真正发版时用 `release/*` 把 `beta` 推过去，工作流文件随之进入默认分支。

仓库在第一天重建过一次：最初的提交里混进了另一个 GitHub 账号
（`trip <trip@users.noreply.github.com>`）的身份，重写历史后分支干净了，但 GitHub 会永久保留
已合并 PR 的 `refs/pull/*/head`（API 拒绝删除，返回 422），贡献者统计仍会把这些提交算给那个账号，
所以只能新建仓库。重建前的 PR 与 Actions 运行记录保存在私有归档仓库 `Open-Less/fleqi-archive`
里，导出快照在 `~/.fleqi/archive/fleqi-before-recreate/`。提交身份现在由
`.githooks/pre-commit` 守卫，设置见 [CONTRIBUTING.md](../../CONTRIBUTING.md)。

### 冷热路径的取舍

- `build-macos.yml` 默认**不**构建 FFmpeg/qpdf 静态引擎（`FLEQI_SKIP_ENGINES=1`）。
  这类引擎首次编译要几十分钟，push 构建承受不起；因此 push 产物只能验证“能构建、能启动”，不能分发。
  需要完整产物时手动触发该工作流并勾选 `with_engines`（引擎中间产物有缓存，第二次很快）。
- `release.yml` **始终**完整构建引擎，并且缺少 `TAURI_SIGNING_PRIVATE_KEY` 时直接失败——
  没有签名的更新包会让所有客户端更新失败，这种版本宁可不出。

## 二、仓库设置（已配置）

### Secrets

| Secret | 用途 | 现状 |
| --- | --- | --- |
| `TAURI_SIGNING_PRIVATE_KEY` | 更新包签名私钥 | 已配置 |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | 私钥口令 | 已配置 |
| `APPLE_CERTIFICATE` / `APPLE_CERTIFICATE_PASSWORD` / `APPLE_SIGNING_IDENTITY` | Developer ID 签名 | 未配置，当前回退 ad-hoc 签名 |
| `APPLE_ID` / `APPLE_PASSWORD` / `APPLE_TEAM_ID` | 公证 | 未配置 |

私钥与口令的备份位置见[发布与分发](Release_and_Distribution.md#三签名密钥)。

### 分支保护

`beta` 与 `main` 都要求：必须走 PR、必须通过 `CI` 与 `PR conventions`、禁止强推与删除。
`main` 额外限制来源分支只能是 `release/*` 或 `hotfix/*`（由 `pr.yml` 校验）。

## 三、本地命令与 CI 的对应关系

| CI 步骤 | 本地等价命令 |
| --- | --- |
| 前端构建与类型检查 | `pnpm build` |
| 界面行为测试 | `pnpm test` |
| 核心库测试 | `pnpm test:core` |
| PI 运行时测试 | `pnpm test:runtime` |
| 桌面端静态检查 | `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`、`cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`、`swift test --package-path src-tauri/native` |
| 提交与 PR 规范 | `pnpm check:commits origin/beta..HEAD`、`node scripts/check-commits.mjs pr --title "…" --base beta --head feat/x` |
| 发版版本校验 | `pnpm check:version v0.0.1` |

macOS 本地若 Homebrew Rust 与未配置的 rustup 并存，使用
`/opt/homebrew/opt/rust/bin/cargo-clippy clippy …` 与 `/opt/homebrew/opt/rust/bin/rustfmt`，
不要为了跑检查去改全局工具链。

## 四、跨平台边界

macOS 是 P0 平台：附件、Finder 贴合、原生材质、效果视图都只在 macOS 上实现。
Linux 与 Windows 的构建任务保证的是**可编译、可出安装包**，产物只能启动工作区与设置；
它们不构建 FFmpeg/qpdf 静态引擎，也不参与分发。P1 落地时的顺序是：先让
`window_motion`、原材料与附件能力拿到平台实现，再打开这两个平台的分发。

## 五、首次运行暴露并修好的问题

第一次跑门禁时仓库还没有被任何 CI 检查过，暴露出一批本地看不见的问题，都已修复：

- 部分 Rust 文件从未跑过 rustfmt，格式检查直接失败；已全量格式化。
- `crates/fleqi-core` 有三处 `collapsible_if`，`update.rs` 有一个未使用函数。
  此前只检查 `src-tauri`，现在两份清单都查——本地也只跑一份会漏。
- `crates/fleqi-core/src/` 的 `credential()` 返回值没有收敛成 `()`，Linux 与 Windows
  编译报 `E0308`，而 macOS 因为当时只跑了另一半检查没有暴露。
- Windows 解压 Node 运行时用了 PATH 里的 GNU tar：它既不认 zip（`This does not look
  like a tar archive`），又会把 `D:\` 的盘符冒号当成远程主机（`Cannot connect to D:
  resolve failed`）。现在解压走 `Expand-Archive`，打包 `pi.tar.gz` 用相对路径。
- 桌面端静态检查缺少 Tauri 构建脚本要求的 bundle 资源（这些资源由
  `prepare-runtime.mjs` 生成、不入库），放占位文件即可。
- `APPLE_SIGNING_IDENTITY` 未配置时 secrets 会展开成空字符串，Tauri 于是拿空身份去
  `codesign`（`Signing with identity ""` → `no identity found`），打包直接失败。
  release 流程现在先判断证书是否齐全：齐全才导出 Apple 变量，否则显式写 ad-hoc 身份 `-`。

本机也要注意两件事，否则会误判"本地通过"：

- `cargo clippy` 在 Homebrew Rust + 未配置默认工具链的 rustup 并存时会直接报错，
  用 `/opt/homebrew/opt/rust/bin/cargo-clippy`。
- 不要用 `… | tail` 之类的管道包住检查命令：退出码会变成管道末尾命令的退出码，
  失败会被吞掉。

## 六、GitLab 镜像（可选）


团队与仓库的主线在 GitHub。若需要在 GitLab 侧同步一份：

1. 在 GitLab 新建一个 group（例如 `fleqi`），再在其中新建项目 `fleqi`。
2. 用 GitLab 的 **Repository mirroring → Pull**（或 `git push --mirror` 的定时任务）从
   `https://github.com/Open-Less/fleqi.git` 拉取，`main`、`beta`、tag 全量同步。
3. 镜像仓库只做只读备份与浏览；CI、发布、更新清单仍只在 GitHub 执行，
   避免两处同时产出同版本号的更新包。

本仓库未包含 `.gitlab-ci.yml`：GitLab 侧不承担构建与发布，走镜像即可。
如果需要把门禁也搬到 GitLab，`ci.yml` 里的命令可以直接复用，但 macOS 任务需要
GitLab 的 macOS Runner 或自建 Runner，这与当前的免费额度方案不兼容。
