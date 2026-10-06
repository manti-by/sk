---
title:              sk CLI — agent skill manager
date:               2026-10-06
type:               implementation
status:             resolved
session_id:         ses_eed698b3effewh52ORmOlA7iQL
services:           [sk]
branch:             -
tickets:            []
tags:               [rust, cli, skills, uv, packaging]
related:            []
---

# sk CLI — agent skill manager

## TL;DR

Built `sk` from scratch in this session: a Rust CLI that installs agent skills the way `uv` installs Python
packages. `sk.toml` declares which skills a project wants, `sk sync` resolves each link to a pinned commit,
downloads `SKILL.md` in parallel and installs it under `.agents/skills/`, and `sk.lock` records the commits so
later syncs stay reproducible. 29 unit tests pass, clippy and rustfmt are clean, and the whole tree is staged
but **not yet committed**.

Five design questions were put to the user before any code was written; the answers are recorded under
[Decisions](#decisions) because several of them contradict the original brief and would be non-obvious to a
future reader.

---

## Overview

- **Binary:** `sk`, one Rust crate, edition 2024, no async runtime.
- **Commands:** `sk init`, `sk sync`, `sk sync --upgrade`. Both commands accept an optional project directory,
  defaulting to `.`.
- **Install layout:** `.agents/skills/<name>/SKILL.md`.
- **Dependencies:** `ureq`, `toml`, `serde`, `serde_json`. Deliberately no tokio/reqwest — see
  [[#step-3--parallel-sync-without-an-async-runtime]].
- **Size:** ~1.5k lines of Rust across 10 modules, of which roughly a third is tests.

Source layout, one module per concern:

- `src/main.rs` — argument parsing and dispatch only
- `src/manifest.rs` — `sk.toml` schema and the `init` template
- `src/lock.rs` — `sk.lock` schema
- `src/github.rs` — link parsing plus the HTTP client; the only module that knows about GitHub
- `src/sync.rs` — resolve, download, install, prune, rewrite the lock
- `src/init.rs` — scaffolding, never overwrites
- `src/parallel.rs` — ordered work queue over `std::thread::scope`
- `src/paths.rs`, `src/error.rs`, `src/util.rs` — layout, error type, atomic writes

## Decisions

The brief asked for several things that were ambiguous or self-contradictory. Each was put to the user as a
question before implementation.

- **Directory name.** The brief said `.agents/skills/` for `init` but `.agent/skills/` for `sync`. Resolved to
  `.agents/skills/`.
- **Recursive vs. single file.** The brief said "download everything recursively" and also "for now download
  only SKILL.md". Resolved to the narrow reading: exactly the one `SKILL.md` named by the link, no directory
  walking.
- **Naming.** Entries are plain strings; `name = link` was added as an optional override so two repos can
  both ship a `code-review` skill.
- **GitHub auth.** `GITHUB_TOKEN`/`GH_TOKEN` read from the environment when present, otherwise anonymous. The
  unauthenticated API allows 60 requests/hour, which 60 does not survive.
- **Concurrency.** Minimal dependencies, so `std::thread` rather than an async runtime.

Two things beyond the brief were added, both to keep the lock file from being inert:

- **`--upgrade`.** Because `sk.lock` pins commits, upstream changes would otherwise be unreachable. Without
  the flag, a locked skill could never be updated.
- **Pruning.** Directories `sk` installed earlier that left `sk.toml` are deleted. Restricted to directories
  recorded in `sk.lock`, so a hand-written skill next to a managed one is never touched.

## Step 1 — Manifest and lock schemas

`sk.toml` mirrors PEP 621 for `[project]` (`name`, `version`, `description`, `readme`, `license`, `authors`,
`keywords`, `homepage`, `repository`) and holds a list of links under `[skills]`.

```toml
[skills]
skills = [
  "https://github.com/manti-by/openwiki/tree/master/skills/wiki-consistency",
  "code-review = https://github.com/mattpocock/skills/tree/main/skills/engineering/code-review",
]
```

Authors accept both forms via an untagged enum:

```rust
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Author {
    Name(String),
    Detailed { name: Option<String>, email: Option<String> },
}
```

Both schemas are `deny_unknown_fields`, so a typo like `nmae = "x"` fails loudly instead of being ignored.

`sk.lock` records the resolved commit per skill. Storing the commit rather than the ref is what makes a
second `sk sync` reproduce byte-identical content after a branch moves.

## Step 2 — GitHub link parsing

`RepoRef::parse` accepts three shapes and normalises them to `{owner, repo, git_ref, path}`:

- `https://github.com/<owner>/<repo>/tree/<ref>/<path>` — the usual form
- `https://github.com/<owner>/<repo>/blob/<ref>/<path>/SKILL.md` — a link straight at the file; the trailing
  `SKILL.md` is stripped back to the containing directory
- `https://github.com/<owner>/<repo>` — a skill at the repository root; ref defaults to `HEAD` so GitHub
  resolves the default branch

Query strings and fragments are stripped, a trailing `.git` is removed from the repo segment, and anything
unrecognised is rejected rather than guessed at. `dir_name()` returns the last path segment, falling back to
the repo name for root-level skills.

## Step 3 — Parallel sync without an async runtime

Downloads run through a small work queue. Each worker pops the next unclaimed item from a shared
`Mutex<VecDeque>`, so a slow download cannot stall the others the way static chunking would.

**File:** `src/parallel.rs`

```rust
let queue = Mutex::new(items.into_iter().enumerate().collect::<VecDeque<_>>());
let collected = Mutex::new(Vec::with_capacity(total));

std::thread::scope(|scope| {
    for _ in 0..workers {
        scope.spawn(|| loop {
            let Some((index, item)) = queue.lock().expect("queue poisoned").pop_front() else {
                break;
            };
            let result = task(index, item);
            collected.lock().expect("results poisoned").push((index, result));
        });
    }
});
```

Results are sorted by index before return, so output order is deterministic regardless of completion order.
Worker count is `(cores * 2).clamp(1, 8)`, never more than the number of items.

Each task resolves then downloads: `/repos/{o}/{r}/commits/{ref}` yields a commit SHA, and the raw URL is then
built against that SHA so the bytes fetched are pinned to the resolution.

Failures are values, not panics. A sync where two of four skills fail still installs the other two, still
writes the lock, and still exits non-zero with a summary.

## Step 4 — Init, atomic writes and pruning

`init` creates `.agents/skills/`, `sk.toml` and `sk.lock` only when absent, and reports what it left alone.
The generated manifest takes its `project.name` from the project directory name.

Every file is written through `write_atomic` — a temp sibling plus a rename — so a reader never observes a
half-written `SKILL.md` and an interrupted sync leaves the previous content intact.

Pruning reads `.agents/skills/` and removes a directory only when all three hold:

- the name is not in `sk.toml`
- the name **is** in the previous `sk.lock` (so it was installed by `sk`)
- the directory contains a `SKILL.md`

## Step 5 — Makefile and install docs

`make build` and `make run` wrap cargo, plus `make check` (fmt-check + clippy + tests), `make sync`,
`make install` and `make clean`. Written in portable syntax because the machine runs GNU Make 3.81.

README gained an Installation section. The honest version: `sk` is `cargo install --path .` only. There is no
`curl | sh` installer and no Homebrew tap, because `sk` is not published anywhere and uv ships prebuilt
binaries. The section says so rather than implying a download exists.

## Bugs hit during implementation

Both were caught by exercising the built binary against a scratch directory, not by the unit tests.

### 1. Optional directory argument was rejected before being read

`sk init /some/path` failed with `unexpected argument`, because `main.rs` checked for a second positional
argument *before* consuming it as the project directory.

**Fix:** the command name is split off first, then the remaining positionals are validated as at most one
directory.

### 2. Path-traversal attempt reported a confusing error

An entry like `../../etc = https://github.com/a/r/tree/main/s` was misparsed as a bare link, so the error read
`is not a GitHub link` — technically true but pointing at the wrong problem.

**Cause:** the rename split only accepted the left side as a name when it was already a valid name, so an
invalid one silently fell through to the no-rename branch.

**Fix:** an explicit `name = link` rename is now detected first (rejecting anything containing `://` so URLs
with `=` in a query string are unaffected), and the name is then validated with a message that names the
actual problem. The underlying defence is unchanged: directory names are restricted to
`[A-Za-z0-9._-]`, so no manifest link can write outside `.agents/skills/`.

## Test Results

29 unit tests, all passing. No network access and no real sleeps anywhere in the suite.

- `cargo test` — 29 passed, 0 failed
- `cargo clippy --all-targets` — no warnings
- `cargo fmt --check` — clean

Coverage by area:

- **Link parsing** (8) — tree/blob/root shapes, `.git` suffix, query strings, rejection of non-GitHub hosts
  and unknown path segments
- **Manifest** (7) — naming from URL, explicit rename, duplicate-name detection, both author forms, traversal
  rejection, template round-trip, unknown-field rejection
- **Lock** (3) — missing file as empty, round-trip, foreign version rejected
- **Prune** (3) — removes only locked-and-undeclared skills, survives a hand-made skill, no-op when the
  directory is absent
- **Init** (3) — creates all three entries, leaves existing files alone, derives the project name
- **Parallel** (3) — order preservation, empty input, worker bound
- **Atomic write** (1) — create then replace, no temp file left behind

End-to-end behaviour was verified manually against a scratch directory: first sync installs, second sync
reports `unchanged` and leaves mtimes untouched, deleting a link from `sk.toml` prunes only that skill,
`--upgrade` re-resolves and repairs a corrupted file, and a tampered `SKILL.md` is rewritten on next sync.

---

## Follow-ups

- **Nothing is committed.** All work is staged; the repo had no commits to begin with and no git remote is
  configured. The README's clone URL is a `<you>/sk` placeholder.
- **`wiki/` provenance is unexplained.** The directory appeared mid-session and I initially, wrongly,
  attributed it to the `wiki-consistency` skill. `sk sync` only ever writes
  `.agents/skills/<name>/SKILL.md`, so it could not have created it. `TEMPLATE.md` there carries a July mtime
  while the rest are from today, which points to a copy from the OpenWiki repo by some plugin or hook. Still
  untracked.
- **`/wiki-update` is referenced but absent.** `wiki/README.md` and `INDEX.md` name `/wiki-sync` and
  `/wiki-update`; only `wiki-sync` is installed. Worth reconciling the docs with what actually exists.
- **Branch names containing a slash cannot be expressed.** `tree/main` and `main` are indistinguishable in a
  link, so `sk` assumes the first segment after `tree/` is the ref. Point such skills at a tag or commit.
- **No CI.** `make check` is the gate; nothing runs it automatically.

## References

- [`uv`](https://docs.astral.sh/uv/) — the packaging model being mirrored
- [PEP 621](https://peps.python.org/pep-0621/) — the `[project]` schema
- [Anthropic Agent Skills](https://code.claude.com/docs/en/skills) — the `.agents/skills/` convention