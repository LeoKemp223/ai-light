# AI Light

Desktop traffic light widget for monitoring AI coding assistants (Claude Code, Codex).

## Status

🚧 **In Development** - MVP implementation in progress

## Architecture

- **Backend:** Rust (Tauri 2.x)
- **Frontend:** Vanilla HTML/CSS/JS
- **GUI Platforms:** Windows, macOS
- **Remote Client:** Ubuntu hook-only forwarding

## Development

```bash
# Run in dev mode
cd src-tauri
cargo tauri dev

# Build
cargo tauri build

# Run tests
cargo test
```

## Documentation

- [Design Spec](docs/superpowers/specs/2026-05-30-ai-light-design.md)
- [Implementation Plan](docs/superpowers/plans/2026-05-30-ai-light-implementation.md)
- [Build & Packaging Guide](docs/BUILDING.md)
- [Ubuntu Hook-Only Forwarding](docs/UBUNTU_HOOK_ONLY.md)
