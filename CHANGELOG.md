# Changelog

## [0.1.0] - 2026-05-31

### Added

- Initial MVP implementation for a floating traffic-light desktop widget.
- Claude Code integration via local hooks and `ai-light-hook`.
- Project-level session aggregation with idle, working, error, and done states.
- Local HTTP hook receiver with `/events` and `/health`.
- Minimal Tauri UI with traffic-light rendering, context menus, and hook install dialog.
- Stable hook binary path under `~/.ai_light/bin/`.

### Known Limitations

- Windows MSI/NSIS packaging is verified via the npm Tauri CLI; a global Cargo-installed Tauri CLI is not installed.
- Current bundle resource configuration targets the Windows hook binary name; macOS/Linux packaging needs platform-specific resource handling.
- Codex file watching is validated by sample format but not yet implemented in the runtime.
- Linux Wayland behavior is untested; Linux MVP should be treated as X11-first.
