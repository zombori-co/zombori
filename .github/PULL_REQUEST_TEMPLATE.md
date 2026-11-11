<!--
PR title = the commit message on main after the squash merge, and it decides the next version.
Format: type(scope): lowercase summary, e.g. "feat(bootsound): play a jingle at boot".
Types: feat, fix, refactor, docs, chore, revert. Add "!" after the scope for a breaking change.
See CONTRIBUTING.md.
-->

## Summary

What this change does and why it exists. Intent, not implementation details.

---

## Context

- Milestone: M_ (see docs/FOUNDATION.md), or none
- Related issue / PR:

Why is this change needed now?

---

## Scope of change

Affected parts:
- [ ] `theme`
- [ ] `splash`
- [ ] `bootsound`
- [ ] `greeter`
- [ ] `system/` (systemd units, install scripts, boot config)
- [ ] `vm/` (QEMU scripts)
- [ ] `assets/`
- [ ] `docs/`
- [ ] CI / repository setup

What is included:
-

What is explicitly not included:
-

---

## Approach

High-level explanation of how the problem was solved: key decisions, trade-offs, anything
non-obvious. Don't narrate the code line by line.

---

## Risk & impact

- Runs during boot? Yes / No
- Can it stop the system from booting or reaching the login prompt? Yes / No, why:
- How to recover if it goes wrong (e.g. `vm/snapshot.sh restore <name>`, disable the unit):

---

## Testing & validation

- [ ] `cargo test` (on the Mac or in CI)
- [ ] Built and installed in the target VM (`system/dev-install.sh`)
- [ ] Rebooted the target VM and observed the result
- [ ] Not applicable (explain why)

What you saw, and anything that is not covered:

---

## Release notes

- User-visible change? Yes / No
- Breaking change (needs `!` in the title)? Yes / No
- Manual steps needed after updating (new packages, config, re-running an install script)?

---

## Follow-ups

Known follow-ups (refactors, cleanup, improvements). If none:
> No known follow-ups.
