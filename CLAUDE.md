# Project

Zombori: a personal operating system. Stock Linux kernel, a hand-installed Debian 13 base (ARM64),
and Rust components that replace the userspace piece by piece. Developed on an Apple Silicon Mac;
the OS runs in QEMU. Plan, decisions and milestones: `docs/FOUNDATION.md`.

# Two machines

Code is edited on the **Mac** and runs in the **target VM**. The repo is shared into the VM over 9p
at `/mnt/zombori`, so there is no copying.

- **Mac:** editing, `cargo test` / `clippy` / `fmt` for pure logic, git and GitHub. Hardware code
  (`#[cfg(target_os = "linux")]`) does not compile here.
- **Target VM** (`vm/target.sh`, shell via `ssh -p 2223 <user>@localhost`): `cargo build`, install,
  reboot, observe. The binaries are Linux ARM64 and talk to `/dev/dri` (GPU) and `/dev/snd` (ALSA).
- **Builder VM** (`vm/builder.sh`): a stock Debian that gets the target disk as `/dev/vdb`; used for
  the hand install and for repairs (mount `vdb2`, `arch-chroot`).

Prompts tell them apart: `<mac-user>@<mac>` / `<user>@zombori` / `root@localhost`. Commands that use
`/mnt/zombori` only work inside the target VM.

# Commands

```bash
# Mac
cargo test --workspace                                  # pure logic: theme, scene, wav
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
vm/target.sh                                            # boot the OS (window + serial + ssh :2223)
vm/builder.sh                                           # workshop VM, target disk as /dev/vdb
vm/snapshot.sh save|restore|list|delete <name>          # target disk snapshots, VM must be off

# Inside the target VM
/mnt/zombori/system/dev-install.sh                      # build (release) + install + enable units
sudo systemctl start zombori-splash zombori-bootsound   # replay without rebooting
journalctl -b -u zombori-<component>                    # logs
systemd-analyze critical-chain zombori-<component>.service
```

# Verification

After any code change, on the Mac (CI runs the same on ARM64 Linux):
```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```
Hardware code can only be checked in the target VM: build with `dev-install.sh`, start the unit or
reboot, and say what was seen in the PR's testing section. Never claim a boot component works
without that.

# Architecture

```
crates/
  theme/        shared look: palette, embedded logo PNG, 5x7 font, boot timeline (theme::boot)
  splash/       boot logo on DRM/KMS. scene.rs = pure render(t); display.rs = DRM, double buffered
  bootsound/    boot jingle via ALSA. wav.rs = pure WAV parser; main.rs = playback
  greeter/      placeholder (M5: greetd client)
system/
  dev-install.sh             builds the workspace in the VM, installs zombori-<crate> + units
  <component>/*.service      systemd units, installed to /etc/systemd/system/
vm/             QEMU scripts; common.sh holds the shared machine definition
assets/         logo/ (pixel art + mockup generator), sounds/boot.wav (16-bit PCM)
docs/           FOUNDATION.md (plan), 01 hand install, 02 dev loop + splash + boot sound
```

Boot order: UEFI → GRUB → kernel → initramfs → systemd → [splash, bootsound] → getty (later greetd →
[greeter]). The splash and sound are cosmetic: nothing waits for them, and they must never be able
to stop boot.

# Code style

Match the existing code; `.claude/rules/rust.md` has the details.
- Each crate splits **pure logic** (testable on the Mac and in CI) from **hardware access**
  (`#[cfg(target_os = "linux")]`, with a non-Linux `main` that prints why it can't run).
- `main` calls `run()` and prints `<crate>: <error>` on failure, exit code 1.
- Assets are embedded with `include_bytes!`, so nothing has to exist on disk at boot. Changing an
  asset requires a rebuild.
- `//!` module docs explain what the file is for; comments explain why, in plain sentences.
- One workspace version (`[workspace.package]`); crates inherit it.

# Workflow

## Commits and PRs
- PR title = the squash commit on `main` = what release-please reads. Format
  `type(scope): lowercase summary`; types `feat`, `fix`, `refactor`, `docs`, `chore`, `revert`;
  scope is a crate (`theme`, `splash`, `bootsound`, `greeter`) or area (`system`, `vm`, `assets`,
  `docs`, `ci`, `repo`). Breaking: `!`. Full rules in `CONTRIBUTING.md`.
- Fill every section of `.github/PULL_REQUEST_TEMPLATE.md`. Only tick testing boxes for what was
  actually done.
- Branches: `type/topic`, e.g. `feat/bootsound`.
- Releases: never bump versions or edit `CHANGELOG.md` by hand; merging release-please's PR tags
  `vX.Y.Z`.

## No AI attribution
**Never add Claude or any AI attribution to commits, PRs, release notes or comments.** No
`Co-Authored-By` trailer, no "Generated with Claude Code" line, no mention of Claude in messages.
The project history carries only its human authors. `.claude/settings.json` turns off the default
attribution as well; this rule applies even where that setting does not reach.

## GitHub
- Repo: `zombori-co/zombori` (public). Ask before force-pushing, moving tags, changing repository
  settings or publishing releases.

# Gotchas

- Building on the 9p share is slow: `dev-install.sh` puts `CARGO_TARGET_DIR` on the VM's own disk.
- The workspace needs `libasound2-dev` and `pkg-config` to build on Linux (the `alsa` crate); without
  them every crate fails to build, the splash included.
- `boot.wav` must be 16-bit PCM WAV. DAWs export 32-bit float by default; convert with
  `afconvert -f WAVE -d LEI16@48000 in.wav assets/sounds/boot.wav`.
- Adding a crate: add its `Cargo.lock` entry to `extra-files` in `release-please-config.json`, or
  releases leave its locked version behind and `--locked` builds fail.
- Ctrl-A X kills QEMU like a power cut (ext4 replays its journal, the EFI FAT partition is left
  dirty). Shut down with `sudo poweroff`.
- `/etc/fstab` must use UUIDs: the target disk is `vdb` in the builder but `vda` when it boots.
- Snapshots only work with the VM off. Take one before risky changes.

# Reminders

1. Hardware code builds and runs only in the target VM; the Mac checks pure logic.
2. Boot components never block boot and fail by logging and exiting.
3. PR title in Conventional Commit form, template filled honestly.
4. No AI attribution anywhere in the history.
5. Verify with: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

When context is compressed, always preserve: list of modified files, failing test names, which
machine (Mac / target / builder) a step runs on, and current task context.
