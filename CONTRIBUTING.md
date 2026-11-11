# Contributing to Zombori

Every change reaches `main` through a pull request, gets squash-merged into one commit, and that
commit's message decides the next release. So the one thing to get right is the **PR title**.

## The flow

1. Branch off `main`: `git switch -c feat/bootsound` (the branch name is free-form; `type/topic` keeps
   it readable).
2. Commit as you like on the branch. Those commits disappear in the squash.
3. Open a PR. Its title follows the format below, and the description follows the template.
4. CI must pass: the PR title check, `cargo fmt`, `clippy` and the tests, on ARM64 Linux.
5. Squash-merge. The PR title becomes the commit on `main`, with the PR number appended.

## PR titles: Conventional Commits

```
type(scope): lowercase summary
```

| Type | Use for | Next release |
|---|---|---|
| `feat` | something new a user can see, hear or use | minor (0.1.0 → 0.2.0) |
| `fix` | a bug fix | patch (0.2.0 → 0.2.1) |
| `refactor` | restructuring with no change in behavior | none on its own |
| `docs` | guides and documentation only | none on its own |
| `chore` | tooling, CI, dependencies, repository setup | none on its own |
| `revert` | undoing an earlier change | none on its own |

The **scope** is the part you touched, in lowercase: a crate (`theme`, `splash`, `bootsound`,
`greeter`) or an area (`system`, `vm`, `assets`, `docs`, `ci`, `repo`). It can be left out when a
change spans everything.

A **breaking change** gets a `!`: `feat(theme)!: rename the boot timeline constants`. While the
version is below 1.0 this bumps the minor version; from 1.0 on, the major.

Examples:

```
feat(bootsound): play a jingle through alsa at boot
fix(splash): wait for the gpu instead of failing when it appears late
docs(vm): explain how to recover a forgotten password from the builder
chore(ci): run clippy on arm64
```

## Releases

Releases are cut by [release-please](https://github.com/googleapis/release-please); nobody edits
versions or the changelog by hand.

1. After each merge to `main`, release-please opens (or updates) a PR titled
   `chore(main): release x.y.z`. It holds the version bump (`Cargo.toml`, `Cargo.lock`,
   `version.txt`) and the new `CHANGELOG.md` entry, built from the PR titles since the last release.
2. Merge it when you want a release. That tags `vX.Y.Z` on `main` and publishes a GitHub release
   with the same notes.
3. Tags are the stable versions: `git checkout v0.2.0` gives exactly that release.

Milestones from `docs/FOUNDATION.md` usually line up with minor versions (`v0.1.0` is M3, the boot
splash), but a release can be cut whenever `main` is in a state worth keeping.

**Adding a crate:** the workspace shares one version. Add the new crate's `Cargo.lock` entry to
`extra-files` in `release-please-config.json`, like the existing ones, so releases bump it too.

## Before opening a PR

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Hardware code (`display.rs`, ALSA) only builds on Linux. Build and try it in the target VM with
`system/dev-install.sh` (see `docs/02-dev-loop-and-splash.md`), and say what you saw in the PR's
testing section.

## License

By contributing you agree that your work is dual-licensed under MIT or Apache-2.0, at the user's
option, like the rest of the project.
