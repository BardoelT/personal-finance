# macOS Distribution

## Overview

This document specifies the requirements for building and distributing a Rust macOS application as a DMG downloadable from GitLab. Distribution outside the Mac App Store requires Apple code signing and notarization to satisfy macOS Gatekeeper; without both, users on modern macOS cannot open the app.

## Rust Requirements

| Requirement | Details |
| --- | --- |
| Rust toolchain | Pin the version in `rust-toolchain.toml` at the repo root so all contributors and CI use the same version |
| Targets | `aarch64-apple-darwin` and `x86_64-apple-darwin` for a universal binary |
| `clippy` | Run `cargo clippy -- -D warnings`; all warnings are treated as errors |
| `rustfmt` | Enforced via `cargo fmt --check` in CI |
| `cargo-audit` | Run in CI to catch known vulnerabilities in dependencies |

## Rust Best Practices

**`unsafe` is forbidden.** No `unsafe` blocks are permitted anywhere in this codebase. This is enforced by adding `#![forbid(unsafe_code)]` at the crate root of every crate.

**Toolchain:**
- Pin the Rust edition and toolchain version in `rust-toolchain.toml`
- Run `cargo clippy -- -D warnings` and `cargo fmt --check` as CI gates

**Dependencies:**
- Prefer pure-Rust crates to avoid `build.rs` complexity with system libraries
- For native dependencies (OpenSSL, SQLite, etc.), use vendored features: `openssl = { features = ["vendored"] }`
- Run `cargo audit` in CI to catch known CVEs in dependencies

**Release builds:** configure the release profile for size and performance:

```toml
[profile.release]
lto = true
strip = true
codegen-units = 1
```

**Universal binary:** build for both architectures and combine with `lipo`:

```bash
cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin
lipo -create -output MyApp \
  target/aarch64-apple-darwin/release/MyApp \
  target/x86_64-apple-darwin/release/MyApp
```

**Development workflow:**
- Use `cargo watch -x run` for fast iteration during development
- Use `cargo flamegraph` (wraps `dtrace`) for profiling on macOS

## UI Framework: Tauri

The application uses Tauri for its GUI layer. Tauri provides a native WebView for the frontend (HTML/CSS/JS) and Rust for the application backend, producing a small native macOS binary (~5–10 MB).

**Stack:**
- Backend: Rust — business logic, data access, state management
- Frontend: a web framework (React, Vue, or Svelte) rendered in the WebView
- IPC: Tauri commands (`#[tauri::command]`) bridge the two layers

**Sidebar:** implemented in the frontend using the chosen web framework's routing or state. Each sidebar item maps to a named route or view component. The Rust backend exposes data via Tauri commands; the frontend renders it.

**Why Tauri:**
- Access to rich charting and table libraries (Chart.js, D3, AG Grid) — important for a finance app
- Full CSS control over sidebar layout and styling
- Significantly smaller binary than Electron (~5–10 MB vs ~80–150 MB)
- Strong security model: WebView capabilities are disabled by default and must be explicitly allowlisted

**Security:** configure `tauri.conf.json` with the minimum required permissions. Scope filesystem access to the app's data directory only; do not enable arbitrary shell execution.

## Prerequisites

| Requirement | Details |
| --- | --- |
| Apple Developer Program | $99/year membership at developer.apple.com |
| Developer ID Application certificate | Specific cert type for outside-App-Store distribution; a development cert will not pass Gatekeeper |
| Apple ID with app-specific password | Used by `notarytool` for notarization submissions |
| Team ID | Found in the Apple Developer portal, required for signing and notarization |
| Xcode Command Line Tools | Provides `codesign`, `xcrun notarytool`, `xcrun stapler` |
| `create-dmg` | Install via `brew install create-dmg` |

## Code Signing

Sign the built binary or `.app` bundle before packaging. The `--options runtime` flag is mandatory — notarization will be rejected without it.

```bash
codesign --deep --force --verify --verbose \
  --sign "Developer ID Application: Your Name (TEAMID)" \
  --options runtime \
  MyApp.app
```

