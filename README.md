# sk

`sk` manages agent skills the way [`uv`](https://docs.astral.sh/uv/) manages Python packages: you declare
the skills a project wants in `sk.toml`, and `sk sync` installs them into a single directory that agents
already look in.

```console
$ sk init
created  ./.agents/skills/
created  ./sk.toml
created  ./sk.lock
```

Add the skills you want, then sync:

```toml
[skills]
skills = [
  "https://github.com/manti-by/openwiki/tree/master/skills/wiki-consistency",
  "code-review = https://github.com/mattpocock/skills/tree/main/skills/engineering/code-review",
]
```

```console
$ sk sync
Syncing 2 skill(s) into .agents/skills/
  added     wiki-consistency/SKILL.md
  added     code-review/SKILL.md
Wrote ./sk.lock
```

```
.agents/skills/wiki-consistency/SKILL.md
.agents/skills/code-review/SKILL.md
```

## Installation

`sk` is a single Rust binary, so installation is `cargo install`. It goes into `~/.cargo/bin`, which
[`rustup`](https://rustup.rs) puts on your `PATH`:

```console
$ git clone https://github.com/<you>/sk
$ cd sk
$ cargo install --path .
```

or, from a checkout, `make install`.

Once on your `PATH`, `sk` works in any project directory:

```console
$ cd ~/some-project
$ sk init
created  ./.agents/skills/
created  ./sk.toml
created  ./sk.lock
$ sk sync
```

Other options:

```console
$ cargo install --path . --root /usr/local   # choose the prefix yourself
$ cargo install --path . --force            # overwrite an installed sk
$ cargo install --path . --locked           # build with the committed Cargo.lock
```

To remove it again:

```console
$ cargo uninstall sk
```

There is no `curl | sh` installer or Homebrew tap yet. `uv` has both because it ships prebuilt binaries;
`sk` is `cargo install` only until it is published somewhere.

## Commands

| Command | What it does |
| --- | --- |
| `sk init [DIR]` | Creates `.agents/skills/`, `sk.toml` and `sk.lock`. Existing files are left untouched. |
| `sk sync [DIR]` | Downloads every skill in `sk.toml` in parallel and installs it under `.agents/skills/`. |
| `sk sync --upgrade` | Re-resolves branches and tags instead of reusing the commits in `sk.lock`. |

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
`keywords`, `homepage` and `repository`. `sk` does not act on these; they are there to describe the project.
Authors may be written as `"Ada Lovelace"` or as `{ name = "Ada Lovelace", email = "ada@example.com" }`.

`[skills]` holds `skills`, a list of links.

## sk.lock

`sk.lock` records the commit each skill was downloaded from, so a later `sk sync` fetches exactly the same
content even after a branch moves. Commit `sk.lock` alongside `sk.toml`. To pick up upstream changes, run
`sk sync --upgrade` and commit the refreshed lock.

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
```

The Makefile only wraps cargo, so `cargo build`, `cargo test` and `cargo clippy` work as usual.