# AI Light · 设计文档

- 日期：2026-05-30
- 状态：Draft（待用户审阅后转入实现计划）

## 1. 产品定位

AI Light 是一个跨平台桌面悬浮红绿灯，监听本地 AI 编码工具的工作状态，以"红/黄/绿"三色灯实时反馈每个项目下 AI 在做什么。用户在屏幕一角即可知晓所有正在运行的 AI 会话状态：是空闲、在干活、出错需介入，还是已经完成等待回应。

### 1.1 目标用户与场景

- 同时使用 Claude Code、Codex 等 CLI 形态 AI 编程工具的开发者
- 经常在多个项目间切换，希望不必频繁切换终端窗口就能感知任务进度
- 在意桌面工具的体积与资源占用（已有 Electron 类工具普遍偏重）
- 跨平台：同时使用 Windows / macOS / Linux 的开发者

### 1.2 与现有方案的差异

调研中已知项目（VibeBar、code-buddy、cc-agent-watch、ClaudeWatch 等）几乎全部为 macOS 菜单栏应用，Windows / Linux 端基本空白。AI Light 的差异化定位：

1. 真正跨三平台
2. 桌面悬浮窗形态而非菜单栏图标，视觉上更接近"现实世界的红绿灯"
3. 选用 Tauri 而非 Electron，体积与资源占用更适合长期常驻

## 2. 范围（MVP）

### 2.1 In Scope

- Windows / macOS / Linux 三平台桌面悬浮窗
- 支持监听 Claude Code（通过 hooks，主路径）
- 支持监听 Codex（通过会话文件监听 + 进程扫描）
- 4 态红绿灯：空闲 / 开发中 / 错误（需介入）/ 完成
- 项目粒度聚合（git 根目录优先，cwd 兜底）
- 待命灯（无活跃会话时的占位 + 应用入口）
- 单击确认、右键菜单、hover tooltip
- 自动安装 Claude Code hooks（弹确认 + 备份）
- 文件监听降级路径（用户拒绝安装 hook 时）
- 窗口位置持久化、暂停/恢复监听、退出

### 2.2 Out of Scope（v2+）

- 系统通知、声音
- 会话详情 mini popup
- 一键回到对应终端窗口
- Cursor / Windsurf / Aider / Gemini CLI / OpenCode 等其他工具
- 状态切换历史、统计面板、token 使用量
- 主题切换、灯样式自定义
- Pin 项目永久驻留

## 3. 状态模型

### 3.1 4 态映射 3 盏灯

| 状态 | 视觉 | 触发条件 |
|---|---|---|
| 空闲 idle | 三灯全暗 | 无活跃会话 / 会话刚创建尚未开始 |
| 开发中 working | 黄灯脉动 | 模型在思考、工具在执行 |
| 错误/需介入 error | 红灯常亮 | 权限请求、Notification hook（卡住）、会话异常退出 |
| 完成 done | 绿灯常亮 | Stop hook 触发（模型轮次结束） |

注：MVP 不区分"等待用户输入"与"完成"，二者都映射到绿灯（done）。Notification hook 之所以归到 error，是因为它代表"模型已停下来等你做出决定（权限请求、长时间无活动）"，语义上接近"卡住需要你介入"。

状态切换次序说明：常见场景下 Stop 先于 Notification，会出现"绿灯 → 红灯"的转换（Claude 完成 → 用户长时间未回应 → Notification 触发）。这是预期行为：绿灯表示"刚完成"，红灯表示"你已忽视太久 / 需要你介入"，逐级升级。

### 3.2 项目粒度

每个"项目"对应一盏红绿灯。

- 项目标识：`git rev-parse --show-toplevel` 找到的仓库根目录绝对路径；不在 git 仓库内则使用会话的 cwd
- 灯下方标签：项目根目录的 basename（如 `n:\AI\ai_light` → `ai_light`）
- 同一项目下可能有多个会话（多个 Claude Code 终端、Claude Code + Codex 等）

### 3.3 多会话聚合

同一项目下可能并发多个会话（多个 Claude Code 终端、Claude Code + Codex 同时跑、等等）。所有会话状态用"最严重优先"聚合，作为该项目灯的显示状态：

```
error > working > done > idle
```

聚合跨工具进行：CC 在 working、同一项目的 Codex 在 error → 项目灯显示 error。tooltip 中可看到具体哪个工具处于哪个状态。

## 4. UI 与交互

### 4.1 窗口形态

- 永远置顶（always-on-top）
- 透明背景，仅红绿灯本体可见
- 可拖动到屏幕任意位置；位置写入本地配置，重启后恢复
- 不点击穿透（鼠标悬停在窗口上不会穿到下层窗口）

### 4.2 布局

