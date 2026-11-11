# Zombori — Foundation

*Zombori* is the working name. It's easy to change later.

## Vision

A personal operating system: the Linux kernel plus a userspace that is gradually replaced by our own
Rust components. It must stay **usable at every stage**: it boots, installs normal software and gets
real work done, while every layer stays open to being swapped out and understood.

## Core decisions

| Decision | Choice | Why |
|---|---|---|
| Kernel | Linux (Debian's stock kernel) | Drivers and hardware support for free. We customize userspace, not the kernel, at least at first |
| Base | Debian 13 "trixie", **installed by hand** with `debootstrap` | Stable base, and the manual install teaches every layer. `apt` makes normal software work from day one |
| Dev architecture | ARM64 (aarch64) | Native speed on Apple Silicon through `hvf`. x86_64 comes later, when it's time to run on a real PC |
| Virtualization | QEMU, driven by scripts in `vm/` | The "hardware" is defined in version-controlled text. The same scripts will later boot images we build |
| Language | Rust (one Cargo workspace, `crates/`) | One toolchain for everything we write |
| Shipping our parts | `.deb` packages, made with `cargo-deb` (milestone 6) | Our components install, upgrade and uninstall like any other software |

## The boot pipeline: where our pieces go

```
 UEFI firmware ─► GRUB ─► Linux kernel ─► initramfs ─► systemd ─► [gate] ─► greetd ─► session
                   │                         │            │         │          │
                   │                         │            │         │          └─ [greeter] our entry screen
                   │                         │            │         └─ the spoken question (docs/03-gate.md)
                   │                         │            └─ [bootsound] after sound.target
                   │                         └─ [splash] eventually starts here, as early as possible
                   └─ GRUB theme: pixel font + logo
```

Each bracketed part is a crate in `crates/`. `theme` holds what they share: palette, logo, font,
timing; `screen` and `audio` hold the shared drawing and sound code. That keeps the splash, the
sound, the gate and the greeter feeling like one product.

## Repository layout

```
Cargo.toml        workspace
crates/
  theme/          shared look: palette, assets, constants
  screen/         shared: drawing to the GPU through DRM/KMS
  audio/          shared: WAV, playback, microphone, speech detection (ALSA)
  splash/         boot logo
  bootsound/      boot jingle
  gate/           spoken question before login, with offline speech recognition
  greeter/        login screen (a greetd client)
assets/           pixel art (logo/), audio (sounds/), fonts (fonts/)
system/           config we install: systemd units, greetd config, GRUB theme, initramfs hooks
vm/               builder.sh, target.sh, snapshot.sh, fetch-base.sh
docs/             this file and the step-by-step guides
```

## Development loop

1. Edit code on the Mac (VS Code with rust-analyzer).
2. The target VM sees the repo through a 9p share (`mount_tag=zombori`), so there's no copying.
3. Build **inside the target VM**, which needs no cross-compiling. Put `CARGO_TARGET_DIR` on the VM's
   own disk (for example `~/.cache/zombori-target`), because builds on a 9p share are slow.
4. Install, then reboot or restart the service in the VM to see the result.
5. Before risky changes, shut down and run `vm/snapshot.sh save <name>`. Roll back with `restore`.

## Milestones

Each milestone ends with something that works and that you can show.

- **M0 — Workshop** ✅ Tools installed, base image verified, repo skeleton, VM scripts.
- **M1 — Hand install** Follow `docs/01-hand-install.md`. *Done when:* the target boots on its own,
  SSH works, and the `fresh-install` snapshot exists.
- **M2 — Dev loop** 🧪 tested, see `docs/02-dev-loop-and-splash.md`. Rust in the target VM, 9p share mounted at boot, a `just`/script recipe to
  build and install the crates. *Done when:* a code change on the Mac runs in the VM in under a minute.
- **M3 — Splash v1** 🧪 tested, see `docs/02-dev-loop-and-splash.md`. `splash` opens the DRM device, sets a mode and draws the pixel logo
  (integer-scaled, nearest-neighbor) and the name centered on a solid background. They fade in smoothly
  from black with an ease-in-out curve (logo first, the name overlapping), then a glint sweeps across
  the sunglasses.
  The reference is `assets/logo/bootscreen.gif`. It starts as a normal systemd service.
  *Done when:* the boot screen plays during boot.
- **M4 — Boot sound** `bootsound` plays a short jingle through ALSA, starting once audio is ready
  and timed to the splash. *Done when:* every boot shows the logo and plays the sound together.
- **M5 — Entry screen** greetd with our `greeter`: first a terminal UI (learn the greetd protocol),
  then graphical in the same pixel style. *Done when:* you log in through our screen.
- **M6 — Seamless boot** `quiet` boot, GRUB theme, splash moved into the initramfs so it is the
  first thing on screen, and a smooth handover from splash to greeter. *Done when:* no text is
  visible from power-on to the login screen.
- **M7 — Packaging** `cargo-deb` packages plus a `zombori-desktop` metapackage and a small local apt
  repository. *Done when:* a fresh Debian becomes Zombori with `apt install zombori-desktop`.
- **M8 — Our own image** A tool (probably `mkosi`) builds a bootable Zombori disk image from those
  packages, replacing the hand install. *Done when:* `vm/target.sh` boots an image we built.

## Going deeper (later, in any order)

- Our own session and shell: launcher, panel, settings app
- Our own Wayland compositor (the `smithay` crate)
- An x86_64 build that boots on a real PC
- Our own PID 1 / service manager
- Our own package format or update system
- Kernel configuration and patches

## Tools

- **Mac:** Homebrew, Rust (rustup), QEMU, VS Code, git, `expect` (comes with macOS)
- **Pixel art:** Aseprite (paid, about $20, the standard) or LibreSprite (a free fork). Not installed yet; pick one.
- **Audio editing:** Audacity (free) for trimming and normalizing the jingle
