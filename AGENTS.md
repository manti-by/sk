# AGENTS.md

Guidance for agents and contributors working on `sk` itself.

## What this project is

`sk` is a CLI that installs agent skills into the skills directory of the agent named by `project.agent` in
`sk.toml`, which is `.agents/skills/` by default and `.claude/skills/` or `.opencode/skills/` on request. It is
modelled on [`uv`](https://docs.astral.sh/uv/): `sk.toml` is the manifest you edit, and `sk sync` reconciles
that directory against it. Rust, edition 2024.

## Commands

| Command | Purpose |
| --- | --- |
| `make build` | Build the binary into `target/debug/sk`. |
| `make run` | Run all tests. Unit tests only; none touch the network. |
| `make check` | `fmt --check`, clippy and tests; the gate before committing. |
| `make release` | Runs the `make check` gate, then builds `target/release/sk`. |
| `make sync` | Install this project's own declared skills. |
| `cargo build` | The same build, without the Makefile. |
| `cargo test` | The same tests, without the Makefile. |
| `cargo clippy --all-targets` | Lint. Must stay warning-free. |
| `cargo fmt` | Format. `cargo fmt --check` must be clean. |

## Layout

| Path | Role |
| --- | --- |
| `src/main.rs` | Argument parsing and dispatch. Keeps no logic beyond wiring. |
| `src/manifest.rs` | `sk.toml` schema (`Manifest`, `Project`, `Dependencies`, `Spec`) and the `init` template. |
| `src/lock.rs` | `sk.lock` schema (`Lock`, `Entry`), load and save. |
| `src/github.rs` | `RepoRef` link parsing and the HTTP `Client`. The only place that knows about GitHub. |
| `src/sync.rs` | `sync`: resolve, download, install, prune, rewrite the lock. |
| `src/init.rs` | `init`: create the three entries, never overwriting. |
| `src/parallel.rs` | `parallel::map`, a small ordered work queue over `std::thread::scope`. |
| `src/paths.rs` | `Agent` and `Layout`, the paths of every file `sk` reads or writes. |
| `src/error.rs` | The `Error` type, `Result`, and the `bail!`/`ensure!` macros. |
| `src/util.rs` | `write_atomic`, so a reader never sees a half-written `SKILL.md`, and `sha256_hex`, the digest recorded per skill. |

## Conventions

- Dependencies stay minimal on purpose: `ureq` plus `std::thread`, `toml`, `serde`, `serde_json`. Do not add
  an async runtime; the work queue in `src/parallel.rs` is enough.
- Errors are `Error::new` for anything the user can fix (a bad link, a duplicate name) and
  `Error::with_source` when wrapping an underlying I/O or HTTP failure. `main` prints the whole chain.
- Every module that can be tested in isolation carries a `#[cfg(test)] mod tests` block. Keep them free of
  network calls and of real sleeps.
- Files are written through `util::write_atomic`, never with `fs::write`, so an interrupted sync cannot leave
  a truncated `SKILL.md`.
- Anything that becomes a directory name is validated by `manifest::is_valid_name`. No path built from a
  manifest link may contain user-controlled separators.
- Changing the shape of `sk.lock` means bumping `lock::LOCK_VERSION`. A lock from an older version is upgraded
  by the next sync, so new fields need a `serde(default)` to keep old locks loadable.

## Working on this repository

`sk.toml`, `sk.lock` and `.agents/skills/` are this project's own manifest, lock and installed skills. Run
`./target/debug/sk sync` after editing `sk.toml`.

Because `sk.lock` pins commits, upstream changes to a skill only arrive via `sk sync --upgrade`. Commit the
refreshed lock with the manifest change. Each entry also carries the SHA-256 of the installed `SKILL.md`, so
reviewing the lock diff shows whether content actually moved.

Releases are cut by CI from a version bump, not by hand: bump `version` in `Cargo.toml` (and the matching
`project.version` in `sk.toml`), merge to `master`, and `.github/workflows/release.yml` builds the four
platform binaries and publishes `v<version>`. The publish job is gated behind `make check`, so a red tree
never releases. `.github/workflows/ci.yml` runs that gate plus an offline smoke test of `install.sh` on
every push and pull request.

Both workflows keep their shell in `.github/scripts/`, one script per step, rather than inlined `run:`
blocks. An inline script gets no syntax highlighting, no editor tooling and no shellcheck; a file in that
directory gets all three, and `ci.yml` lints the whole directory on every push. The scripts are `bash`,
take their inputs from the environment rather than arguments so the workflow can pass them through `env:`
instead of interpolating `${{ … }}` into shell text, and assume they run from the repository root.

## Adding a command

1. Add a module with a `run` entry point and a test block.
2. Wire it in the `match command` in `src/main.rs` and add it to `USAGE` and to the README table.
3. Keep the module free of argument parsing; `main.rs` owns that.