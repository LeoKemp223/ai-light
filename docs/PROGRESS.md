# AI Light Implementation Progress

**Last Updated:** 2026-05-31  
**Current Status:** MVP implementation compiles, passes automated tests, produces Windows installers, and monitors Claude Code plus Codex sessions

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

## Implemented Runtime

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

- Commit `7110c49`: improved widget layout and transparent-window dragging behavior.
- Commit `f34a2b7`: polished project path display, removed Windows verbatim path prefixes, fixed hook alias parsing, reduced DOM churn with incremental light updates, and opened Claude session logs from the expected directory.
- Commit `3f2eb90`: enabled reliable dragging for the transparent frameless widget with Tauri permissions and pointer fallback.
- Commit `c40847e`: replaced the unbound standby traffic light with a compact `AI` app handle so only real sessions show project lights.
- Commit `bd172d4`: added Codex rollout session monitoring from `~/.codex/sessions/**/rollout-*.jsonl`.
- Commit `ff47d7d`: hardened the Codex watcher against historical replay, incomplete JSON lines, stale working sessions, and inactive done sessions.
- Commit `9a3d700`: removed project lights immediately when their final session closes.
- Commit `bc54aec`: moved project names above each light group, resolved labels from project metadata, and increased widget/layout padding so the bottom lamp housing is not clipped.

## Development Log - 2026-05-31

- Claude Code monitoring is installed through global hooks in `~/.claude/settings.json`, with the hook binary copied to `~/.ai_light/bin/ai-light-hook.exe`.
- Codex monitoring is implemented in `src-tauri/src/codex_watcher.rs`; it watches rollout JSONL files, maps session events into light states, tracks last tool calls, and avoids replaying old logs on startup.
- The UI now shows only real project lights plus a small standby `AI` handle. Project labels appear above each light group.
- Project labels are now derived from project metadata before falling back to the folder name:
  - `src-tauri/tauri.conf.json` `productName`
  - `package.json` `name`
  - `Cargo.toml` `[package].name`
  - `pyproject.toml` `[project].name`
  - `go.mod` module basename
- The transparent frameless Windows widget can be dragged from the control surface, including the project light area.
- Window height and light layout were adjusted after visual testing so the lower green lamp and bottom housing radius are fully visible.
- Closing or ending a session now removes the corresponding light instead of leaving a stale done/error indicator.
- Required stability features added after MVP:
  - Cross-platform file lock prevents multiple AI Light instances from running at the same time.
  - Hook binary updates now compare file contents instead of size and mtime.
  - Right-click diagnostics show key runtime paths, hook status, light count, and recent app log lines.
  - App log can be opened from the widget context menu.
  - Linux and macOS Tauri resource config files include the non-`.exe` hook binary name.
- Latest verified Windows release artifacts:
  - `target/release/ai-light.exe`
  - `target/release/bundle/msi/AI Light_0.1.0_x64_en-US.msi`
  - `target/release/bundle/nsis/AI Light_0.1.0_x64-setup.exe`

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
- Current release smoke test passes:
  - `target/release/ai-light.exe` starts successfully.
  - `GET /health` returns `ok`.
  - Starting the app twice leaves only one `ai-light.exe` process running.
  - Windows MSI and NSIS installers are produced successfully.

## Remaining Work

- Install Tauri CLI globally if desired; current successful packaging used `npx @tauri-apps/cli@2.11.2 build`.
- Validate Linux and macOS packaging on native CI runners.
- Optimize Codex watching so it does not recursively scan all session history every second.
- Decide whether to remove the frontend polling fallback now that Tauri event push is working.
- Implement or hide unfinished menu commands such as pause/resume/settings.
