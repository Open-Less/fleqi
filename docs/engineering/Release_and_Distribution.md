# 发布与分发

自动更新的链路是：本地改版本号 → 合入 `beta` → 打 tag → GitHub Actions 构建、签名、
写更新清单并发布 Release → 已安装的客户端在启动后自动检查并安装。

## 一、版本号与通道

版本号写在两处，必须一致，`scripts/check-version.mjs` 会在发版时校验：

- `package.json` 的 `version`
- `src-tauri/tauri.conf.json` 的 `version`

（`src-tauri/Cargo.toml` 的版本跟随同一个值。）

| tag | 通道 | Release 标记 | 更新清单 | 客户端读取地址 |
| --- | --- | --- | --- | --- |
| `v0.0.1` | stable | 正式版 | `latest.json` | `releases/latest/download/latest.json` |
| `v0.0.1-beta.1` | beta | prerelease | `latest-beta.json` | `releases/download/beta/latest-beta.json` |

客户端按自己的版本号选通道：版本里包含 `-beta` 就读 beta 清单，否则读正式清单
（`src-tauri/src/update.rs` 的 `channel()`）。

beta 需要一个固定下载地址，是因为 GitHub 的 `releases/latest` 只指向非 prerelease；
发版流程会维护一个常驻的 `beta` Release，只放清单文件，不要手工删除它。

## 二、发布步骤

```sh
# 1. 在 beta 上把版本号改成要发布的版本（正式版还要先合并到 main）
#    package.json 与 src-tauri/tauri.conf.json 同时改
# 2. 提交并推送
git switch beta && git pull
# 3. 打 tag 并推送，之后全部交给 CI
git tag v0.0.1 && git push origin v0.0.1            # 正式版
git tag v0.0.1-beta.1 && git push origin v0.0.1-beta.1   # 测试版
```

`release.yml` 会依次执行：校验 tag 与版本号一致 → 确认签名密钥存在 → 完整构建 FFmpeg/qpdf
静态引擎 → 打包 `.app`、DMG 与更新包（`createUpdaterArtifacts`）→ 校验 bundle 内 PI 运行时
→ 生成更新清单 → 创建 Release（beta 会标 prerelease 并刷新 beta 通道）。

发版失败时用 `workflow_dispatch` 传入同一个 tag 重跑即可，不要重新打 tag。

### 先在 dry run 里验证

不想直接发版时，手动触发 `Release` 并勾选 `dry_run`（`tag` 留空表示按当前分支的版本号）：
它完整构建、签名、生成清单，但不创建 Release，产物作为 workflow artifact 保留 7 天。
当前流水线已用这种方式验证过一次：DMG、`Fleqi_aarch64.app.tar.gz`、`.sig` 与 `latest.json`
全部产出正常（本仓库尚未配置 Developer ID 证书，签名走的是 ad-hoc）。

### tag 必须打在有工作流文件的提交上

GitHub 只运行被 tag 的那个提交里存在的工作流文件。`main` 目前还停留在初始提交
（里面没有 `.github/workflows/`），所以在把 `beta` 通过 `release/*` 合并进 `main` 之前，
**tag 要打在 `beta` 的提交上**：`git tag v0.0.1 origin/beta && git push origin v0.0.1`。
第一次正式发版建议先把 `beta` 合进 `main`，之后两条线都可用。

## 三、签名密钥

更新包用 minisign 签名，客户端用内置公钥校验；校验不过的包不会安装。

| 项目 | 位置 |
| --- | --- |
| 私钥 | `~/.fleqi/fleqi-updater.key`（**不要入库、不要放进任何同步目录**） |
| 口令 | `~/.fleqi/updater-key-password.txt` |
| 公钥（可公开） | `src-tauri/src/update.rs` 的 `PUBKEY` |
| CI 用私钥与口令 | 仓库 Secrets：`TAURI_SIGNING_PRIVATE_KEY`、`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` |

请把私钥与口令另存一份到密码管理器或离线介质：**私钥或口令丢失后无法再签出更新包，
只能让所有用户手动重装**。

轮换密钥的步骤：`pnpm tauri signer generate -w ~/.fleqi/fleqi-updater-new.key` →
更新 `PUBKEY` 常量 → 更新两个 Secret → 出一个新版本。已安装的客户端只有在升级到带新公钥的
版本之后才会接受新密钥签的包，因此轮换必须分两次发布，中间那次不要更换签名密钥。

## 四、Apple 签名与公证

未配置 Apple 证书时构建回退 ad-hoc 签名，应用能运行，但用户从浏览器下载后首次打开需要在
「系统设置 → 隐私与安全性」里放行，CI 日志里会打印 warning。

配好下面这些 Secret 后 `release.yml` 会自动签名并公证：

| Secret | 说明 |
| --- | --- |
| `APPLE_CERTIFICATE` | Developer ID Application 证书（base64 的 `.p12`） |
| `APPLE_CERTIFICATE_PASSWORD` | 导出 `.p12` 时设置的口令 |
| `APPLE_SIGNING_IDENTITY` | 例如 `Developer ID Application: Name (TEAMID)` |
| `APPLE_ID` | Apple 账号 |
| `APPLE_PASSWORD` | App 专用密码 |
| `APPLE_TEAM_ID` | 团队 ID |

## 五、更新清单长什么样

`scripts/write-updater-manifest.mjs` 由打包产物生成，不要手写：

```json
{
  "version": "0.0.1",
  "notes": "",
  "pub_date": "2026-09-14T07:40:00.000Z",
  "platforms": {
    "darwin-aarch64": {
      "signature": "<Fleqi_aarch64.app.tar.gz.sig 的内容>",
      "url": "https://github.com/Open-Less/fleqi/releases/download/v0.0.1/Fleqi_aarch64.app.tar.gz"
    }
  }
}
```

`url` 必须指向**本 tag** 的资产，不能写 `latest`：否则 beta 与正式版会互相拉到对方的包。

## 六、客户端行为

- 主界面启动 1.5 秒后自动检查一次；手动检查在「设置 → 关于与更新」。
- 有新版本时弹出对话框说明版本与更新内容；确认后显示下载进度，安装完成自动重启。
- 凭据在 Keychain、设置与任务记录在 SQLite，重启不会丢。
- 检查失败不阻塞使用，只提示一次；未配置仓库或公钥时完全不发网络请求。

## 七、回滚

GitHub Release 可以随时把某个版本标为“最新”（Latest），客户端的 `releases/latest` 随之改变；
但已经升级的用户不会自动降级。需要让用户停留在旧版时，正确做法是发布一个版本号更高的
修复版本，而不是删除 Release——删除资产会让 `latest.json` 指向 404，客户端检查更新报错。
