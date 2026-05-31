# AI Light

AI Light 是一个桌面红绿灯小组件，用来显示 AI 编程助手的当前状态。

它会把 Claude Code 和 Codex 的会话按项目聚合成一组灯：

- 红灯：等待用户处理、权限请求、通知或异常状态。
- 黄灯：AI 正在工作。
- 绿灯：任务已完成。
- 会话结束后，对应灯组会自动消失。

<img width="1113" height="637" alt="PixPin_2026-06-01_00-18-06" src="https://github.com/user-attachments/assets/c347a110-859e-423b-912d-2248bbaa3b66" />

当前定位是一个轻量的本地辅助工具：Windows/macOS 显示 GUI，Ubuntu/Linux 只作为远程 hook 转发端使用。

## 功能概览

- 悬浮透明窗口，始终置顶。
- 按项目显示灯组，项目名称显示在灯组顶部。
- 支持 Claude Code hooks。
- 支持 Codex 本地会话文件监听。
- 支持 Windows 本机使用。
- 支持 Windows 显示 GUI、Ubuntu SSH 端转发 Claude Code 状态。
- 支持右键菜单打开 Settings、诊断信息、日志、项目路径等。
- 支持安装/卸载 Claude Code 集成。

## 平台支持

| 平台 | 支持方式 |
| --- | --- |
| Windows | GUI + Claude Code hooks + Codex watching |
| macOS | GUI 目标平台，打包仍需在 macOS 上验证 |
| Ubuntu/Linux | hook-only，不提供 GUI，只负责把 Claude Code 事件转发到 Windows/macOS |

## 安装使用

### Windows

使用最新安装包：

```text
target/release/bundle/nsis/AI Light_0.1.0_x64-setup.exe
```

安装后启动 AI Light。首次启动会复制 hook helper 到：

```text
%USERPROFILE%\.ai_light\bin\ai-light-hook.exe
```

AI Light 会在本地启动 HTTP 接收服务，并写入运行时端口：

```text
%USERPROFILE%\.ai_light\runtime.json
```

### 配置 Claude Code

右键 AI Light 小组件，打开：

```text
Settings -> Install Claude Integration
```

这会把 AI Light hooks 合并到：

```text
%USERPROFILE%\.claude\settings.json
```

写入格式使用 `command + args`，避免 Windows 路径在 bash/sh shell 中被反斜杠转义：

```json
{
  "type": "command",
  "command": "C:\\Users\\kemp\\.ai_light\\bin\\ai-light-hook.exe",
  "args": ["prompt-submit"]
}
```

安装后重启 Claude Code 或 VSCode 中的 Claude Code 会话。

可以在 Claude Code 中输入：

```text
/hooks
```

确认 AI Light hooks 已被 Claude Code 加载。

### 配置 Codex

Codex 不需要手动安装 hooks。

AI Light 会监听本机 Codex 会话文件：

```text
%USERPROFILE%\.codex\sessions
```

当 Codex 会话产生新的 rollout 事件时，小组件会自动更新对应项目的灯状态。

## 常用操作

右键 AI Light 小组件可以打开菜单：

- Settings：配置监听地址、端口，以及安装/卸载 Claude Code 集成。
- Diagnostics：查看运行时路径、hook 状态、当前灯数量等信息。
- Open Project：打开项目目录。
- Open Session Logs：打开 Claude Code session 日志目录。
- Open App Log：打开 AI Light 应用日志。
- Quit：退出应用。

## Settings 配置

AI Light 配置文件位于：

```text
%USERPROFILE%\.ai_light\config.json
```

默认只监听本机：

```json
{
  "http_bind": "127.0.0.1",
  "http_port": null
}
```

如果要让 Ubuntu SSH 端转发到 Windows，需要改为局域网监听和固定端口：

```json
{
  "http_bind": "0.0.0.0",
  "http_port": 17321
}
```

修改后需要重启 AI Light。

## Ubuntu SSH 远程使用

典型场景：

```text
Windows 运行 AI Light GUI
Ubuntu 通过 SSH 运行 Claude Code
Ubuntu hook -> Windows AI Light -> Windows 桌面显示红绿灯
```

Windows 侧先设置：

```json
{
  "http_bind": "0.0.0.0",
  "http_port": 17321
}
```

然后确认 Windows 防火墙允许 Ubuntu 访问该端口。

