# Building AI Light

Last updated: 2026-05-31

AI Light is a Tauri 2 desktop app with two Rust binaries:

- `ai-light`: the Tauri desktop app.
- `ai-light-hook`: the Claude Code hook helper bundled into the app and copied to `~/.ai_light/bin/` on startup.

## Current Packaging Status

Windows packaging is verified.

Current Windows artifacts:

- `target/release/ai-light.exe`
- `target/release/bundle/msi/AI Light_0.1.0_x64_en-US.msi`
- `target/release/bundle/nsis/AI Light_0.1.0_x64-setup.exe`

Linux and macOS still need packaging validation. The Rust runtime is mostly platform-aware, but the bundle resource config currently targets the Windows hook binary:

```json
"resources": {
  "../target/release/ai-light-hook.exe": "ai-light-hook.exe"
}
```

For Linux and macOS, the bundled hook binary should be `ai-light-hook` without the `.exe` suffix.

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

## Linux Build

Build on a Linux machine or Linux CI runner. Do not rely on Windows for a final Linux package.

Install typical Tauri Linux dependencies on Ubuntu/Debian:

```bash
sudo apt update
sudo apt install -y \
  build-essential \
  curl \
  wget \
  file \
  libssl-dev \
  libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev
```

Then build:

```bash
cargo build -p ai-light-hook --release
npx @tauri-apps/cli@2.11.2 build
```

Expected app binary:

```text
target/release/ai-light
```

Expected bundle directories depend on the Linux bundler tools available, commonly:

```text
target/release/bundle/deb/
target/release/bundle/rpm/
target/release/bundle/appimage/
```

Linux notes:

- Linux behavior should be validated on X11 first.
- Wayland behavior for transparent, frameless, always-on-top windows is not yet verified.
- Ensure the packaged app includes `ai-light-hook` as a resource.

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

Recommended follow-up: split bundle resources by platform.

Example Linux config:

```json
// src-tauri/tauri.linux.conf.json
{
  "bundle": {
    "resources": {
      "../target/release/ai-light-hook": "ai-light-hook"
    }
  }
}
```

Example macOS config:

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

## Can Windows Build Linux or macOS?

Windows is suitable for building the Windows installer only.

Linux packages should be built on Linux because Tauri depends on Linux desktop libraries and packaging tools such as WebKitGTK, GTK, AppImage, deb, and rpm tooling.

macOS packages should be built on macOS because `.app`, `.dmg`, code signing, and notarization rely on Apple's toolchain.

WSL can be used for experimental Linux builds, but final Linux packaging should still be validated in a real Linux environment or CI runner. macOS packaging from Windows is not a practical path.

## Recommended Release Path

Use CI runners per platform:

- Windows runner: build `ai-light-hook.exe`, then MSI/NSIS.
- Ubuntu runner: build `ai-light-hook`, then Linux bundles.
- macOS runner: build `ai-light-hook`, then `.app`/`.dmg`, with signing/notarization when ready.