- 一排红绿灯横向排列（左 → 右）
- 每盏红绿灯结构：
  - 上中下三圆灯（红 / 黄 / 绿，纵向）
  - 圆灯外有圆角矩形外壳，模拟现实红绿灯
  - 下方一个项目名标签
- 待命灯永远位于最左，作为应用级入口
- 项目灯按"首次出现时间"从左到右排列（避免顺序抖动）
- 单盏红绿灯参考尺寸：约 60 × 120 px（含标签）

### 4.3 视觉规则

- 灯亮：饱和色（红 #ef4444 / 黄 #fbbf24 / 绿 #22c55e）+ glow（CSS box-shadow 光晕）
- 灯灭：暗色（深红 / 深黄 / 深绿，opacity ≈ 0.35），保留三灯轮廓使外形稳定
- 黄灯（working）脉动：1.4 s 周期，opacity 与 scale 微幅变化
- 红灯 / 绿灯：常亮，无动画
- 状态切换瞬间灯立即变化，无淡入淡出

### 4.4 交互

| 操作 | 行为 |
|---|---|
| 单击 红灯 | 确认；该项目灯回到当前真实状态（通常是空闲） |
| 单击 绿灯 | 确认；从窗口移除该项目灯（已完成且确认） |
| 单击 黄灯 / 灰灯 | 无反应 |
| hover 任意灯 | 显示 tooltip：项目名、状态、运行时长、最近一条工具调用 |
| 右键 项目灯 | 项目菜单：打开项目目录、复制路径、查看会话日志、移除此灯 |
| 右键 待命灯 | 应用菜单：设置、暂停监听 / 重启监听、关于、退出 |
| 拖动窗口任意位置 | 移动窗口（拖动距离阈值区分单击与拖动） |

### 4.5 通知

完全不发系统通知，无声音。所有反馈纯靠视觉。

### 4.6 项目灯生命周期

- 出现：该项目的第一个会话启动（SessionStart hook 或文件监听检测到新会话文件）
- 消失：所有会话 SessionEnd 后
  - 当前为黄 / 灰 → 立即移除
  - 当前为红 / 绿 → 保留到用户点击确认
- 待命灯永远存在，不消失

## 5. 技术架构

### 5.1 技术栈

- Tauri 2.x（Rust 后端 + Web 前端）
- 前端：HTML + CSS + 轻量 JS（暂不引入框架；如复杂度上升再考虑 Svelte）
- 后端关键 crate：
  - `notify` 文件监听
  - `sysinfo` 进程扫描
  - `tokio` 异步运行时
  - `axum` 本地 HTTP 服务（接收 hook 事件）
  - `serde` / `serde_json` JSON 处理
  - `git2` 或 shell 调用 `git` 识别项目根

### 5.2 进程模型

单一 Tauri 应用进程：

- Tauri runtime（主线程）
- 前端 webview：悬浮窗 + 设置窗（按需创建）
- 后端工作任务（tokio task）：
  - HTTP 服务：监听 `127.0.0.1:<auto-port>`，接收 Claude Code hook 推送
  - 文件监听器：监视 `~/.codex/sessions/**/*.jsonl`（及降级模式下的 `~/.claude/projects/**/*.jsonl`）
  - 进程扫描器：周期（5 s）扫描本地进程，识别 `claude` / `codex` 兜底
  - 状态聚合器：合并三路输入，按"项目 + 工具"维度维护状态机；变化通过 Tauri event 推前端

### 5.3 端口分配

- HTTP 服务启动时随机选一个空闲端口
- 端口写入 `~/.ai_light/runtime.json`
- 配套 CLI `ai-light-hook` 每次执行时重读 runtime.json 拿端口；这样应用重启换端口也无需改 settings.json

### 5.4 数据流

```
Claude Code → hook 命令 → ai-light-hook → POST 127.0.0.1:<port>/events ┐
Codex → 写 jsonl → notify watcher → 解析 ──────────────────────────── ├→ 状态聚合器 → Tauri event → 前端
本地进程 → sysinfo 周期扫描 ─────────────────────────────────────── ┘                  ↑
                                                                                       │
                                                  前端 Tauri command（confirm / remove / pause）
```

### 5.5 核心数据结构（后端）

```rust
// 一盏红绿灯对应一个项目（聚合该项目下所有工具的所有会话）
struct LightState {
    project_id: String,           // git root 或 cwd 的绝对路径
    project_label: String,        // basename
    status: Status,               // 聚合后的状态：error > working > done > idle
    sessions: Vec<SessionRef>,    // 跨工具的所有活跃会话
    last_event_at: Instant,
    last_tool_call: Option<String>, // for tooltip
}

struct SessionRef {
    session_id: String,           // 工具自身的 session id
    tool: Tool,                   // ClaudeCode | Codex
    status: Status,               // 该会话独立状态（用于聚合）
    started_at: Instant,
}

enum Status { Idle, Working, Error, Done }
enum Tool { ClaudeCode, Codex }
```

