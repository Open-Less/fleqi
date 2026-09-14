# GitHub 账号、团队协作与自动更新

本文档说明 Fleqi 已经预留好的 GitHub 相关能力，以及你需要在 GitHub 侧准备的东西。
所有接口都已经实现在代码里，填入下面三处配置即可启用。

## 一、需要在 GitHub 侧准备

### 1. 代码仓库（自动更新用）

1. 新建一个仓库（建议私有转公开前先确认发布策略），例如 `fleqi/fleqi`。
2. 发布新版本时使用 Tag，例如 `v0.0.2`，并在该 Release 中上传：
   - `Fleqi.app.tar.gz`（应用打包后的压缩包）
   - `Fleqi.app.tar.gz.sig`（签名文件）
   - `latest.json`（更新描述，见下文第四节）

### 2. GitHub OAuth 应用（账号登录用）

1. GitHub → Settings → Developer settings → **OAuth Apps** → New OAuth App。
2. 名称随意（例如 `Fleqi Desktop`）；Homepage URL 填仓库地址。
   **必须勾选 Enable Device Flow**（Fleqi 使用设备码流程，不需要回调地址）。
3. 记下 **Client ID**（不需要 Client Secret：设备码流程不下发 secret）。

### 3. 更新签名密钥（自动更新用）

```bash
pnpm tauri signer generate -w ~/.fleqi/fleqi.key
```

- 生成 `fleqi.key`（私钥，**不要提交到仓库**）与 `fleqi.key.pub`（公钥）。
- 私钥与口令用于发布时签名；公钥填写进应用，用于校验下载到的更新包。

## 二、需要填入代码的三处配置

| 位置 | 常量 | 填写内容 |
| --- | --- | --- |
| `src-tauri/src/github.rs` | `CLIENT_ID` | 上一步 OAuth App 的 Client ID（也可用环境变量 `FLEQI_GITHUB_CLIENT_ID` 在构建时注入） |
| `src-tauri/src/update.rs` | `REPOSITORY` | `owner/repo`，例如 `fleqi/fleqi` |
| `src-tauri/src/update.rs` | `PUBKEY` | `fleqi.key.pub` 的内容（一整行） |

未填写时的表现：界面显示「GitHub 未配置」与「更新通道待配置」，不会发起任何网络请求。

## 三、已经预留好的接口

### 命令（Rust，前端通过 `backend()` 调用）

| 命令 | 作用 |
| --- | --- |
| `github_status` | 读取登录状态与账号展示信息（不返回令牌） |
| `github_login_start` | 申请设备码并打开 GitHub 授权页 |
| `github_login_poll` | 轮询授权结果；成功后把令牌写入系统凭据存储 |
| `github_logout` | 删除系统凭据中的 GitHub 令牌 |
| `update_check` | 检查更新，返回 `{available, version, notes, date}` |
| `update_install` | 下载并安装更新，完成后自动重启应用 |

### 事件

| 事件 | 载荷 |
| --- | --- |
| `fleqi:update` | `{phase:'download'\|'installing'\|'restarting', downloaded, total}` |

### 界面位置

- 侧边栏底部：GitHub 账号（未登录时是「通过 GitHub 登录」）。
- 设置 → 关于与更新：应用信息、GitHub 账号、自动更新状态、文档与反馈链接。
- 主界面启动 1.5 秒后自动检查更新；有更新时弹出对话框，确认后显示下载进度并重启。

## 四、发布一个版本的完整流程

```bash
# 1. 更新版本号（src-tauri/tauri.conf.json 与 src-tauri/Cargo.toml 保持一致）
# 2. 构建并打包
pnpm desktop:build

# 3. 生成更新包与签名
tar -czf Fleqi.app.tar.gz -C "src-tauri/target/release/bundle/macos" Fleqi.app
pnpm tauri signer sign -f ~/.fleqi/fleqi.key -p "$FLEQI_KEY_PASSWORD" Fleqi.app.tar.gz
# 产出 Fleqi.app.tar.gz.sig

# 4. 生成 latest.json
cat > latest.json <<'JSON'
{
  "version": "0.0.2",
  "notes": "本次更新内容",
  "pub_date": "2026-09-14T12:00:00Z",
  "platforms": {
    "darwin-aarch64": {
      "signature": "<Fleqi.app.tar.gz.sig 的内容>",
      "url": "https://github.com/OWNER/REPO/releases/download/v0.0.2/Fleqi.app.tar.gz"
    }
  }
}
JSON

# 5. 上传：tag v0.0.2，附加 Fleqi.app.tar.gz、.sig、latest.json
```

## 五、安全约定

- GitHub 令牌只写入系统凭据存储（Keychain，服务 `app.fleqi.desktop`，账号 `github`），
  不进入日志、诊断导出或任务记录。
- 所有网络请求只发往固定白名单主机：`github.com`、`api.github.com`，且必须是 https。
- 更新包必须通过公钥签名校验，校验失败不会安装。
- 更新下载地址只接受 `https://github.com/...`。
