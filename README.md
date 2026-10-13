# SK Tool

`sk` manages agent skills the way [`uv`](https://docs.astral.sh/uv/) manages Python packages: you declare
the skills a project wants in `sk.toml`, and `sk sync` installs them into a single directory that agents
already look in.

```console
$ sk init
created  ./sk.toml
created  ./.agents/skills/
created  ./sk.lock

Add skill links under [dependencies] skills in ./sk.toml, then run `sk sync`.
```

Add the skills you want, then sync:

```toml
[dependencies]
skills = [
  "https://github.com/manti-by/openwiki/tree/master/skills/wiki-consistency",
  "code-review = https://github.com/mattpocock/skills/tree/main/skills/engineering/code-review",
]
```

```console
$ sk sync
Syncing 2 skill(s) into .agents/skills/
  added     wiki-consistency/SKILL.md  5c0ab8e3
  added     code-review/SKILL.md  09eb147f
Wrote ./sk.lock
```

```
.agents/skills/wiki-consistency/SKILL.md
.agents/skills/code-review/SKILL.md
```

## Installation

The quickest route, on macOS and Ubuntu, is the install script. It downloads the newest release for
your machine, verifies its SHA-256 and drops the binary into `~/.local/bin`:

```console
$ curl -LsSf https://raw.githubusercontent.com/manti-by/sk/master/install.sh | sh
```

For more installation options check [INSTALL.md](wiki/docs/INSTALL.md).

## Commands

Run `sk --help` for full usage. Every command and its options are listed in
[MORE.md](wiki/docs/MORE.md), which also covers how a link becomes a directory, what goes in
`sk.toml` and `sk.lock`, and the GitHub rate limit.