For a CLI binary (no `.app` bundle), omit `--deep` and point directly at the binary. Verify the signature before proceeding:

```bash
codesign --verify --verbose MyApp.app
spctl --assess --verbose MyApp.app
```

## Notarization and Stapling

Notarization submits the DMG to Apple's servers for malware scanning. The `--wait` flag blocks until Apple responds (typically 1–5 minutes). Stapling attaches the notarization ticket to the DMG so it passes Gatekeeper offline.

```bash
# Submit for notarization
xcrun notarytool submit MyApp.dmg \
  --apple-id you@example.com \
  --team-id YOURTEAMID \
  --password "@keychain:AC_PASSWORD" \
  --wait

# Staple the ticket
xcrun stapler staple MyApp.dmg
```

For CI, store credentials as environment variables and pass `--password "$NOTARIZATION_PASSWORD"` instead of a keychain reference. Apple recommends generating a dedicated app-specific password for notarization rather than using the main Apple ID password.

## DMG Creation

Create the DMG after signing but before notarization. `create-dmg` wraps `hdiutil` and produces a DMG with a background, icon layout, and Applications symlink.

```bash
brew install create-dmg
create-dmg \
  --volname "MyApp" \
  --window-pos 200 120 \
  --window-size 800 400 \
  --icon-size 100 \
  --app-drop-link 600 185 \
  MyApp.dmg \
  MyApp.app
```

For a CLI binary (no `.app`), package the binary into a simple folder and point `create-dmg` at that folder, or use `hdiutil create` directly. The DMG is what gets signed, notarized, stapled, and uploaded to GitLab.

## GitLab CI/CD

**Runner requirement:** a macOS runner is required. GitLab's free shared runners are Linux-only.

| Option | Notes |
| --- | --- |
| GitLab SaaS macOS runners | Available on Premium/Ultimate; also available on free tier with a credit card on file |
| Self-hosted runner | Install `gitlab-runner` on your own Mac; most control over environment |
| Third-party CI (Cirrus CI, GitHub Actions) | Build the DMG externally, then push the artifact back to GitLab Releases |

**Certificate handling in CI:**

Store the Developer ID certificate (`.p12`) and its password as masked CI/CD variables, then import into a temporary keychain at build time:

```bash
# Decode and import cert
echo "$CERT_BASE64" | base64 --decode > cert.p12
security create-keychain -p "$KEYCHAIN_PASSWORD" build.keychain
security import cert.p12 -k build.keychain -P "$CERT_PASSWORD" -T /usr/bin/codesign
security set-key-partition-list -S apple-tool:,apple: -k "$KEYCHAIN_PASSWORD" build.keychain
security default-keychain -s build.keychain
```

Pass notarization credentials (`APPLE_ID`, `TEAM_ID`, `NOTARIZATION_PASSWORD`) as masked variables as well. Upload the final stapled DMG as a GitLab Release artifact.

## Distribution Checklist

- [ ] Install Tauri CLI (`cargo install tauri-cli`) and Node.js
- [ ] Choose and scaffold a frontend framework (React, Vue, or Svelte) inside the Tauri project
- [ ] Configure `tauri.conf.json` allowlist with minimum required permissions
- [ ] Implement sidebar routing/state in the frontend framework
- [ ] Enroll in Apple Developer Program and obtain Developer ID Application certificate
- [ ] Export certificate as `.p12` and store securely (CI variable + local Keychain)
- [ ] Build universal binary (`aarch64-apple-darwin` + `x86_64-apple-darwin`, then `lipo`)
- [ ] Sign binary/app with `codesign --options runtime`
- [ ] Verify signature with `codesign --verify` and `spctl --assess`
- [ ] Package into DMG with `create-dmg`
- [ ] Notarize DMG with `xcrun notarytool submit --wait`
- [ ] Staple ticket with `xcrun stapler staple`
- [ ] Upload stapled DMG to GitLab Release
- [ ] Configure macOS runner in GitLab CI (SaaS, self-hosted, or third-party)
- [ ] Store all signing/notarization credentials as masked CI/CD variables