聚合规则：`LightState.status` 由 `sessions` 中所有 `SessionRef.status` 按 `error > working > done > idle` 取最严重值得到。

### 5.6 前端 ↔ 后端通信

- 后端推前端：Tauri event `state-changed`，payload 为 `Vec<LightState>` 全量快照（数量小，全量比 diff 简单可靠）
- 前端推后端 Tauri command：
  - `confirm_light(project_id)` — 确认红/绿灯
  - `remove_light(project_id)` — 右键"移除此灯"
  - `pause_monitoring()` / `resume_monitoring()`
  - `open_project(project_id)`、`copy_path(project_id)`、`open_session_logs(project_id)` 等右键菜单项

### 5.7 配置与持久化

- 应用配置：`~/.ai_light/config.json`（窗口位置、暂停状态、是否已安装 hook 等）
- 运行时状态：`~/.ai_light/runtime.json`（当前 HTTP 端口号）
- 项目灯状态不持久化，每次启动从头扫描

## 6. Hook 安装与降级策略

### 6.1 首次启动流程

1. 应用启动 → 检查 `~/.claude/settings.json` 是否已包含 AI Light 的 hooks（识别标记：hook 命令路径中包含 `ai-light-hook` 可执行文件名）
2. 未安装 → 弹出对话框：

   > AI Light 需要在 ~/.claude/settings.json 中添加 hook 才能精准监听 Claude Code 状态。
   > 这些 hook 只会在你使用 Claude Code 时向本机端口 127.0.0.1:<port> 发送状态事件。
   > 现有配置会被备份到 settings.json.bak。
   > [允许] [稍后] [仅查看要写入的内容]

3. 用户选择"允许" → 备份 settings.json → JSON merge 写入 hook → 提示成功
4. 用户选择"稍后" → 应用降级到"日志监听 + 进程扫描"模式（功能正常，精度降低）
5. 用户选择"仅查看" → 弹出预览框，用户复制后自行修改

### 6.2 写入的 hook 配置

```json
{
  "hooks": {
    "SessionStart":     [{ "matcher": "", "hooks": [{ "type": "command", "command": "<path>/ai-light-hook session-start" }]}],
    "UserPromptSubmit": [{ "matcher": "", "hooks": [{ "type": "command", "command": "<path>/ai-light-hook prompt-submit" }]}],
    "Notification":     [{ "matcher": "", "hooks": [{ "type": "command", "command": "<path>/ai-light-hook notification" }]}],
    "Stop":             [{ "matcher": "", "hooks": [{ "type": "command", "command": "<path>/ai-light-hook stop" }]}],
    "SessionEnd":       [{ "matcher": "", "hooks": [{ "type": "command", "command": "<path>/ai-light-hook session-end" }]}]
  }
}
```

`<path>` 为 AI Light 安装目录中的绝对路径，避免 PATH 问题。`ai-light-hook` 是一个轻量 Rust CLI，从 stdin 读 hook payload，附加事件类型，POST 到 `runtime.json` 中记录的端口。

### 6.3 Hook → 状态映射

| Hook 事件 | 状态变化 |
|---|---|
| SessionStart | 创建项目灯（如不存在），初始 idle |
| UserPromptSubmit | working |
| Notification | error（卡住 / 权限请求） |
| Stop | done |
| SessionEnd | 移除该会话；若该项目已无其他会话且当前为黄/灰，移除项目灯 |

### 6.4 Codex 状态采集

Codex 没有 hook 系统，主路径走文件监听 + 进程扫描兜底：

1. 文件监听 `~/.codex/sessions/**/rollout-*.jsonl`：
   - 文件创建 → 等价于 SessionStart
   - 追加新行 → 解析 JSON
     - `role: user` 输入 → working
     - `error` 字段 → error
     - 最后一行为 assistant 完成（无后续追加）→ done
   - 文件停止追加 N 秒 → 维持当前状态，不主动转 idle
2. 进程扫描兜底：周期扫描 `codex` 进程；进程消失视为 SessionEnd

具体 jsonl 字段名以实际样本为准，实现前先抓取真实样本验证。

### 6.5 降级路径（用户未安装 hook 时的 Claude Code）

监视 `~/.claude/projects/**/*.jsonl`，逻辑与 Codex 类似。状态精度降低（无法区分权限请求与思考中），仅区分 working / done / idle。

### 6.6 端口失联恢复

- Hook CLI POST 失败（应用未启动或端口已变）：静默失败，不影响 Claude Code 运行
- 应用启动时若 runtime.json 端口与实际不一致 → 重写 runtime.json
- Hook CLI 每次 POST 前重读 runtime.json，无需修改 settings.json

