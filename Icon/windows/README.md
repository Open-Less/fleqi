# Windows 图标

`icon.ico` 已生成并登记：`src-tauri/icons/icon.ico`，同时在
`src-tauri/tauri.conf.json` 的 `bundle.icon` 中列出。Windows 打包（CI 的
`build-cross-platform.yml`）需要它生成资源文件，缺失会直接构建失败。

生成方式（与 macOS 图标同一份 1024 源图，避免两个平台外观不一致）：

```sh
pnpm exec tauri icon "Icon/exports/Fleqi-iOS-Default-1024@1x.png" -o /tmp/fleqi-icons
cp /tmp/fleqi-icons/icon.ico src-tauri/icons/icon.ico
```

只取 `icon.ico`，不要整目录覆盖：`src-tauri/icons/` 里已有的 PNG 与
`icon.icns` 是按 macOS 需要单独产出的，重新生成会与当前外观不一致。
本目录只放生成说明，不放手工裁剪的图标。
