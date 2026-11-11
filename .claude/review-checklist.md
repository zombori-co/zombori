# Review checklist — zombori

Known failure modes in this project, for reviewing a PR. Conventions themselves live in `CLAUDE.md`
and `.claude/rules/`.

A tier is the **highest** weight an item can get, not an automatic verdict: there has to be a
concrete scenario in the change. Only Critical problems block a merge.

## Critical — blocks merge (boot, build, data)

- A boot component that can stop or stall boot: `Requires=`/`Before=` on something essential,
  waiting without a timeout, or a unit other units depend on
- The workspace no longer builds on ARM64 Linux, or a Linux-only dependency or module is not behind
  `cfg(target_os = "linux")`, breaking the Mac build
- A change to `vm/` scripts that can destroy or overwrite a disk image (`qemu-img create` on an
  existing file, writing to the wrong drive)
- Install instructions or scripts that write to the wrong disk or partition (`vda` vs `vdb`), or an
  `/etc/fstab` entry by device name instead of UUID
- A secret, password or private key committed

## Worth fixing — real defects

- A panic, `unwrap()` or unbounded retry in a boot component instead of log-and-exit
- Pure logic that could be tested on the Mac but sits in the Linux-only part, untested
- A new asset without a test that decodes it; a `boot.wav` that is not 16-bit PCM
- A new crate missing from `release-please-config.json` `extra-files`
- Testing boxes in the PR ticked without the VM having been used
- Docs or `CLAUDE.md` contradicting the change

## Conventions — nits, never block

- Shared colors, sizes or timings defined outside `theme`
- Missing `//!` module doc, or comments that narrate the code instead of explaining why
- Binary or unit not named `zombori-<crate>`
- PR title scope that doesn't name the touched crate or area

## Skip

- Formatting and lints (CI runs `cargo fmt --check` and `clippy -D warnings`)
- Version numbers and `CHANGELOG.md` (release-please owns them)
