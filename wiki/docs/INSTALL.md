# Installation

The quickest route, on macOS and Ubuntu, is the install script. It downloads the newest release for
your machine, verifies its SHA-256 and drops the binary into `~/.local/bin`:

```console
$ curl -LsSf https://raw.githubusercontent.com/manti-by/sk/master/install.sh | sh
```

Prebuilt binaries are published for Linux and macOS on both x86_64 and arm64. Read the script before
piping it to a shell if you prefer; it is plain POSIX `sh` and only needs `curl` or `wget`, `tar`, `sed`,
`cut`, `uname`, `mktemp` and a SHA-256 tool (`sha256sum`, `shasum` or `openssl`). Pin a release with
`--version`, or install somewhere else with `--dir`:

```console
$ curl -LsSf https://raw.githubusercontent.com/manti-by/sk/master/install.sh | sh -s -- --version v0.1.2
$ curl -LsSf https://raw.githubusercontent.com/manti-by/sk/master/install.sh | sh -s -- --dir /usr/local/bin
```

Set `SK_INSTALL_DIR` to change the target directory without a flag. If `~/.local/bin` is not on your
`PATH`, the script says so; add it before running `sk`.

`SK_VERSION` is the flag-free equivalent of `--version`, and `GITHUB_TOKEN` (or `GH_TOKEN`) raises the
GitHub rate limit the script resolves releases with. Run `./install.sh --help` for the full list,
including `SK_REPO` and `SK_RELEASE_BASE` for pointing the script at a fork.

`sk` is also a single Rust binary, so `cargo install` works. It goes into `~/.cargo/bin`, which
[`rustup`](https://rustup.rs) puts on your `PATH`:

```console
$ git clone https://github.com/manti-by/sk
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

To remove a script install, delete the binary; to remove a `cargo install`:

```console
$ rm ~/.local/bin/sk
$ cargo uninstall sk
```

There is no Homebrew tap yet. The install script covers `curl | sh` the way `uv` does.