# Windows 图标（待生成）

P0 仅面向 macOS；Windows 资源管理器挂靠与打包属于 P1。届时执行 `pnpm tauri icon Icon/exports/Fleqi-iOS-Default-1024@1x.png` 生成 `icon.ico`，放入 `src-tauri/icons/`，并在 `src-tauri/tauri.conf.json` 的 `bundle.icon` 中登记。生成前请勿在本目录放置手工裁剪的图标，以免与生成结果不一致。
