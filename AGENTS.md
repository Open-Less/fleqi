# Repository Guidelines

## Project Structure & Module Organization

`src/` contains React/TypeScript surfaces: the workspace, settings dialog, and attached bar. `src/components/ui/` contains shadcn primitives. `src/assets/icons/` holds the user-provided SVG icons in three groups: `common/` (general UI glyphs), `folder/` (folder and add-folder glyphs), and `trash/` (trash glyph); reference them only through `src/icons.tsx`. `crates/fleqi-core/` owns SQLite, task records, and file execution. `src-tauri/` owns native commands, menus, and lifecycle; its `native/` Swift package handles macOS materials and Finder attachment, and its `icons/` directory holds the generated bundle icons plus the `Fleqi.icon` copy shipped as a resource. `runtime/pi/` contains the isolated PI bridge. `Icon/` is the icon design source (`design/`, `exports/`, `macos/`, `windows/`, `linux/`; see `Icon/README.md`). Documentation lives in `docs/` by category: `product/`, `engineering/`, `design/`, `qa/`, `licenses/`, `previews/` (see `docs/README.md`). `public/references/` holds the static Finder reference image used by `?surface=preview`.

## Repository Index

| Area | Entry points |
| --- | --- |
| Documentation | `docs/README.md`; PRD `docs/product/Fleqi_PRD_v0.0.1_Final.md`; architecture `docs/engineering/P0_Architecture_and_Preparation.md`; repository and CI `docs/engineering/Repository_and_CI.md`; release `docs/engineering/Release_and_Distribution.md`; updates `docs/engineering/GitHub_and_Updates.md`; UI `docs/design/P0_UI_Spec.md`; motion `docs/design/UI_Motion_and_Typography.md`; acceptance `docs/qa/P0_Acceptance.md`; contributor rules `CONTRIBUTING.md` |
| Repository automation | `.github/workflows/{ci,build-macos,build-cross-platform,release,pr}.yml`, `.github/pull_request_template.md`, `.githooks/commit-msg`, `scripts/{check-commits,check-version,write-updater-manifest}.mjs` |
| Icons and images | `src/icons.tsx`, `src/assets/icons/{common,folder,trash}/`, `Icon/README.md`, `src-tauri/icons/`, `public/icon.svg`, `public/app-icon.png`, `docs/previews/` |
| Frontend state | `src/backend.ts` (`backend()`, `useFleqi()`), `src/native.ts`, `src/appearance.ts` |
| Motion tokens | `src/motion/tokens.json` is the single source read by React, CSS, and Swift |
| Native host | `src-tauri/src/{lib,commands,state,shell,platform,runtime,runtime_package,auth,github,update,material,window_motion}.rs`, `src-tauri/native/Sources/FleqiGlass/` |
| Core | `crates/fleqi-core/src/{lib,model,db,files}.rs`, `crates/fleqi-core/src/files/{documents,engines}.rs` |
| Runtime | `runtime/pi/{entry,bridge,catalog}.mjs`, tests in `runtime/pi/test/` |
| Packaging | `scripts/{build-desktop,prepare-runtime,prepare-notices,check-package,record-motion}.mjs`, `scripts/prepare-engines.sh`, `src-tauri/tauri.conf.json`, `.cargo/config.toml` |

## API Index

### Tauri commands (`src-tauri/src/commands.rs`, `src-tauri/src/material.rs`)

Invoke from the frontend with `backend<T>(command, args)` in `src/backend.ts`; all commands are registered in `src-tauri/src/lib.rs`.

- Materials: `material_support`, `apply_material`
- Snapshot and settings: `app_snapshot`, `settings_save`, `import_appearance`
- Model profiles and auth: `profile_save`, `profile_delete`, `profile_logout`, `profile_run`, `auth_open`
- GitHub and updates: `github_status`, `github_login_start`, `github_login_poll`, `github_login_cancel`, `github_logout`, `update_check`, `update_install`
- Context and files: `context_capture`, `context_peek`, `context_clear`, `choose_files`, `choose_directory`, `result_reveal`
- Tasks: `task_submit`, `task_cancel`, `interaction_answer`, `runtime_cancel`, `history_clear`
- Permissions and diagnostics: `permissions_request`, `permissions_check`, `selftest_start`, `engines_check`, `diagnostics_export`
- Windows, shortcuts, and bar: `open_settings`, `close_settings`, `shortcut_record`, `bar_toggle`, `bar_resize`

