#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Install AI Light hook-only forwarding for Ubuntu.

Usage:
  AI_LIGHT_URL=http://WINDOWS_IP:17321 ./scripts/install-ubuntu-hook.sh
  ./scripts/install-ubuntu-hook.sh http://WINDOWS_IP:17321

Optional:
  AI_LIGHT_HOOK_SOURCE=/path/to/ai-light-hook ./scripts/install-ubuntu-hook.sh http://WINDOWS_IP:17321

This installs only ~/.ai_light/bin/ai-light-hook and configures Claude Code
hooks in ~/.claude/settings.json. It does not install or launch the AI Light GUI.
USAGE
}

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"

ai_light_url="${1:-${AI_LIGHT_URL:-}}"
if [[ -z "$ai_light_url" ]]; then
  usage >&2
  exit 2
fi

if [[ "$ai_light_url" != */events ]]; then
  ai_light_url="${ai_light_url%/}/events"
fi

install_dir="${HOME}/.ai_light/bin"
hook_dest="${install_dir}/ai-light-hook"
settings_path="${HOME}/.claude/settings.json"

hook_source="${AI_LIGHT_HOOK_SOURCE:-}"
if [[ -z "$hook_source" ]]; then
  if [[ -x "${repo_root}/target/release/ai-light-hook" ]]; then
    hook_source="${repo_root}/target/release/ai-light-hook"
  elif command -v cargo >/dev/null 2>&1; then
    (cd "$repo_root" && cargo build -p ai-light-hook --release)
    hook_source="${repo_root}/target/release/ai-light-hook"
  else
    echo "error: ai-light-hook not found and cargo is not available" >&2
    echo "set AI_LIGHT_HOOK_SOURCE=/path/to/ai-light-hook or install Rust/Cargo" >&2
    exit 1
  fi
fi

if [[ ! -f "$hook_source" ]]; then
  echo "error: hook source not found: $hook_source" >&2
  exit 1
fi

mkdir -p "$install_dir"
cp "$hook_source" "$hook_dest"
chmod 755 "$hook_dest"

mkdir -p "$(dirname "$settings_path")"

python3 - "$settings_path" "$hook_dest" "$ai_light_url" <<'PY'
import json
import shlex
import sys
from datetime import datetime
from pathlib import Path

settings_path = Path(sys.argv[1])
hook_path = sys.argv[2]
ai_light_url = sys.argv[3]

events = [
    ("SessionStart", "session-start"),
    ("UserPromptSubmit", "prompt-submit"),
    ("PreToolUse", "pre-tool-use"),
    ("PermissionRequest", "permission-request"),
    ("PostToolUse", "post-tool-use"),
    ("Notification", "notification"),
    ("Stop", "stop"),
    ("SessionEnd", "session-end"),
]

if settings_path.exists():
    backup_path = settings_path.with_suffix(
        settings_path.suffix + f".ai-light-{datetime.now().strftime('%Y%m%d%H%M%S')}.bak"
    )
    backup_path.write_bytes(settings_path.read_bytes())
    content = settings_path.read_text(encoding="utf-8")
    data = json.loads(content) if content.strip() else {}
else:
    backup_path = None
    data = {}

if not isinstance(data, dict):
    raise SystemExit("settings root must be a JSON object")

hooks = data.setdefault("hooks", {})
if not isinstance(hooks, dict):
    raise SystemExit("settings hooks field must be a JSON object")

command_prefix = f"AI_LIGHT_URL={shlex.quote(ai_light_url)} {shlex.quote(hook_path)}"

def contains_ai_light_hook(entry):
    if not isinstance(entry, dict):
        return False

    commands = entry.get("hooks", [])
    if not isinstance(commands, list):
        return False

    return any(
        "ai-light-hook" in str(command.get("command", ""))
        for command in commands
        if isinstance(command, dict)
    )

for claude_event, hook_event in events:
    entries = hooks.setdefault(claude_event, [])
    if not isinstance(entries, list):
        raise SystemExit(f"settings hooks.{claude_event} field must be an array")

    entries[:] = [entry for entry in entries if not contains_ai_light_hook(entry)]

    entries.append({
        "matcher": "",
        "hooks": [{
            "type": "command",
            "command": f"{command_prefix} {hook_event}",
        }],
    })

settings_path.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

print(f"installed hook: {hook_path}")
print(f"configured settings: {settings_path}")
print(f"AI_LIGHT_URL: {ai_light_url}")
if backup_path:
    print(f"backup: {backup_path}")
PY

echo "AI Light Ubuntu hook-only install complete."
