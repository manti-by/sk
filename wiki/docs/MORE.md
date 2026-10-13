# Commands and reference

The command list, `sk.toml` and `sk.lock`, the GitHub rate limit, and how to work on `sk` itself.
`sk --help` covers the flags; this file covers everything behind them.

## Commands

- `sk init [DIR]` - Creates the agent's skills directory (`.agents/skills/` by default), `sk.toml` and `sk.lock`. Existing files are left untouched.
- `sk sync [DIR]` - Downloads every skill in `sk.toml` in parallel and installs it under the agent's skills directory.
- `sk sync --upgrade` - Re-resolves branches and tags instead of reusing the commits in `sk.lock`.

Both commands default to the current directory. Run `sk --help` for full usage.

## How a link becomes a directory

A skill is a GitHub directory containing a `SKILL.md`. The skill is installed as
`.agents/skills/<last path segment>/SKILL.md`, so the link
`https://github.com/mattpocock/skills/tree/main/skills/engineering/code-review` installs to
`.agents/skills/code-review/SKILL.md`. Prefix an entry with `name = ` to pick the directory yourself, which
is useful when two repositories both ship a `code-review` skill.

Accepted link shapes:

- `https://github.com/<owner>/<repo>/tree/<ref>/<path>` — the usual form
- `https://github.com/<owner>/<repo>/blob/<ref>/<path>/SKILL.md` — a link straight to the file
- `https://github.com/<owner>/<repo>` — a skill at the repository root, on the default branch

Only `SKILL.md` is downloaded for now. Directory names are restricted to letters, digits, `.`, `_` and `-`,
so a link can never write outside `.agents/skills/`.

## sk.toml

Two tables, mirroring [`pyproject.toml`](https://peps.python.org/pep-0621/).

`[project]` holds metadata and accepts `name`, `version`, `description`, `readme`, `license`, `authors`,
`keywords`, `homepage`, `repository` and `agent`. Apart from `agent`, `sk` does not act on these; they are
there to describe the project. Authors may be written as `"Ada Lovelace"` or as
`{ name = "Ada Lovelace", email = "ada@example.com" }`.

`agent` picks which agent's directory the skills are installed into:

- `"agents"` (default) - `.agents/skills/`
- `"claude"` - `.claude/skills/`
- `"opencode"` - `.opencode/skills/`

```toml
[project]
name = "my-project"
agent = "claude"
```

`sk init` creates the directory the setting asks for, and `sk sync` installs into it. Anything other than the
three names above is rejected when `sk.toml` is read.

`[dependencies]` holds `skills`, a list of links.

## sk.lock

`sk.lock` records the commit each skill was downloaded from, so a later `sk sync` fetches exactly the same
content even after a branch moves. It also records the SHA-256 of every installed `SKILL.md`, which is what lets
a sync tell you *what* changed:

```console
$ sk sync
Syncing 2 skill(s) into .agents/skills/
  added     wiki-consistency/SKILL.md  41edece4
  updated   code-review/SKILL.md  a3f1c0b8 -> 9d4e27af
  unchanged other-skill/SKILL.md
Wrote ./sk.lock
```

Skills whose content did not change print no digest, so anything with one is worth reading. Diffs of `sk.lock`
show the digests too, so `git diff` tells you whether an upstream commit moved content or only touched other
files in the repository. Every digest is a plain SHA-256, so you can check one yourself with
`shasum -a 256 .agents/skills/code-review/SKILL.md`.

Commit `sk.lock` alongside `sk.toml`. To pick up upstream changes, run `sk sync --upgrade` and commit the
refreshed lock.

`version` at the top of the file is the lock format, not the `sk` version. `sk` bumps it whenever the shape of
the file changes and rewrites the lock on the next sync, so a lock written by an older `sk` is upgraded in place
instead of being rejected. A lock from a *newer* format is refused, because this build cannot know what its extra
fields mean; delete it and run `sk sync` to re-resolve.

`sync` also prunes: a directory `sk` installed earlier that is no longer declared in `sk.toml` is deleted.
Skills you wrote yourself are never touched, because only directories recorded in `sk.lock` are considered
removable.

## GitHub rate limits

Unauthenticated GitHub API requests are limited to 60 per hour. Set `GITHUB_TOKEN` (or `GH_TOKEN`) to raise
the limit to 5000 per hour:

```console
$ export GITHUB_TOKEN=ghp_...
```

## Development

```console
$ make build     # build target/debug/sk
$ make run       # run the test suite
$ make check     # fmt --check, clippy and tests
$ make release   # run the check gate, then build target/release/sk
```

The Makefile only wraps cargo, so `cargo build`, `cargo test` and `cargo clippy` work as usual. `make release`
depends on `make check`, so a tree that fails the gate never produces a release binary; it prints the size of
the result when it finishes.

Releases are automated. Bump `version` in `Cargo.toml` (and the matching `project.version` in `sk.toml`), open
a pull request and merge it into `master`; `.github/workflows/release.yml` then builds Linux and macOS binaries
for x86_64 and arm64, checksums them and publishes a `v<version>` GitHub release. A push that does not change
the version builds nothing. `.github/workflows/ci.yml` runs `make check` plus an offline smoke test of
`install.sh` on every push and pull request.