### Core crate (`crates/fleqi-core`)

- `model.rs`: DTOs `Appearance`, `SettingsSnapshot`, `Protocol`, `AuthType`, `ServiceTier`, `ModelInfo`, `ModelProfile`, `ContextSnapshot`, `TaskRequest`, `TaskStatus`, `ActionRecord`, `TaskRecord`, `PermissionItem`, `PlatformCapabilities`; `validate()` on settings and profiles, `validate_selection()` on profiles.
- `db.rs`: `Database::{open, memory}`; `settings`, `save_settings`, `import_appearance`; `profiles`, `profile`, `save_profile`, `delete_profile`; `create_task`, `task`, `task_by_request`, `tasks`, `update_task`, `recover_interrupted`, `clear_history`; action ledger `record_action`, `finish_action`.
- `files.rs` and `files/`: `FileExecutor::{new, execute}` with `EnginePaths`, `Operation`, `FileAction::{needs_confirmation, confirmation}`, `FileResult::produced_outputs`; `files::engines::run` wraps bundled FFmpeg/qpdf; `files::documents` handles text and simple DOCX.
- `lib.rs`: `Error`, `now_ms()`.

### PI runtime bridge (`runtime/pi/`)

- `entry.mjs`: JSONL loop; inbound `start` (modes `test`, `oauth`, `api_key`, `none`), `login`, `models`, `cancel`, `selftest`, `selftest_cleanup`, `fleqi_connection_check`; outbound `text`, `message_end`, `question`, `provider_error`, `auth_event`, `error`, and `credential_read` requests answered by the host.
- `bridge.mjs`: `Bridge` (process protocol), `NativeCredentials` (host-backed credential isolation, never the user's PI directory).
- `catalog.mjs`: `discoverModels` for OpenAI Chat Completions, OpenAI Responses, Anthropic Messages, Google Gemini, and Codex OAuth, including pagination.

### Frontend (`src/backend.ts`, `src/icons.tsx`)

- `backend<T>(command, args?)` thin invoke wrapper; `useFleqi()` returns the `FleqiController` state/action hub; shared types mirror the core DTOs; `statusLabels`, `thinkingLabels`, `settingsPages`, `terminalStates` are the shared constants.
- Icons: `GlyphName`, `AssetIcon`, `TintIcon`, `AccentIcon`, `SvgImage`, `PiIcon`, `SettingsIcon`; accent colors `ACCENT`, `ACCENT_DARK`. Add a glyph by placing the SVG in the matching `src/assets/icons/` group and registering it in the `glyphs` map.

### Swift host bridge (`src-tauri/native/Sources/FleqiGlass`)

C ABI called from Rust: `fleqiMaterialSupport`, `fleqiApplyMaterial` (GlassMaterial.swift); `fleqiPreparePanel`, `fleqiFinishPanel`, `fleqiAttachPanel`, `fleqiHostCommand`, `fleqiCredential`, `fleqiStringFree` (HostBridge.swift); `fleqiWindowVisibility` (WindowMotion.swift). Attachment geometry rules live in `AttachmentGeometry.swift` and are covered by `swift test`.

## Build, Test, and Development Commands

- `pnpm install --frozen-lockfile` — install frontend dependencies; `pnpm --dir runtime/pi install --frozen-lockfile` — install runtime dependencies.
- `pnpm dev` — preview the workspace on port 1420; `?surface=preview` shows the reference scene.
- `pnpm build` — type-check and bundle the frontend.
- `pnpm test` — run Playwright with installed Chrome.
- `pnpm test:core` — validate persistence and scoped file execution.
- `pnpm test:runtime` — test PI and local protocol fixtures.
- `pnpm test:engines` — test FFmpeg/qpdf; `pnpm test:package` — verify bundled PI.
- `swift test --package-path src-tauri/native` — validate attachment geometry.
- `pnpm desktop:build --debug` — prepare resources sequentially and build the development application.
- `pnpm check:commits origin/beta..HEAD` — validate this branch's commit messages; `pnpm hooks:install` — install the local `commit-msg` hook.
- `pnpm check:version v0.0.1` — validate a release tag against the manifests; `FLEQI_SKIP_ENGINES=1 bash scripts/prepare-engines.sh` — skip the static FFmpeg/qpdf build (compile checks only, never for distribution).

Native development requires Rust and Xcode 26+. Target macOS 15+ Apple Silicon; Liquid Glass requires macOS 26+. Never prepare runtime resources concurrently with Tauri builds. Build outputs (`dist/`, `src-tauri/target/`, `.build/`, `src-tauri/gen/`, `src-tauri/runtime-resources/`, `*.dmg`) and local tool state (`.mimosa/`) are ignored by Git; regenerate them instead of committing them.

## Coding Style & Naming Conventions

Use two-space TypeScript and four-space Rust/Swift indentation. Format Rust with `cargo fmt`; run clippy on both manifests (`cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` and the same for `crates/fleqi-core/Cargo.toml`) because CI checks both and a failing pipe can hide the exit code. Reuse shadcn components, neutral colors, and `docs/design/UI_Motion_and_Typography.md` motion/font rules. Share geometry across light and black themes. Keep settings authoritative in SQLite; synchronize windows through backend events. Product documentation uses concise Simplified Chinese and lives under the matching `docs/` category. Preserve PRD requirement IDs. Keep SVG originals under `src/assets/icons/` with their Apple symbol names; keep icon design sources under `Icon/`.

## Testing Guidelines

Playwright files use `*.spec.ts` with behavior names. Cover viewport constraints, keyboard/IME interaction, settings, and theme geometry. Rust integration tests live under `crates/fleqi-core/tests/`; runtime tests use Node's test runner. Use disposable fixtures. No coverage percentage is mandated. Browser and local protocol tests do not prove Finder behavior or real account authentication; record those separately in `docs/qa/P0_Acceptance.md`.

## Branching, CI, and Release

Work happens on `beta`; `main` carries released versions and accepts only `release/*` and `hotfix/*` pull requests. Branch off `beta` as `feat/`, `fix/`, `chore/`, `docs/`, `refactor/`, `perf/`, `test/`, `ci/`, `build/`, or `style/`. The repository lives at `Open-Less/fleqi` under the `@Open-Less/fleqi` team.

CI runs on every push to `beta`/`main` and on pull requests: `ci.yml` (frontend build and type check, Playwright, core tests, PI runtime tests, Rust fmt/clippy, Swift tests), `build-macos.yml` (debug `.app` bundle artifact), `build-cross-platform.yml` (Linux deb/AppImage, Windows NSIS), and `pr.yml` (title, commits, target branch). Releases come from `v*` tags through `release.yml`, which builds the full engines, signs updater artifacts, writes `latest.json` or `latest-beta.json`, and refreshes the `beta` channel release. Never push a release tag whose version differs from both `package.json` and `src-tauri/tauri.conf.json`.

## Commit & Pull Request Guidelines

Commit messages follow `类型(范围): 主题` with types `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`, imperative subjects, no trailing period, at most 100 characters for the header. Mark breaking changes with `!` plus a `BREAKING CHANGE:` footer. The same rules are enforced by `scripts/check-commits.mjs` in CI, so validate before pushing rather than relying on review. Commits must be authored as `TRIP <1933142963@qq.com>` (the repository owner's GitHub identity): check `git log -1 --format='%an <%ae>'` after committing, because a machine-default identity silently adds a phantom contributor to GitHub. Pull requests should describe behavior, affected requirements, validation, and remaining limitations; include screenshots for UI changes under `docs/previews/`. Full rules live in `CONTRIBUTING.md`, and the automation map lives in `docs/engineering/Repository_and_CI.md`.

## Architecture & Data Boundaries

Prioritize macOS P0. Windows/Linux attachment and the P1 terminal remain unavailable. Keep credentials in system storage, file contents out of diagnostics, and user PI installations untouched. Never commit real credentials, account identifiers, absolute user paths, or unredacted diagnostics; tests must use local fixtures and placeholder keys. Model text cannot declare a file operation successful; the executor must verify its result.
