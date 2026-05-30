# Data Format Validation Findings

Captured from local environment 2026-05-30. Samples in this directory are gitignored.

## Codex `rollout-*.jsonl`

**Location:** `~/.codex/sessions/YYYY/MM/DD/rollout-<ISO-ts>-<session-id>.jsonl`
(observed: `~/.codex/sessions/2026/05/30/rollout-2026-05-30T22-49-25-019e795c-...jsonl`)

**Top-level shape:** every line is `{timestamp, type, payload}`.

**`type` values observed:** `session_meta`, `turn_context`, `event_msg`, `response_item`.

| line type        | role                                                             |
|------------------|------------------------------------------------------------------|
| `session_meta`   | first line, has `payload.id`, `payload.cwd`, `payload.originator` |
| `turn_context`   | per turn metadata (cwd, approval_policy, sandbox_policy)         |
| `event_msg`      | runtime events (state transitions live here)                     |
| `response_item`  | model output stream (messages, reasoning, function_call, etc.)   |

**State transitions live in `event_msg.payload.type`:**

| payload.type     | meaning                                | maps to Status |
|------------------|----------------------------------------|----------------|
| `task_started`   | a new turn began                       | Working        |
| `agent_message`  | streaming assistant text (still busy)  | Working (no transition) |
| `token_count`    | rate-limit / token housekeeping        | (ignore)       |
| `task_complete`  | turn finished, has `last_agent_message`| Done           |
| `user_message`   | user input recorded                    | Idle → Working trigger when next `task_started` lands |

Errors were not observed in this short sample. VibeBar treats `event_msg.payload.type` matching `error` / `stream_error` / `turn_aborted` as Error; we'll do the same and add fallbacks defensively in code.

**Tool calls** appear as `response_item` with `payload.type == "function_call"` (and matching `function_call_output`). Used for `last_tool_call`.

## Claude Code session JSONL

**Location:** `~/.claude/projects/<encoded-cwd>/<sessionId>.jsonl`
(observed: `~/.claude/projects/n--AI-ai-light/83baf3f3-...jsonl`)

The encoded-cwd folder name is the absolute cwd with separators replaced by `-` (drive colon dropped).

**Per-line shape varies by `type`:**

| type                    | useful keys                                           |
|-------------------------|-------------------------------------------------------|
| `mode`                  | `mode`, `sessionId`                                   |
| `user`                  | `message`, `cwd`, `gitBranch`, `sessionId`, `timestamp`, `entrypoint`, `version` |
| `assistant`             | `message.content[]`, `message.stop_reason`            |
| `system`                | `level`, `subtype`, `content`                         |
| `attachment`            | attachment metadata                                   |
| `queue-operation`       | task queue ops                                        |
| `file-history-snapshot` | file snapshots Claude maintains                       |
| `last-prompt`           | trailing pointer to last prompt                       |

**Stop-reason in assistant rows is the cleanest fallback signal:**
- `stop_reason: "tool_use"` → still Working (tool call in flight)
- `stop_reason: "end_turn"` → Done
- API errors land as `assistant` rows with `isApiErrorMessage: true` → Error

But: hooks are the primary path for Claude Code, so the file watcher is best-effort fallback only.

## Decisions

- **Codex monitoring:** hooks N/A → **file watch (primary)**, process scan (last-resort fallback only).
- **Claude Code monitoring:** **hooks (primary)**, file watch (deferred — out of MVP scope, noted in plan Task 14 as future).
- **Spec adjustments needed:** none. The plan already deferred Codex file watcher implementation to a later iteration; this validation confirms the schema we'll build against.

## Hook CLI Installation Path

**Decision:** install `ai-light-hook[.exe]` to `~/.ai_light/bin/`.

**Rationale:**
- Survives app upgrades (lives outside the app install directory).
- Stable absolute path so `~/.claude/settings.json` never has to be rewritten on upgrade.
- User-writable, no admin/UAC required.

**Upgrade strategy:** the app overwrites `~/.ai_light/bin/ai-light-hook` on first launch if the embedded sidecar binary is newer than what's on disk (compare bytes / mtime). The settings.json command path stays constant.

## Single-Instance Strategy

**Decision:** single instance enforced by HTTP-server port binding.

**Implementation:**
1. On startup the Rust backend tries to bind a fresh ephemeral port.
2. Successful bind → write `~/.ai_light/runtime.json` with the port and continue.
3. Bind failure isn't the right signal here (we always bind 127.0.0.1:0), so we additionally probe the existing `runtime.json`: if a `runtime.json` exists, GET `http://127.0.0.1:<port>/health`. If it responds, another instance is live → show error dialog "AI Light is already running" and exit 0.
4. If it doesn't respond, the file is stale → overwrite and continue.

**Alternative considered:** OS file locks rejected (cross-platform pain on Windows + stale-lock recovery is painful). HTTP probe gives us a liveness signal "for free" since we already run the server.
