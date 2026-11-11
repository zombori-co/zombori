---
name: new-component
description: "Scaffold a new Zombori boot component: a workspace crate with the pure/hardware split, a systemd unit, dev-install.sh wiring and release-please version tracking. Use when adding a new crate that runs as part of the system (e.g. the greeter or a new boot-time service)."
disable-model-invocation: true
argument-hint: "<crate-name>"
---

# Add a boot component

The crate name comes from `$ARGUMENTS` (lowercase, one word if possible, e.g. `greeter`). The binary
is installed as `zombori-<name>`.

## Steps

1. **Crate** `crates/<name>/`:
   - `Cargo.toml` inheriting `version`, `edition` and `license` from the workspace (copy
     `crates/bootsound/Cargo.toml`). Linux-only dependencies go under
     `[target.'cfg(target_os = "linux")'.dependencies]`; depend on `theme` if it draws or times
     anything.
   - `src/main.rs` with the two `main`s (Linux: `run()` + error print + exit 1; elsewhere: message +
     exit 1), as in `crates/bootsound/src/main.rs`.
   - A pure module for everything testable without hardware, with tests (see
     `.claude/rules/rust.md`).

2. **systemd unit** `system/<name>/zombori-<name>.service`, following `.claude/rules/system.md`:
   pick the target that brings the device it needs, never block boot, and comment the ordering.

3. **dev-install.sh**: add
   ```sh
   sudo install -m 755 "$CARGO_TARGET_DIR/release/<name>" /usr/local/bin/zombori-<name>
   sudo install -m 644 system/<name>/zombori-<name>.service /etc/systemd/system/
   ```
   and add `zombori-<name>.service` to the `systemctl enable` line.

4. **release-please**: in `release-please-config.json`, add to `extra-files`:
   ```json
   { "type": "toml", "path": "Cargo.lock", "jsonpath": "$.package[?(@.name.value==='<name>')].version" }
   ```

5. **Lock file and checks** on the Mac:
   ```bash
   cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
   ```
   This also adds the crate to `Cargo.lock`; commit it.

6. **Docs**: note the component in `CLAUDE.md` § Architecture, and in `docs/` if it needs setup in
   the VM (packages, config). If it belongs to a milestone, mention it in `docs/FOUNDATION.md`.

7. **Report** what to run in the target VM: any `apt install` for build dependencies, then
   `/mnt/zombori/system/dev-install.sh`, `sudo systemctl start zombori-<name>`, and
   `journalctl -b -u zombori-<name>`. The component is untested until that has been done.

PR title: `feat(<name>): <what it does>`.
