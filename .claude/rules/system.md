---
paths:
  - "system/**"
  - "vm/**"
---

# Boot integration and VM scripts

## systemd units (`system/<component>/zombori-<component>.service`)

- Binary installed as `/usr/local/bin/zombori-<crate>`, unit as `zombori-<crate>.service`.
- Early boot components use `DefaultDependencies=no` plus
  `Conflicts=shutdown.target` / `Before=shutdown.target`, and order themselves on what they really
  need (the splash: `local-fs.target`, udev; the sound: `sound.target`, `alsa-restore.service`).
- Start them from the target that brings their device (`WantedBy=sound.target`) or from
  `sysinit.target`. Nothing may `Require=` or wait on a cosmetic component.
- `Type=simple`; the program exits when done.
- Comments in the unit explain each ordering choice.

## dev-install.sh

- Runs inside the target VM only. Builds the whole workspace in release mode with
  `CARGO_TARGET_DIR` off the 9p share, then installs and enables each component.
- A new component adds its `install` lines and its unit to the `systemctl enable` call.

## VM scripts (`vm/`)

- Bash with `set -euo pipefail`; shared settings and the base QEMU machine live in `vm/common.sh`,
  which is sourced.
- The QEMU flags define the "hardware"; comment why each device is there.
- `vm/images/` (disks, firmware variables, downloads) is never committed.
- Don't run `vm/builder.sh` and `vm/target.sh` at the same time: both open `target.qcow2`.