### 6.7 安全考虑

- HTTP 服务只绑定 `127.0.0.1`，不接收外部连接
- Hook payload 校验：`source` 字段必须为 `claude-code`，过滤其他来源
- settings.json 写入前用 JSON schema 校验；写入失败回滚到 `.bak`
- 敏感信息（如 hook payload 中的 prompt 文本）不持久化，仅取摘要存到 tooltip 用的 `last_tool_call` 字段

## 7. 项目结构

```
ai_light/
├── src-tauri/                  # Rust 后端
│   ├── src/
│   │   ├── main.rs             # Tauri app 入口
│   │   ├── http_server.rs      # 接收 hook 事件
│   │   ├── codex_watcher.rs    # 文件监听
│   │   ├── claude_watcher.rs   # 降级模式文件监听
│   │   ├── process_scanner.rs  # 进程兜底
│   │   ├── aggregator.rs       # 状态聚合器
│   │   ├── project.rs          # git root 识别
│   │   ├── hook_installer.rs   # settings.json merge / 写入
│   │   ├── config.rs           # 配置和 runtime.json
│   │   └── ipc.rs              # Tauri commands
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src-hook/                   # ai-light-hook CLI
│   ├── src/main.rs             # stdin → HTTP POST
│   └── Cargo.toml
├── src/                        # 前端
│   ├── index.html              # 悬浮窗
│   ├── settings.html           # 设置窗
│   ├── styles.css              # 红绿灯视觉
│   └── app.js                  # Tauri event 订阅、点击 / 拖动
├── docs/
│   └── superpowers/specs/      # 设计与计划文档
├── tests/
└── README.md
```

## 8. 测试策略

### 8.1 后端（Rust）

- 单元测试：
  - 状态聚合器（输入事件序列 → 输出状态快照）
  - 项目识别（git root 解析、非 git 目录兜底）
  - hook payload 解析
  - settings.json merge 逻辑
- 集成测试：
  - mock HTTP 客户端模拟 hook 推送，端到端验证状态变化
  - 文件监听器用临时目录 + 真实写文件验证

### 8.2 前端

- Playwright 或 Tauri webdriver 端到端 UI 测试：模拟后端 event 推送，验证灯渲染、点击、tooltip
- 视觉回归：每个状态的截图对比（可选，MVP 不必）

### 8.3 跨平台 CI

GitHub Actions 在 windows-latest / macos-latest / ubuntu-latest 各跑一遍 `cargo test` + 前端测试。

## 9. 打包与分发

Tauri 自带打包：

- Windows：`.msi` 与 `.exe`（NSIS）
- macOS：`.dmg` 与 `.app`（无苹果开发者证书时用户首次打开需右键允许）
- Linux：`.deb`、`.rpm`、`.AppImage`

`ai-light-hook` 作为 Tauri sidecar 一起打包，安装时落到应用资源目录；写入 `settings.json` 的命令使用其绝对路径。

## 10. 风险与未知数

1. **Codex jsonl 格式**：未阅读实际样本。实现前需抓取一份真实文件验证字段名、新增行节奏、错误标记位置。可参考 VibeBar 实现作为入口。
2. **Tauri 透明置顶窗口在 Linux Wayland 的兼容性**：X11 没问题，部分 Wayland 合成器可能限制（透明、置顶、无装饰）。需要在 Linux 早期实测，若兼容性差则首版 Linux 包仅保证 X11，Wayland 列为已知限制。
3. **Windows hook 命令路径**：Claude Code 在 Windows 上执行 hook command 的 shell 处理与 Mac/Linux 不同（PowerShell vs sh），需测试 `ai-light-hook.exe` 的引用与参数解析。
4. **进程扫描精度**：仅按命令名 `claude` / `codex` 匹配易与同名进程冲突。需要进一步验证：可执行文件路径前缀、cwd 是否在用户目录、命令行参数特征等。
5. **Stop 与 done 的语义模糊**：Stop hook 触发时可能模型并未真的"完成"（用户中断、context compaction 等）。MVP 接受这种模糊，所有 Stop 都映射 done；未来可结合最近一条 assistant message 是否非空进一步细分。
6. **多用户 / 多账号**：MVP 仅支持当前用户主目录下的一个 Claude Code / Codex 安装；多账号场景暂不考虑。

## 11. 后续路线图（参考）

v2 候选项（按价值预估排序）：

- 系统通知 + 声音（可关）
- 一键聚焦对应终端窗口（按平台分别实现）
- 会话详情 mini popup
- 支持 Cursor / Windsurf / Aider / Gemini CLI
- token 用量与成本统计（参考 VibeBar）
- 主题与灯样式自定义、Pin 项目永久驻留
- 无障碍：键盘导航、对色盲友好的状态符号化（不只靠颜色）