Ubuntu 侧在仓库目录执行：

```bash
./scripts/install-ubuntu-hook.sh http://WINDOWS_IP:17321
```

例如：

```bash
./scripts/install-ubuntu-hook.sh http://192.168.1.10:17321
```

Ubuntu 不会安装 GUI，只会安装：

```text
~/.ai_light/bin/ai-light-hook
```

并把 Claude Code hooks 写入：

```text
~/.claude/settings.json
```

更多细节见：

- [Ubuntu Hook-Only Forwarding](docs/UBUNTU_HOOK_ONLY.md)

## 验证

### 验证 AI Light 服务

PowerShell：

```powershell
$runtime = Get-Content "$env:USERPROFILE\.ai_light\runtime.json" | ConvertFrom-Json
Invoke-WebRequest -UseBasicParsing "http://127.0.0.1:$($runtime.http_port)/health" |
  Select-Object -ExpandProperty Content
```

期望输出：

```text
ok
```

### 手动验证 Claude hook

PowerShell：

```powershell
New-Item -ItemType Directory -Force C:\Temp\ai-light-test | Out-Null

'{"session_id":"manual-claude-test","cwd":"C:\\Temp\\ai-light-test"}' |
  & "$env:USERPROFILE\.ai_light\bin\ai-light-hook.exe" session-start

'{"session_id":"manual-claude-test"}' |
  & "$env:USERPROFILE\.ai_light\bin\ai-light-hook.exe" prompt-submit
```

小组件应该出现对应项目灯组，并变为黄灯。

### 查看 hook 日志

hook helper 会写本地执行日志：

```text
%USERPROFILE%\.ai_light\hook.log
```

查看最近日志：

```powershell
Get-Content "$env:USERPROFILE\.ai_light\hook.log" -Tail 30
```

常见结果：

- `sent ... status=200`：hook 已成功发送到 AI Light。
- `failed ...`：hook 执行了，但发送失败，需要看 `target` 和 `error`。
- `ignored: no target url`：AI Light 未运行，或 `runtime.json` 不存在。

## 卸载

Windows 应用本体可以通过系统的“应用和功能”卸载。

Claude Code 集成可以在 AI Light 中执行：

```text
Settings -> Remove Claude Integration
```

这会移除 `~/.claude/settings.json` 中的 AI Light hooks，并删除：

```text
%USERPROFILE%\.ai_light\bin\ai-light-hook.exe
```

配置、日志等用户数据位于：

```text
%USERPROFILE%\.ai_light
```

如需完全清理，可手动删除该目录。

## 开发

安装依赖后，在仓库根目录运行：

```powershell
cargo test
cargo build -p ai-light-hook --release
npx @tauri-apps/cli@2.11.2 build
```

Windows 产物：

```text
target/release/ai-light.exe
target/release/bundle/msi/AI Light_0.1.0_x64_en-US.msi
target/release/bundle/nsis/AI Light_0.1.0_x64-setup.exe
```

开发模式：

```powershell
npx @tauri-apps/cli@2.11.2 dev
```

更多打包说明见：

- [Build & Packaging Guide](docs/BUILDING.md)

## 项目结构

```text
src/
  index.html              # 主悬浮窗口
  app.js                  # 前端状态渲染和右键菜单
  styles.css              # 主窗口样式
  settings.html/js/css    # Settings 窗口
  install-hooks.*         # 首次安装 Claude hooks 引导

src-tauri/
  src/main.rs             # Tauri app 入口
  src/http_server.rs      # 本地 hook HTTP 接收服务
  src/aggregator.rs       # 会话状态聚合
  src/codex_watcher.rs    # Codex 文件监听
  src/hook_installer.rs   # Claude hooks 安装/移除
  src/ipc.rs              # Tauri IPC 命令
  icons/                  # 应用图标

src-hook/
  src/main.rs             # ai-light-hook CLI

scripts/
  install-ubuntu-hook.sh  # Ubuntu hook-only 安装脚本
```

## 文档

- [Build & Packaging Guide](docs/BUILDING.md)
- [Ubuntu Hook-Only Forwarding](docs/UBUNTU_HOOK_ONLY.md)
- [Design Spec](docs/superpowers/specs/2026-05-30-ai-light-design.md)
- [Implementation Plan](docs/superpowers/plans/2026-05-30-ai-light-implementation.md)
