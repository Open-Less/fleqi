# 贡献指南

本仓库用两条分支区分正式版与开发版，用机器校验提交与 PR 规范。规则的具体执行者是
`scripts/check-commits.mjs`，CI 与本地钩子共用同一份实现。

## 一、分支模型

| 分支 | 角色 | 谁能合入 |
| --- | --- | --- |
| `main` | 正式版。用户下载的版本从这里打 tag 发布 | 只接收 `release/*`、`hotfix/*` |
| `beta` | 开发主干。所有功能、修复先合到这里 | 所有工作分支 |
| `feat/*` `fix/*` `chore/*` `docs/*` `refactor/*` `perf/*` `test/*` `ci/*` `build/*` `style/*` | 工作分支，一律从 `beta` 开出 | — |
| `release/*` `hotfix/*` | 从 `beta`（热修从 `main`）开出，用于把版本推进到 `main` | — |

日常流程：`git switch beta && git pull` → `git switch -c feat/attached-bar` → 提交 → 开 PR 指向 `beta`。

## 二、提交信息

格式：`类型(范围): 主题`，范围可省略。主题用祈使句写结果，中文或英文都可以，结尾不加句号。

```
feat(bar): 底栏贴合宿主窗口下边缘
fix(selection): 捕获空选区时回退到窗口目录
docs: 补充发版与更新通道说明
ci(release): 用 tag 版本号生成更新清单
```

- 类型：`feat` `fix` `docs` `style` `refactor` `perf` `test` `build` `ci` `chore` `revert`
- 标题不超过 100 字符，主题不超过 72 字符。
- 破坏性变更：标题加 `!` 并在正文写 `BREAKING CHANGE: 具体影响`，例如
  `feat(api)!: 任务记录不再接受字符串路径`。
- 禁止把 `fixup!`、`squash!`、`WIP` 提交合入主干。

启用本地预检（可选，一次即可）：

```sh
pnpm hooks:install                       # git config core.hooksPath .githooks
node scripts/check-commits.mjs commit --message "feat(bar): 示例"
pnpm check:commits origin/beta..HEAD     # 合入前自查这一批提交
```

## 三、PR 规范

1. 目标分支默认 `beta`；只有发版与热修复指向 `main`。
2. 标题必须符合提交信息格式——squash 合并会把标题写进主干历史，因此 CI 会直接校验标题。
3. 描述按模板填写：做了什么、关联的 PRD 需求 ID、实际跑过的验证、界面截图、已知限制。
4. 合并前必须通过的检查：`CI`（前端构建与类型检查、界面行为测试、核心库测试、PI 运行时测试、桌面端静态检查）、`Build Linux and Windows`、`PR conventions`。
   改动原生几何、材质或窗口行为时，额外需要 `pnpm desktop:build --debug` 的真机验证记录（写进 `docs/qa/P0_Acceptance.md`）。
5. 合并方式用 squash：一个 PR 一个提交，提交信息取 PR 标题。

## 四、本地检查

```sh
pnpm build                                                  # tsc --noEmit + vite build
pnpm test                                                   # Playwright，真实 Chrome
pnpm test:core                                              # 持久化与受限文件执行
pnpm test:runtime                                           # PI 与本地协议夹具
pnpm test:engines                                           # FFmpeg/qpdf（需要引擎已构建）
pnpm test:package                                           # 打包后的 .app 运行时解包
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo clippy --manifest-path crates/fleqi-core/Cargo.toml --all-targets -- -D warnings
swift test --package-path src-tauri/native
```

两个 crate 的 clippy 都要跑：CI 检查两份清单，只跑一份会漏掉另一半的 lint。
另外不要用 `… | tail` 之类的方式吞掉退出码，本机 Homebrew Rust 缺少
`cargo-clippy` 时会直接报错（见下面「本机工具链」）。

浏览器与本地协议测试不能证明 Finder 行为或真实账号授权：这类结论必须落在
`docs/qa/P0_Acceptance.md` 的真机记录里。所谓“模型说文件已处理完成”不算证据，
执行器必须自己核验结果。

### 提交身份

提交必须使用你自己的 GitHub 账号身份。身份写错时 GitHub 会把提交算到另一个账号头上，
仓库的贡献者列表里就会出现一个从没参与过的人：

```sh
git config user.name "TRIP"
git config user.email "1933142963@qq.com"   # 与 GitHub 账号绑定的邮箱
git log -1 --format="%an <%ae>"             # 每次提交后确认作者
```

本仓库的 `.git/config` 已经固定了该身份。换机器或克隆到新目录后要重新设置，
不要沿用系统或别的项目的默认身份（历史上出现过一次
`trip <trip@users.noreply.github.com>`，已通过重写历史修正）。

### 本机工具链


Homebrew 的 Rust 与未配置默认工具链的 rustup 并存时，`cargo clippy` / `cargo fmt`
会被 rustup 的 shim 拦住并报 “could not choose a version of cargo-clippy”。用
Homebrew 自带的二进制，不要改全局工具链：

```sh
/opt/homebrew/opt/rust/bin/cargo-clippy clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
/opt/homebrew/opt/rust/bin/rustfmt --edition 2024 --check src-tauri/src/*.rs
```

## 五、发布

改版本号 → 合入 `beta` → 合并到 `main` 或直接打 tag → CI 自动构建、签名、写更新清单、
发布 Release。完整步骤、通道规则与密钥管理见
[发布与分发](docs/engineering/Release_and_Distribution.md)。
