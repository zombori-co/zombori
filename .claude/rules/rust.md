---
paths:
  - "crates/**/*.rs"
---

# Rust in Zombori

## Pure logic vs hardware

Every component splits into two parts. Keep it that way when adding code.

| Part | Example | Builds on | Tested |
|---|---|---|---|
| Pure logic | `splash/src/scene.rs`, `audio/src/{wav,speech}.rs`, `screen/src/scale.rs`, `gate/src/{answer,beat,scene}.rs`, `theme` | everywhere | `cargo test` on the Mac and in CI |
| Hardware access | `screen/src/display.rs` (DRM), `audio/src/alsa_io.rs` (ALSA), `gate/src/main.rs` | Linux only | by hand in the target VM |

Shared hardware code lives in the `screen` and `audio` libraries; a new component uses those
rather than talking to DRM or ALSA itself.

- Hardware modules and Linux-only dependencies sit behind `#[cfg(target_os = "linux")]` and
  `[target.'cfg(target_os = "linux")'.dependencies]`.
- The non-Linux `main` prints `<crate> ... ; build and run it inside the VM` and exits 1.
- Pure modules used only by Linux code get
  `#[cfg_attr(not(target_os = "linux"), allow(dead_code))]` rather than being cfg'd out, so their
  tests still run on the Mac.
- Move anything decidable without hardware (parsing, layout, timing, color math) into the pure part.

## Errors

- `main` → `run()`; on `Err(e)`: `eprintln!("<crate>: {e}")` and `std::process::exit(1)`. systemd
  puts stderr in the journal.
- `run()` returns `io::Result<()>` or `Result<(), Box<dyn std::error::Error>>`; use `?`.
- `expect("...")` only for embedded assets that cannot fail unless the asset itself is broken (see
  `theme::logo()`); a test should cover that asset.
- No panics, retries forever, or waits without a timeout in boot components: a missing device means
  log and exit.

## Shared style

- Colors, sizes and timings shared between components live in `theme`; the boot timeline is
  `theme::boot`. Don't duplicate constants.
- Assets come in through `include_bytes!`, with a test that decodes them.
- `//!` doc at the top of each file saying what it is for. Comments explain why, in full sentences.
- Small, plain code: no new dependencies when a few dozen readable lines do (the WAV parser is
  hand-written on purpose). When adding one, prefer small crates with few dependencies.

## Tests

- Tests sit at the bottom of the file in `#[cfg(test)] mod tests`.
- Test behavior visible from outside: a rendered frame, a parsed file. `scene.rs` compares the final
  frame pixel for pixel with `assets/logo/bootscreen.png`; keep that test meaningful when changing
  the look (update the mockup through `assets/logo/bootscreen.py`).
