# Fleqi P0 界面与挂靠规范

按 2026-09-14 最新反馈执行；参考图中的文字仅作展示数据。[需求基线](../product/Fleqi_PRD_v0.0.1_Final.md) · [架构](../engineering/P0_Architecture_and_Preparation.md) · [验证状态](../qa/P0_Acceptance.md)。

动效与字体以最新的[统一规范](UI_Motion_and_Typography.md)为准；入口、尺寸变化和退出动画使用共享曲线。

## 工作区与设置

主页使用 [shadcn/ui](https://ui.shadcn.com/docs) 的 `sidebar-07`，设置使用 `sidebar-13`。组件源代码位于 `src/components/ui/`，定制结构位于 `app-sidebar.tsx` 和 `settings-dialog.tsx`。

左侧为品牌、工作区导航、模型与权限入口、底部连接菜单。右侧为面包屑、概览内容和右下角 SVG 设置按钮。概览自身禁止滚动；状态卡完整显示，任务与准备清单在各自分区内滚动。最小原生窗口为 760 × 560。

设置是独立原生窗口，浏览器预览使用同样结构的 Radix Dialog。左侧首项“通用”，随后为外观、模型与账号、权限与自检、文件处理、任务与诊断、关于。右上角关闭，仅右侧内容区滚动。全局设置即时保存，模型表单明确保存，失败保留输入。

截图：[概览](../previews/dashboard-overview.png) · [设置](../previews/settings-general.png) · [黑色](../previews/settings-dark.png) · [紧凑概览](../previews/dashboard-compact.png)。截图为浏览器预览，不证明原生挂靠。

## 配色与图标

核心色为灰、黑、白，使用 `src/shadcn.css` 的 neutral tokens。深浅主题共用尺寸。用户的原始 SVG 保留在 `src/assets/icons/`（common、folder、trash 分类），由 `src/icons.tsx` 统一引用；发送、文件夹、思考、PI 和回收站已接入。设置齿轮使用内联 SVG，不使用字符表情。

输入框实际使用 `border-beam` 1.3.0，`size="md"`、`colorVariant="mono"`、`strength={0.7}`。光效仅包围输入框，不增加布局尺寸，并遵循减少动态效果设置。

## 底部栏几何

| 项目 | 原生栏 | 原始参考场景 |
| --- | --- | --- |
| 宽度 | Finder 文件窗口全宽 | 最大 1120 CSS px |
| 高度 | 56 pt | 68 px |
| 与 Finder 间距 | 8 pt | 原组合图拼接 |
| 按钮 | 36 × 36 pt | 44 × 44 px |
| 输入框 | 34 pt 高 | 43 px 高 |
| 水平内边距 | 20 pt | 26 px |
| 控件间距 | 14 pt | 18 px |
| 圆角 | 20 pt，暂沿用 | 20 px |

栏左右边缘与实际 Finder 窗口对齐。弹出斜杠菜单、问题或结果时，窗口向上扩展，输入栏底边不漂移。普通窗口空间不足时先上移 Finder，必要时缩短；仅恢复未被用户再次修改的布局。全屏采用窗口内侧底边。

拖动或缩放期间隐藏，鼠标释放且位置稳定 140 ms 后显示。仅识别 Finder 文件浏览窗口；空格打开的 Quick Look 不替换关联窗口。关闭、最小化或切换其他应用隐藏栏，任务继续独立运行。

## 材质与操作

磨砂为默认值。macOS 使用 `NSVisualEffectView`，macOS 26+ 可选 `NSGlassEffectView`；减少透明度时降级。浏览器仅提供 CSS 磨砂预览。原生外观状态以 SQLite 为准，跨窗口事件同步；旧 localStorage 偏好只导入一次。

`+` 选择文件，纸飞机提交，齿轮打开全部设置。输入 `/` 列出 `/model`、`/thinking`、`/speed`、`/settings`；方向键选择，Enter 确认，Esc 收起。模型和参数只影响新任务。未连接或服务未声明的能力不伪造可用选项。

快捷键通过实际按键录入，录入时临时停用已有全局快捷键，结束或失焦后恢复。中文输入法确认不触发提交，发送与连接操作防止连点，并显示加载、失败与保存结果。
