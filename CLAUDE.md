# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

`onlyclimb-launcher` is a small Windows desktop GUI (the "Only Climb Together" launcher). It collects a license key + TikTok username, validates them against a remote license server keyed to a machine fingerprint, persists the result locally, and on success launches the actual game binary. The UI strings are in **French**.

## Commands

```bash
cargo run                 # build + run the launcher (debug)
cargo build               # debug build
cargo build --release     # optimized build (opt-level=z, LTO, stripped) -> small single exe
cargo clippy              # lint
cargo fmt                 # format
cargo check               # fast type-check without producing a binary
cargo test                # runs the api.rs JSON-parsing tests (the only tests)
```

The only tests live in `src/api.rs` and cover deserializing each documented server response shape. There is no live server, so there is no network/integration test — validate response-handling changes by extending those parsing tests.

## Platform constraint (important)

This is **Windows-only by design**. `fingerprint.rs` shells out to `wmic` and `reg query` to read the CSPRODUCT UUID, C: volume serial, and `HKLM\SOFTWARE\Microsoft\Cryptography\MachineGuid`. It will not produce a real fingerprint on Linux/macOS (every value falls back to `"UNKNOWN"`), and `wmic` itself is deprecated on recent Windows builds — keep that in mind if fingerprints ever come back wrong.

## Architecture

The whole app is the [`iced`](https://docs.rs/iced) 0.12 Elm-style `Application` in `src/main.rs`. Everything flows through that pattern:

- **State** = the `OnlyClimb` struct fields.
- **`Message` enum** = every event (input changes, `CheckLicense`, async `LicenseResult`, `LaunchGame`, `KeyPress`).
- **`update`** mutates state and returns `Command`s. Network validation is fired as an async `Command::perform` (see `OnlyClimb::check_license`) so the UI stays responsive while checking.
- **`view`** rebuilds the widget tree from state each frame: a single glassy card centered (`center_x`/`center_y`) over a gradient backdrop. The window is a **fixed, non-resizable 460×880** sized to fit the tallest state (license valid → info panel + launch button all visible), so there is intentionally **no scrollable** — if you add content, keep it within that height.
- **`main()`** builds `Settings` with the bundled fonts (`include_bytes!` from `fonts/`, loaded via `Settings.fonts`), the fixed window size, and the **runtime window icon** (`window::icon::from_file_data(include_bytes!("../assets/icon.png"), None)` — this drives the title bar, the taskbar while running, and Alt-Tab). Fonts in `fonts/` are OFL and embedded at compile time.
- **App icon** lives in `assets/` (`icon.ico` multi-size for the embedded resource, `icon.png` 256px for the runtime window icon — both generated, regenerate together if the logo changes). `build.rs` embeds `assets/icon.ico` into the `.exe` via `winresource` (a build-dependency; it auto-locates the Windows SDK `rc.exe`) so the icon also shows in Explorer, shortcuts, and the pinned taskbar. The build script degrades to a `cargo:warning` (not a hard failure) if no resource compiler is found.
- **`subscription`** maps global Enter-key presses to `Message::KeyPress` (Enter triggers license check).

Four supporting modules, each with a single responsibility:

- `src/style.rs` — the **design system** ("SUMMIT" neon-noir theme). Centralises the palette (`CYAN`/`MAGENTA`/`TEXT`/…), the two bundled fonts (`DISPLAY` = Bebas Neue, `BODY` = Chakra Petch), and one custom `iced` stylesheet per surface, each exposed as a small constructor (`style::card()`, `style::field()`, `style::activate()`, `style::launch()`, `style::pill(ok)`, …). **Restyle here, not in `view`.** Note the iced-0.12 gotcha: `container`/`button`/`text_input` `Appearance` use `border: Border { color, width, radius }` and `shadow: Shadow { color, offset, blur_radius }` (not flat `border_radius`/`border_width` fields), and gradients are `Background::Gradient(Gradient::Linear(...))`.

- `src/api.rs` — the **license server contract**. Sends `GET https://panel.mystreamgame.com/api-license.php?license_key=&fingerprint=&tiktok_username=` (URL hardcoded) and parses JSON into `LicenseResponse`. Header fields are always present: `status` ("valid"|"invalid"|"error"), `valid` (bool), `reason` (stable code: `valid`, `revoked`, `expired`, `inactive`, `fingerprint_mismatch`, `not_found`, ...), and `message` (ready-to-display French). The optional `license` block (struct `License`) carries detail (status, dates in UTC ISO 8601, `expires_in_days`, `fingerprint_bound`/`fingerprint_match`) and is present **even when the key is refused** so the UI can explain why — it is absent only for unknown keys. **`valid` (bool) is authoritative** for unlocking the launch button (not `status`). Every field is `#[serde(default)]` and unknown fields are ignored, so the client is forward-compatible with partial/extended responses. The `#[cfg(test)]` module parses each documented payload shape (valid / revoked / fingerprint_mismatch / not_found / minimal-error) — these are the only runnable tests, since there is no live server.
- `src/fingerprint.rs` — builds the machine fingerprint: `SHA256("uuid|volserial|machineguid")` truncated to the **first 16 hex chars, uppercased**. Must stay byte-stable, since the server matches a license to this exact string.
- `src/license.rs` — local persistence. Stores a flat `key=value` file named **`license.key` next to the executable** (`license_key=`, `fingerprint=`, `tiktok_username=`). `delete_license()` exists but is currently unused.

## Key runtime conventions

- **Paths are relative to the launcher executable**, not the working directory. The saved `license.key` and the launched game both resolve via `current_exe().parent()`.
- On a valid license, `LaunchGame` spawns **`system_core.exe`** (expected to sit beside the launcher) — that is the real game/mod binary, intentionally innocuously named.
- On startup, a previously saved `license.key` is loaded into the fields but **not auto-validated**; the user must click "Activer" (or press Enter) to re-check against the server.
