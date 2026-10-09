---
title: 'MNT-236: Automatic release creation'
date: '2026-10-09'
type: implementation
status: resolved
session_id: ses_edfd87bb1ffed0SngNzrqPXVL6
services: [ci, release, AGENTS, README, install, .sessions, INDEX, 2026-10-09-release-workflow-and-install-script]
branch: mnt-236-automatic-release-creation
tickets: [MNT-236]
tags: [wiki, feature, backend]
related: [2026-10-06-sk-cli-implementation.md, 2026-10-06-sk-agent-dir-and-lock-hashes.md]
---
# MNT-236: Automatic release creation

## TL;DR

The implementation session resulted in the creation of a GitHub workflow to automate the release process and a shared install script for MacOS and Ubuntu. The workflow builds binaries for Linux x86_64, macOS x86_64, and macOS aarch64, and creates a new release in the repository when the `cargo.toml` version is changed. The default install directory is `~/.local/bin` with an override option using `SK_INSTALL_DIR`. The repository URL is hardcoded in `install.sh` as `manti-by/sk`.

---

## Overview

The implementation involved creating two workflow files, `ci.yml` and `release.yml`, to automate the release process. New files `install.sh` and `wiki/pages/2026-10-09-release-workflow-and-install-script.md` were added, and files `README.md`, `AGENTS.md`, `wiki/INDEX.md`, and `wiki/.sessions.json` were modified. The release workflow builds binaries for multiple targets and creates a new release on a version bump to `main`.

## Changed files

- `.github/workflows/ci.yml` (109/0)
- `.github/workflows/release.yml` (182/0)
- `AGENTS.md` (6/0)
- `README.md` (31/6)
- `install.sh` (182/0)
- `wiki/.sessions.json` (2/1)
- `wiki/INDEX.md` (1/0)
- `wiki/pages/2026-10-09-release-workflow-and-install-script.md` (194/0)

## Stat

```text
.github/workflows/ci.yml                           | 109 ++++++++++++

 .github/workflows/release.yml                      | 182 +++++++++++++++++++

 AGENTS.md                                          |   6 +

 README.md                                          |  37 +++-

 install.sh                                         | 182 +++++++++++++++++++

 wiki/.sessions.json                                |   3 +-

 wiki/INDEX.md                                      |   1 +

 ...26-10-09-release-workflow-and-install-script.md | 194 +++++++++++++++++++++

 8 files changed, 707 insertions(+), 7 deletions(-)
```

## Build plan

## Implementation Plan
The implementation plan involves creating a GitHub workflow to automate the release process and adding a shared install script for MacOS and Ubuntu.

### Key Technical Decisions
* Create two workflow files: `ci.yml` for checking on every push and PR, and `release.yml` for publishing on a version bump to `main`.
* Use `github/workflows/ci.yml` to run `make check` on every push and PR.
* Use `github/workflows/release.yml` to publish releases on a version bump to `main`.
* The release workflow will build four targets: Linux x86_64, Linux aarch64, macOS x86_64, and macOS aarch64.
* The default install directory will be `~/.local/bin` with an override option using `SK_INSTALL_DIR`.
* The repository URL will be hardcoded in `install.sh` as `manti-by/sk`.

### Files and Com…

## Test Results

- Session status: `resolved`
- OpenCode session id: `ses_edfd87bb1ffed0SngNzrqPXVL6`

---

## Follow-ups

- None

## References

- External: https://linear.app/mnt/issue/MNT-236/automatic-release-creation
