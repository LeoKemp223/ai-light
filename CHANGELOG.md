# Changelog

## [0.1.0] - 2026-05-31

### Added

- Initial MVP implementation for a floating traffic-light desktop widget.
- Claude Code integration via local hooks and `ai-light-hook`.
- Codex integration via rollout JSONL session watching under `~/.codex/sessions`.
- Project-level session aggregation with idle, working, error, and done states.
- Local HTTP hook receiver with `/events` and `/health`.
- Minimal Tauri UI with traffic-light rendering, context menus, and hook install dialog.
- Stable hook binary path under `~/.ai_light/bin/`.
- Project names displayed above each light group, resolved from project metadata when available.
- Diagnostics and application log entries from the widget context menu.
- Cross-platform single-instance guard using a file lock.
- Content-based hook binary update detection.
- macOS bundle resource config for the non-`.exe` hook binary.
- Optional remote hook mode with `AI_LIGHT_URL` for sending events from SSH/Linux clients to a Windows host.
- Configurable HTTP bind address and fixed HTTP port for LAN-based monitoring.
- Ubuntu hook-only installer script for configuring Claude Code forwarding without launching a GUI.

### Known Limitations

- Windows MSI/NSIS packaging is verified via the npm Tauri CLI; a global Cargo-installed Tauri CLI is not installed.
- macOS GUI packaging config exists but still needs validation on a native macOS runner or real machine.
- Ubuntu/Linux is supported as a hook-only remote client, not as a GUI package.
