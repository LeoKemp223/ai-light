# AI Light Implementation Progress

**Last Updated:** 2026-05-31  
**Current Status:** MVP implementation compiles, passes automated tests, and produces Windows installers

## Completed Baseline

### Task 0: Pre-Implementation Validation
- Commit: `72d8fd5`
- Captured local Codex and Claude Code samples.
- Documented runtime formats and decisions in `docs/validation/findings.md`.
- Decided stable hook binary path: `~/.ai_light/bin/ai-light-hook[.exe]`.
- Decided single-instance liveness probe via `runtime.json` + `GET /health`.

### Task 1: Project Initialization
- Commit: `c1f4e07`
- Added Rust workspace with `src-tauri` Tauri app and `src-hook` hook CLI.
- Added vanilla frontend scaffold, README, icons, and ignore rules.

### Task 2: Core Data Structures
- Commit: `4898dc8`
- Added shared `Status`, `Tool`, `SessionRef`, and `LightState` types.
- Added status ordering and aggregation tests.

## Implemented In Working Tree

### Tasks 3-13: MVP Runtime
- Project detection: `src-tauri/src/project.rs`
- Config/runtime paths: `src-tauri/src/config.rs`
- State aggregation: `src-tauri/src/aggregator.rs`
- Local HTTP receiver: `src-tauri/src/http_server.rs`
- Hook CLI: `src-hook/src/main.rs`
- IPC commands: `src-tauri/src/ipc.rs`
- Hook installer: `src-tauri/src/hook_installer.rs`
- Frontend widget: `src/index.html`, `src/styles.css`, `src/app.js`
- First-run hook dialog: `src/install-hooks.html`, `src/install-hooks.css`, `src/install-hooks.js`
- Tests added for aggregator, config, project detection, HTTP parsing, hook installer, and lifecycle integration.

### Task 15: Build & Package Setup
- Enabled Tauri bundling and global Tauri API in `src-tauri/tauri.conf.json`.
- Configured bundled hook resource for the Windows hook binary at `../target/release/ai-light-hook.exe`.
- Updated Tauri identifier to `com.ai-light.desktop`.
- Added startup copy from bundled resource into the stable hook path.
- Added `CHANGELOG.md`.

## Latest Fixes

- Hook installer now appends AI Light hooks while preserving user hooks on the same Claude event.
- Reinstalling hooks now replaces only older AI Light hook entries instead of wiping user entries.
- HTTP hook event parsing accepts Claude-style aliases such as `sessionId`, `toolName`, and `tool_name`.
- Missing `session_id` falls back to `"unknown"` so malformed/minimal hook payloads do not break Claude Code.
- Startup now probes an existing `runtime.json` port with `/health` and exits if another AI Light instance is alive.
- Packaging resources now use the exact Windows hook binary path so Cargo/Tauri does not accidentally bundle the `.d` depfile.
- Moved Tauri IPC commands out of the public library crate so backend integration tests do not load the desktop WebView2 stack.
- `SessionRef` and `LightState` now serialize only; skipped `Instant` fields no longer require unsupported deserialization defaults.

## Verification

- `cargo fmt --all` passes.
- `cargo metadata --no-deps --format-version 1` passes.
- `cargo build -p ai-light-hook --release` passes and produces `target/release/ai-light-hook.exe`.
- `cargo check` passes.
- `cargo test` passes.
- `cargo build -p ai-light --release` passes.
- `npx @tauri-apps/cli@2.11.2 build` passes when `C:\Users\kemp\.cargo\bin` is added to `PATH`.
- Windows installers produced:
  - `target/release/bundle/msi/AI Light_0.1.0_x64_en-US.msi`
  - `target/release/bundle/nsis/AI Light_0.1.0_x64-setup.exe`
- Release app smoke test passes:
  - `target/release/ai-light.exe` starts successfully.
  - `~/.ai_light/runtime.json` is written with the HTTP port.
  - Bundled hook binary is copied to `~/.ai_light/bin/ai-light-hook.exe`.
  - `GET /health` returns `200 ok`.
  - Manual `POST /events` returns `200 ok`.
  - `~/.ai_light/bin/ai-light-hook.exe` successfully sends `UserPromptSubmit`, `Stop`, and `SessionEnd` events from stdin.
- Real Claude Code hook test passes:
  - `~/.claude/settings.json` was backed up to `settings.json.ai-light-test.bak`.
  - AI Light hooks were installed while preserving existing Claude settings.
  - `claude -p --verbose --include-hook-events --output-format stream-json` reported successful `SessionStart`, `UserPromptSubmit`, and `Stop` hook executions with exit code 0.
  - The Claude test prompt returned `AI_LIGHT_TEST_OK`.
  - AI Light remained healthy via `GET /health` after the Claude session.

## Remaining Work

- Install Tauri CLI globally if desired; current successful packaging used `npx @tauri-apps/cli@2.11.2 build`.
- Add cross-platform packaging configuration for non-Windows hook binary names if macOS/Linux installers are needed.
- Optional UI-level visual check: run AI Light during an interactive Claude Code session and visually confirm yellow/green transitions.
- Decide whether Codex file watching belongs in this MVP or the next iteration; validation data exists, runtime watcher is not implemented yet.
