# 图标资产

本目录是 Fleqi 应用图标的唯一设计源；构建实际使用的生成图标位于 `src-tauri/icons/`。

## 目录

| 目录 | 内容 |
| --- | --- |
| `design/` | 分层设计源（`01_terminal_base` … `06_terminal_arrow` 图层 PNG）、图层总览 `layer_overview.jpg`、最终效果 `app-icon-preview.png` |
| `exports/` | Icon Composer 导出的 1024 px 外观母版：Default、Dark、ClearDark、ClearLight |
| `macos/Fleqi.icon/` | Icon Composer 工程（macOS 专属格式，含 `icon.json` 与 `Assets/`），是所有平台图标生成的源头 |
| `windows/` | Windows 图标（`.ico`）尚未生成；见目录内说明 |
| `linux/` | Linux 使用 `src-tauri/icons/` 中的 PNG 集；见目录内说明 |

## 生成链路

`design/` → `macos/Fleqi.icon`（Icon Composer）→ `exports/` 母版 → `pnpm tauri icon <母版>` → `src-tauri/icons/`。

`src-tauri/icons/` 按平台对应：

| 平台 | 文件 |
| --- | --- |
| macOS | `icon.icns`、菜单栏模板图 `tray-template.png`（由 `src-tauri/src/shell.rs` 内嵌） |
| Windows | `icon.ico`（P1 生成，尚不存在） |
| Linux / 通用 | `32x32.png`、`128x128.png`、`128x128@2x.png`、`256x256.png`、`icon.png` |

`src-tauri/icons/Fleqi.icon/` 是 Icon Composer 工程的随包资源副本（见 `src-tauri/tauri.conf.json` 的 `bundle.resources`）；修改设计后需从 `macos/Fleqi.icon/` 同步。前端 favicon 使用 `public/icon.svg` 与 `public/app-icon.png`。
