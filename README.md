# Zombori

A personal operating system: the Linux kernel, a Debian base, and a growing set of Rust components
that replace the userspace piece by piece.

- **Plan and decisions:** [docs/FOUNDATION.md](docs/FOUNDATION.md)
- **Start here:** [docs/01-hand-install.md](docs/01-hand-install.md), then
  [docs/02-dev-loop-and-splash.md](docs/02-dev-loop-and-splash.md), then
  [docs/03-gate.md](docs/03-gate.md), then [docs/04-tavern.md](docs/04-tavern.md)

```sh
vm/fetch-base.sh            # download + verify the Debian builder image
vm/builder.sh               # workshop VM (target disk attached as /dev/vdb)
vm/target.sh                # boot the system you built (window + audio + ssh on :2223)
vm/snapshot.sh save <name>  # snapshot the target disk (VM off)
```

## Versions and contributing

Stable versions are the `vX.Y.Z` tags and their [GitHub releases](https://github.com/zombori-co/zombori/releases);
what changed in each is in [CHANGELOG.md](CHANGELOG.md). Changes go through pull requests with
Conventional Commit titles, and releases are cut from those automatically. See
[CONTRIBUTING.md](CONTRIBUTING.md).

## License

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at your option.
