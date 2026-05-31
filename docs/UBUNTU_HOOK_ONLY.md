# Ubuntu Hook-Only Forwarding

Last updated: 2026-05-31

Use this mode when:

- AI Light GUI runs on Windows.
- Claude Code runs on Ubuntu over SSH.
- Ubuntu should not install or show the AI Light desktop widget.
- Ubuntu only forwards Claude Code hook events to the Windows host.

## Architecture

```text
Claude Code on Ubuntu
-> ~/.ai_light/bin/ai-light-hook
-> AI_LIGHT_URL=http://WINDOWS_IP:17321/events
-> AI Light on Windows
-> Windows desktop light changes
```

## Windows Host Setup

AI Light on Windows must listen on a fixed LAN port.

Edit:

```text
%USERPROFILE%\.ai_light\config.json
```

Recommended config:

```json
{
  "http_bind": "0.0.0.0",
  "http_port": 17321
}
```

Restart AI Light after editing the config.

Windows Firewall must allow inbound TCP traffic on the selected port, for example `17321`.

## Ubuntu Client Setup

From a checkout of this repository on Ubuntu:

```bash
./scripts/install-ubuntu-hook.sh http://WINDOWS_IP:17321
```

Example:

```bash
./scripts/install-ubuntu-hook.sh http://192.168.1.10:17321
```

The script installs:

```text
~/.ai_light/bin/ai-light-hook
```

and merges Claude Code hooks into:

```text
~/.claude/settings.json
```

It backs up the previous settings file before writing.

## Existing Hook Binary

If you already have a Linux `ai-light-hook` binary:

```bash
AI_LIGHT_HOOK_SOURCE=/path/to/ai-light-hook \
  ./scripts/install-ubuntu-hook.sh http://WINDOWS_IP:17321
```

If no binary is provided, the script tries to build it with Cargo:

```bash
cargo build -p ai-light-hook --release
```

## Verify

On Windows:

```powershell
$runtime = Get-Content "$env:USERPROFILE\.ai_light\runtime.json" | ConvertFrom-Json
Invoke-WebRequest -UseBasicParsing "http://127.0.0.1:$($runtime.http_port)/health" |
  Select-Object -ExpandProperty Content
```

Expected:

```text
ok
```

On Ubuntu:

```bash
AI_LIGHT_URL=http://WINDOWS_IP:17321 \
  ~/.ai_light/bin/ai-light-hook session-start <<'JSON'
{"session_id":"ubuntu-test","cwd":"/tmp/ubuntu-test"}
JSON
```

The Windows AI Light widget should show a project light for `/tmp/ubuntu-test`.

## Notes

- Ubuntu does not run the Tauri GUI in this mode.
- Ubuntu does not need `runtime.json`.
- `AI_LIGHT_URL` may include `/events`, but the hook also works if it is omitted.
- Use LAN mode only on trusted networks. For untrusted networks, prefer SSH tunneling.
