# GitHub 账号、团队协作与自动更新

Fleqi 的账号登录、团队协作与自动更新都落在 GitHub 上。本文档说明当前已经实现的部分、
还需要在 GitHub 网页侧补的动作，以及安全边界。

- 仓库：<https://github.com/Open-Less/fleqi>
- 团队：`@Open-Less/fleqi`（Write 权限）
- 发版流程：见[发布与分发](Release_and_Distribution.md)；仓库自动化全貌见[仓库与持续集成](Repository_and_CI.md)

## 一、当前状态

| 能力 | 状态 | 说明 |
| --- | --- | --- |
| 代码仓库 | 已建 | `Open-Less/fleqi`，公开；自动更新需要匿名可下载的 Release 资产 |
| 项目团队 | 已建 | `@Open-Less/fleqi`，Write 权限覆盖本仓库，仓库管理员为 maintainer |
| 自动更新仓库常量 | 已填 | `src-tauri/src/update.rs` 的 `REPOSITORY = "Open-Less/fleqi"` |
| 更新签名公钥 | 已填 | 同文件的 `PUBKEY`；私钥与口令位置见发布文档 |
| CI 签名 Secrets | 已配 | `TAURI_SIGNING_PRIVATE_KEY`、`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` |
| GitHub 登录（令牌方式） | 可用 | 用户生成只读令牌粘贴到界面即可登录，令牌只进系统钥匙串 |
| GitHub 登录（设备码方式） | 待注册 OAuth 应用 | 注册后填 Client ID 即自动切换，界面无需改动 |
| Apple 签名与公证 | 未配置 | 构建回退 ad-hoc 签名，用户首次打开需手动放行 |

## 二、GitHub 登录

界面在侧边栏底部与「设置 → 关于与更新」提供登录入口，按后端状态自动选择方式：

### 设备码方式（推荐，需要先注册 OAuth 应用）

1. GitHub → Settings → Developer settings → **OAuth Apps** → New OAuth App。
2. 名称随意（例如 `Fleqi Desktop`），Homepage URL 填仓库地址。
3. **必须勾选 Enable Device Flow**（Fleqi 用设备码，不需要回调地址，也不使用 Client Secret）。
4. 记下 Client ID，二选一填入：
   - 构建时注入：`FLEQI_GITHUB_CLIENT_ID=<id> pnpm desktop:build`；
   - 或直接写进 `src-tauri/src/github.rs` 的 `CLIENT_ID` 常量。
5. 重新构建后，`github_status` 的 `mode` 从 `token` 变为 `device`，界面显示「通过 GitHub 登录」：
   点击后在浏览器完成授权，应用轮询到授权结果就把令牌写入系统钥匙串。

### 令牌方式（当前默认，注册应用之前即可用）

界面提供一个令牌输入框与「打开 GitHub 令牌页」按钮（预填 `read:user` 权限）。
粘贴的令牌会先经过一次 `api.github.com/user` 校验，通过后写入系统钥匙串；
校验失败、被撤销或权限不足都会给出明确提示，不会静默保存。

## 三、命令与事件

| 命令 | 作用 |
| --- | --- |
| `github_status` | 返回 `{configured, mode, account}`，不含令牌 |
| `github_login_start` | 申请设备码并打开 GitHub 授权页 |
| `github_login_poll` | 轮询授权结果；成功后把令牌写入系统凭据存储 |
| `github_login_token` | 用个人访问令牌登录（先校验再保存） |
| `github_open_tokens` | 打开预填权限的令牌创建页 |
| `github_logout` | 删除系统凭据中的 GitHub 令牌 |
| `update_check` | 检查更新，返回 `{available, version, currentVersion, channel, notes, date}` |
| `update_install` | 下载并安装更新，完成后自动重启应用 |

| 事件 | 载荷 |
| --- | --- |
| `fleqi:update` | `{phase:'download'\|'installing'\|'restarting', downloaded, total}` |

`app_snapshot` 里的 `github` 与 `update` 字段分别携带账号状态与更新通道信息
（`{configured, repository, channel, currentVersion}`），界面据此渲染。

## 四、安全约定

- 令牌只写入系统凭据存储（Keychain，服务 `app.fleqi.desktop`，账号 `github`），
  不进入日志、诊断导出、任务记录与界面以外的任何地方；界面上只显示登录名与头像。
- 所有网络请求只发往固定白名单主机：`github.com`、`api.github.com`，且必须是 https，
  不接受重定向到其他主机。
- 令牌与设备码在写入状态前都会做控制字符与长度检查，避免被当作命令或路径使用。
- 更新包必须通过内置公钥做 minisign 校验，校验失败不安装；下载地址只接受 `https://github.com/...`。
- 未配置仓库或公钥时，应用不发起任何更新相关请求。

## 五、团队协作

- 团队成员通过 `@Open-Less/fleqi` 团队拿到仓库 Write 权限，无需逐个加 collaborator。
- 分支保护要求所有改动走 PR，并至少通过 `CI` 与 `PR conventions`；详见
  [CONTRIBUTING.md](../../CONTRIBUTING.md)。
- 提交与 PR 规范由 CI 强制执行，不靠口头约定。
