# Building AI Light

Last updated: 2026-05-31

AI Light is a Tauri 2 desktop app with two Rust binaries:

- `ai-light`: the Tauri desktop app.
- `ai-light-hook`: the Claude Code hook helper bundled into the app and copied to `~/.ai_light/bin/` on startup.

## Remote Ubuntu -> Windows Mode

For the SSH workflow where Claude Code runs on Ubuntu and AI Light displays on Windows, use the hook-only guide:

- [Ubuntu Hook-Only Forwarding](UBUNTU_HOOK_ONLY.md)

## Current Packaging Status

Windows packaging is verified.

Current Windows artifacts:

- `target/release/ai-light.exe`
- `target/release/bundle/msi/AI Light_0.1.0_x64_en-US.msi`
- `target/release/bundle/nsis/AI Light_0.1.0_x64-setup.exe`

macOS GUI packaging still needs validation. Ubuntu/Linux is hook-only for remote forwarding and does not ship a GUI package.

The main config currently targets the Windows hook binary:

```json
"resources": {
  "../target/release/ai-light-hook.exe": "ai-light-hook.exe"
}
```

For macOS, the bundled hook binary should be `ai-light-hook` without the `.exe` suffix.

## Windows Build

Run from the repository root on Windows:

```powershell
$env:PATH = "C:\Users\kemp\.cargo\bin;$env:PATH"
cargo build -p ai-light-hook --release
npx @tauri-apps/cli@2.11.2 build
```

Expected artifacts:

```text
target/release/ai-light.exe
target/release/bundle/msi/AI Light_0.1.0_x64_en-US.msi
target/release/bundle/nsis/AI Light_0.1.0_x64-setup.exe
```

Smoke test:

```powershell
Start-Process -FilePath "N:\AI\ai_light\target\release\ai-light.exe" -WindowStyle Hidden
Start-Sleep -Seconds 2
$runtime = Get-Content "$env:USERPROFILE\.ai_light\runtime.json" | ConvertFrom-Json
Invoke-WebRequest -UseBasicParsing "http://127.0.0.1:$($runtime.http_port)/health" |
  Select-Object -ExpandProperty Content
```

Expected output:

```text
ok
```

## macOS Build

Build on a macOS machine or macOS CI runner. macOS packaging should not be treated as buildable from Windows.

```bash
cargo build -p ai-light-hook --release
npx @tauri-apps/cli@2.11.2 build
```

Expected app binary:

```text
target/release/ai-light
```

Expected bundle outputs commonly include:

```text
target/release/bundle/macos/
target/release/bundle/dmg/
```

macOS notes:

- Local unsigned builds may work for personal testing.
- Public distribution needs Apple signing and notarization.
- Ensure the packaged app includes `ai-light-hook` as a resource.
- Add a proper `.icns` icon before macOS packaging polish.

## Platform-Specific Resource Config

Windows GUI packaging is verified. macOS GUI packaging has a dedicated resource config:

macOS config:

```json
// src-tauri/tauri.macos.conf.json
{
  "bundle": {
    "resources": {
      "../target/release/ai-light-hook": "ai-light-hook"
    }
  }
}
```

Windows can keep:

```json
{
  "bundle": {
    "resources": {
      "../target/release/ai-light-hook.exe": "ai-light-hook.exe"
    }
  }
}
```

## Can Windows Build macOS?

Windows is suitable for building the Windows installer only.

macOS packages should be built on macOS because `.app`, `.dmg`, code signing, and notarization rely on Apple's toolchain.

macOS packaging from Windows is not a practical path.

## Recommended Release Path

Use CI runners per platform:

- Windows runner: build `ai-light-hook.exe`, then MSI/NSIS.
- macOS runner: build `ai-light-hook`, then `.app`/`.dmg`, with signing/notarization when ready.
- Ubuntu runner: optionally build/publish the hook-only `ai-light-hook` binary for remote forwarding.
