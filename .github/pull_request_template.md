## 做了什么

<!-- 一句话说明行为变化；如果是修 bug，先写清原来的错误表现。 -->

## 关联需求

<!-- 保留 PRD 需求 ID，例如 P0-ATT-03；没有对应 ID 就写“无”。 -->

## 验证

<!-- 贴出实际跑过的命令与结果，不要只写“已测试”。 -->

- [ ] `pnpm build`
- [ ] `pnpm test`
- [ ] `pnpm test:core`
- [ ] `pnpm test:runtime`
- [ ] `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`
- [ ] `swift test --package-path src-tauri/native`（改动原生几何或材质时）
- [ ] 真机验证（Finder 贴合、选区捕获、Quick Look 等黑盒测试覆盖不到的部分，记入 `docs/qa/P0_Acceptance.md`）

## 界面改动

<!-- 有 UI 变化时附截图或录屏，并说明浅色/黑色两套主题下的表现。 -->

## 已知限制

<!-- 明确写出没做完、没验证的部分；没有就写“无”。 -->
