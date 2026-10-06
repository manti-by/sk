---
title:              sk — per-agent skill directories and content-hashed lock
date:               2026-10-06
type:               implementation
status:             resolved
session_id:         ses_eed4dd3cfffeurW34nr5sHmDKg
services:           [sk]
branch:             -
tickets:            []
tags:               [rust, cli, skills, sha256, lockfile, toml]
related:            [2026-10-06-sk-cli-implementation.md]
---

# sk — per-agent skill directories and content-hashed lock

## TL;DR

Two changes to `sk`, both aimed at making its output trustworthy rather than just quiet. `sk.toml` now takes
`agent` in `[project]` (`agents` by default, `claude` or `opencode`), and that setting picks the directory
skills are installed into: `.agents/skills/`, `.claude/skills/` or `.opencode/skills/`. `sk sync` also records
the SHA-256 of every installed `SKILL.md` in `sk.lock`, and prints `old -> new` digests for the skills whose
content actually changed, so a sync tells you *what* moved rather than only that something did.

`sk.lock`'s format version went from 1 to 2 for the new field, but an older lock is upgraded in place by the
next sync rather than rejected. 39 unit tests pass; clippy and rustfmt are clean. Continues from
[[2026-10-06-sk-cli-implementation]], which built the original CLI.

---

## Overview

- **`Agent`** (`src/paths.rs`) — a three-variant enum with `serde(rename_all = "lowercase")`, so
  `agent = "opencode"` parses and `agent = "cursor"` fails loudly instead of falling back to a default.
- **`Layout`** (`src/paths.rs`) — gained an `agent` field and `with_agent()`, which clones the layout with a
  different agent. `skills_dir()`, `skill_dir()` and `skill_file()` derive from it, so nothing else in the
  codebase knows the difference between the three directories.
- **`Entry.hash`** (`src/lock.rs`) — lowercase hex SHA-256 of the installed `SKILL.md`.
- **`LOCK_VERSION`** (`src/lock.rs`) — now 2, and documented as the lock *format* version rather than the `sk`
  version.

## Decisions

Two questions were put to the user before implementing the `agent` setting; both answers are recorded because
the alternatives were reasonable.

- **Where the setting lives.** Chosen: `agent = "claude"` directly in `[project]`, alongside `name` and
  `version`. Rejected: a `[tool.sk]` table, which is the PEP 621 idiom but adds a level of nesting for a
  single key; and putting it in `[skills]`, which would mix a project-level fact into a list of links.
- **Directory layout.** Chosen: `<agent-dir>/skills/<name>/SKILL.md` for all three, so
  `.claude/skills/` and `.opencode/skills/` mirror the `.agents/skills/` convention rather than inventing a
  second shape. Confirmed against OpenCode's published docs, which list `.opencode/skills/` as its
  project-scoped directory.

Two judgement calls made without asking:

- **Old locks are upgraded, not rejected.** `Lock::load` accepts any version in `1..=LOCK_VERSION` and refuses
  only a *newer* one, on the grounds that refusing to read your own project's lock after a format bump is
  annoying, and that the new field can be `serde(default)`-ed safely. If a hard stop on every format change is
  preferred, it is a one-line change in `Lock::load`.
- **SHA-256 written out rather than added as a dependency.** The project keeps dependencies minimal on purpose,
  and a digest the user can reproduce with `shasum -a 256` is worth more in `sk.lock` than the lines a crate
  would have saved.

## Step 1 — `agent` in `sk.toml`

The enum and the mapping to a directory:

```rust
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Agent {
    /// `.agents/skills`, read by every agent following the open standard.
    #[default]
    Agents,
    /// `.claude/skills`, read by Claude Code.
    Claude,
    /// `.opencode/skills`, read by OpenCode.
    OpenCode,
}

impl Agent {
    /// Directory holding the skills, relative to the project root.
    pub fn skills_dir(self) -> &'static str {
        match self {
            Agent::Agents => SKILLS_DIR,
            Agent::Claude => ".claude/skills",
            Agent::OpenCode => ".opencode/skills",
        }
    }
}
```

`Project.agent` is an `Option<Agent>`, with `Manifest::agent()` collapsing the absent case to the default. The
`init` template documents the option commented out, so a fresh project keeps `.agents/skills/` without
pretending the user chose it.

The only structural change is that `Layout` knows which agent it is writing for:

```rust
pub struct Layout {
    pub root: PathBuf,
    pub agent: Agent,
}

impl Layout {
    pub fn with_agent(&self, agent: Agent) -> Self {
        Self { root: self.root.clone(), agent }
    }
}
```

## Step 2 — `init` writes the manifest first

`init` previously created `.agents/skills/` and then wrote `sk.toml`. That order is wrong once the directory
depends on the file: on a fresh project the setting does not exist yet. The manifest is now written or kept
first, then loaded to pick the directory.

```rust
let layout = &layout.with_agent(Manifest::load(layout)?.agent());
let skills_dir = layout.skills_dir();
```

Reordering also means a user who edits `agent = "opencode"` into an existing manifest and re-runs `sk init`
gets `.opencode/skills/` created, which is what a smoke test confirmed.

`sync` does the same rebinding, once, before installing or pruning:

```rust
pub fn run(root: &Layout, upgrade: bool) -> Result<()> {
    let manifest = Manifest::load(root)?;
    let layout = &root.with_agent(manifest.agent());
```

Prune therefore only ever considers the configured agent's directory. Switching agents leaves the previous
directory untouched rather than pruning across into it, which is the conservative reading of a tool that only
deletes what it recorded in `sk.lock`.

## Step 3 — SHA-256 in `sk.lock`

`Entry` gains a hash, defaulted so a version 1 lock still loads:

```rust
/// SHA-256 of the installed `SKILL.md`, lowercase hex, so a sync can report
/// what actually changed instead of only that something did.
///
/// Defaults to empty so a lock written before hashes existed still loads;
/// the next `sk sync` fills it in and rewrites the lock at the current
/// version.
#[serde(default)]
pub hash: String,
```

`Lock::load` now checks `lock.version > 0 && lock.version <= LOCK_VERSION` instead of exact equality, so an
old lock is read and rewritten by the next sync. `Lock::hash_for` returns `None` for an entry whose hash is
empty, which is how the report distinguishes "no earlier digest" from "earlier digest".

The digest itself is a straight SHA-256 in `src/util.rs` — padding, message schedule, the eight working
variables — with the round constants spelled out at the top. About 60 lines, verified against the FIPS 180-4
vectors and against reference digests for lengths that straddle the block-padding boundary (55, 56, 64, 65 and
1000 bytes), so a padding bug on a multi-block message cannot pass silently.

## Step 4 — What the report says

`Installed` now carries both the new digest and whatever the lock held before:

```rust
fn digest_change(&self) -> Option<String> {
    if self.status == Status::Unchanged {
        return None;
    }
    let short = |hash: &str| &hash[..hash.len().min(SHORT_HASH)];
    Some(match self.previous_hash.as_deref().filter(|p| !p.is_empty()) {
        Some(previous) if previous != self.hash => {
            format!("{} -> {}", short(previous), short(&self.hash))
        }
        _ => short(&self.hash).to_owned(),
    })
}
```

Unchanged skills print no digest at all, so anything carrying one is worth reading:

```console
$ sk sync
Syncing 2 skill(s) into .agents/skills/
  added     wiki-consistency/SKILL.md  41edece4
  updated   code-review/SKILL.md  a3f1c0b8 -> 9d4e27af
  unchanged other-skill/SKILL.md
Wrote ./sk.lock
```

Recording the hash also makes `git diff sk.lock` informative: an upstream commit that moved content shows a
changed digest, one that touched other files in the repository shows an unchanged line.

## Friction hit during implementation

Four compile failures, all of them small and none of them bugs in the design.

- **`toml::from_str::<Agent>("agent = \"agents\"")` fails.** A bare enum is not a table, so the key has no
  field to land in. The parse test needed a local `struct Holder { agent: Agent }` wrapper.
- **A closure could not return a borrow.** `let short = |hash: &str| &hash[..]` needs the input lifetime to
  outlive the closure's, which inference will not assume. Rewrote it as a nested `fn`.
- **`write_atomic` stopped compiling** when `map_err` was given a block that returned `Err(...)` instead of an
  `Error`, nesting one `Result` inside the other.
- **Clippy `chunks_exact_to_as_chunks`.** `as_chunks::<64>()` is the suggested spelling for the constant chunk
  size in the SHA-256 loop.

A first pass at the version check also rejected any lock whose version was not exactly `LOCK_VERSION`, which
would have made the version 1 lock in this very repo unreadable. Relaxing it to `1..=LOCK_VERSION` is what
makes the in-place upgrade work.

## Test Results

39 unit tests, all passing. No network access and no real sleeps anywhere in the suite.

- `make check` — `cargo fmt --check` clean, `cargo clippy --all-targets` no warnings, `cargo test` 39 passed
  0 failed
- New tests (7) — per-agent directory mapping; `Agent` parsing from the manifest spelling, including rejection
  of an unknown agent; `Layout` paths following the agent; `init` creating the directory each of the three
  agents asks for; `Manifest::agent()` defaulting and overriding; `digest_change` for added, updated and
  unchanged; prune following the configured agent; a version 1 lock loading with no hash

End-to-end verification against the real GitHub API, on this repo:

- `sk sync` upgraded `sk.lock` from `version = 1` to `version = 2` and wrote hashes that match
  `shasum -a 256` on the installed files, checked byte for byte
- A second `sk sync` printed `unchanged` for both skills with no digest
- Appending a line to an installed `SKILL.md` and re-syncing printed
  `updated   wiki-consistency/SKILL.md  5c0ab8e3` while its neighbour stayed silent
- `sk init` against a scratch directory with `agent = "opencode"` created `.opencode/skills/` and left the
  manifest alone

Not exercised: a real sync with `agent = "claude"` or `"opencode"` on a project whose skills were previously
installed under `.agents`. Download and prune are unchanged apart from the directory they use, but the
transition case is untested.

---

## Follow-ups

- **Switching agents leaves the old directory behind.** `.agents/skills/` survives a switch to
  `agent = "claude"` with stale copies inside. Skipping it was deliberate (never delete what the current lock
  does not describe), but a user who switches back and forth may want a `sk sync` note, or the option of
  pruning the previous agent's directory.
- **Nothing is committed.** The working tree carries both features uncommitted; the repo still has no git
  remote and the README's clone URL is a `<you>/sk` placeholder.
- **`wiki/` provenance is still unexplained** and still untracked, per
  [[2026-10-06-sk-cli-implementation#Follow-ups]].
- **Branch names containing a slash still cannot be expressed** — `tree/main` and `main` are indistinguishable
  in a link.
- **`README.md` documents a digest that the tool truncates to 8 characters.** The lock stores all 64; the
  README example shows the short form without saying so in the sample output block.

## References

- Related: [[2026-10-06-sk-cli-implementation]]
- [OpenCode agent skills](https://opencode.ai/docs/skills/) — confirms `.opencode/skills/<name>/SKILL.md`
- [FIPS 180-4](https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.180-4.pdf) — SHA-256 spec and test vectors
- [PEP 621](https://peps.python.org/pep-0621/) — the `[project]` schema `agent` was added to