# Zombori

A personal operating system: the Linux kernel, a Debian base, and a growing set of Rust components
that replace the userspace piece by piece.

- **Plan and decisions:** [docs/FOUNDATION.md](docs/FOUNDATION.md)
- **Start here:** [docs/01-hand-install.md](docs/01-hand-install.md), then
  [docs/02-dev-loop-and-splash.md](docs/02-dev-loop-and-splash.md)

```sh
vm/fetch-base.sh            # download + verify the Debian builder image
vm/builder.sh               # workshop VM (target disk attached as /dev/vdb)
vm/target.sh                # boot the system you built (window + audio + ssh on :2223)
vm/snapshot.sh save <name>  # snapshot the target disk (VM off)
```
