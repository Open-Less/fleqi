# GitHub 账号、团队协作与自动更新

## 当前状态

仓库为 [Open-Less/fleqi](https://github.com/Open-Less/fleqi)，开发默认分支为 `beta`，`main` 用于发布。2026-09-15 已核对所有本地与远端分支，将压缩合并的文档提交补齐历史关系，其余已合并工作分支已清理。

组织下已注册 [Fleqi Desktop](https://github.com/apps/fleqi-desktop)，启用 Device Flow，保留用户授权令牌过期设置。App 只用于识别登录账号，未申请仓库内容、组织管理或账户写入权限，未启用 Webhook。`Fleqi` 名称被 GitHub 保留，因此注册名采用 `Fleqi Desktop`。

公开 Client ID 为 `Iv23liy1Kop6o7aIC6Ns`，默认内置在 `src-tauri/src/github.rs`，开发构建可用 `FLEQI_GITHUB_CLIENT_ID` 覆盖。Client ID 是应用公开标识，不是凭据。代码不包含 Client Secret 或 App 私钥。

## 用户登录流程

1. 点击“通过 GitHub 登录”。宿主申请短期设备授权，并弹出 macOS 系统浏览器授权窗口。
2. 用户在 GitHub 输入界面显示的一次性验证码，登录账号并授权 `Open-Less` 的 `Fleqi Desktop`。组织需要 SSO 时，在 GitHub 完成组织验证。
3. 宿主按 GitHub 返回的间隔轮询，读取账号信息，写入系统钥匙串，关闭授权窗口并同步应用内的登录状态。

此路径是 GitHub 为桌面应用提供的 OAuth 设备授权流程。用户不再生成或粘贴个人访问令牌；旧的令牌输入框、令牌创建页入口及对应 Tauri 命令已移除。参见 [GitHub App 用户授权文档](https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/generating-a-user-access-token-for-a-github-app#using-the-device-flow-to-generate-a-user-access-token)。

macOS 使用 `ASWebAuthenticationSession`，登录页面不在 Fleqi 的 WebView 中执行，也拿不到 Fleqi 的 IPC 命令。网络轮询运行在 Rust 宿主中，关闭设置窗口不会中断授权；关闭系统授权窗口、点击取消或退出账号会终止对应会话。每次请求用独立会话编号识别，过期或取消后的响应不能写入凭据。

当前账号令牌到期后需重新登录；尚未实现长期令牌自动刷新。Windows/Linux 的系统凭据存储与宿主适配仍按 P0 平台边界标记为未完成。

## 命令

| 命令 | 作用 |
| --- | --- |
| `github_status` | 返回 `{configured, mode:'browser', account, flow}`，不含访问令牌或设备私有码 |
| `github_login_start({flowId})` | 创建授权会话并启动宿主后台流程 |
| `github_login_poll({flowId})` | 读取本地会话状态；不额外向 GitHub 发请求 |
| `github_login_cancel({flowId})` | 取消该会话并关闭它的授权窗口 |
| `github_logout` | 删除钥匙串中的 GitHub 凭据并终止当前会话 |
| `update_check` | 检查更新，返回版本、渠道与发布说明 |
| `update_install` | 下载、验证、安装更新并重启 |

`flow` 仅含 `id`、`status`、`userCode`、`expiresAt`、`message`。状态包括 `starting`、`waiting`、`authorized`、`cancelled`、`expired`、`failed`。账号和会话变化通过现有 `fleqi:changed` 事件同步；`app_snapshot.github` 提供同一份数据。

## 凭据与请求边界

- 访问令牌只写入系统凭据存储（Keychain 服务 `app.fleqi.desktop`，账号 `github`）；界面只取得展示字段。
- GitHub 请求固定为 `https://github.com` 和 `https://api.github.com`，禁止跳转，拒绝额外端口、userinfo 和片段；验证码页面必须是官方固定地址。
- 校验验证码、令牌格式与有效期，遵守 `slow_down` 返回的轮询间隔，不延长 GitHub 给出的授权有效期。
- 取消、退出和凭据保存使用同一份受锁保护的会话状态，旧请求不能覆盖新的登录或退出操作。
- 账号头像仅允许 GitHub 官方头像域名通过应用 CSP。

## 更新与团队

自动更新使用 [GitHub Releases](https://github.com/Open-Less/fleqi/releases)，更新包必须通过内置公钥校验；未发布更新时启动检查保持安静。仓库签名配置和发版规则见[发布与分发](Release_and_Distribution.md)，CI 与分支管理见[仓库与持续集成](Repository_and_CI.md)。团队权限通过 `@Open-Less/fleqi` 维护。
