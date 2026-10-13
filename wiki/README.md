# SK Wiki

A persistent, compounding knowledge base for this project. Each page captures **one session** -
a debugging chase, a code review, an investigation into how something works, or a set of changes
- so the reasoning survives after the terminal scrollback is gone.

Inspired by [Karpathy's LLM-maintained wiki pattern](https://gist.github.com/karpathy/442a6bf555914893e9891c11519de94f):
the human curates and directs; the agent does the bookkeeping (writing pages, cross-linking,
keeping the index current). Pages are plain Markdown so they stay human-readable and diff cleanly in git.

## Layout

- `README.md` - This file - conventions (the "schema").
- `TEMPLATE.md` - The one unified page template. Copy it to start a page.
- `INDEX.md` - Catalog of every page with a one-line summary. Kept current on each new page.
- `QUESTIONS.md` - Open questions about wiki discrepancies, awaiting a human answer.
- `pages/YYYY-MM-DD-<topic>.md` - The pages themselves, created on demand by `/wiki-update`.
- `docs/` - **User-facing documentation**, published to readers rather than to the team. See "Docs" below.
- `.openwiki-sessions.json` - Internal cache mapping session id to page file (`{ "<session_id>": "<file>.md" }`),
  kept current by `/wiki-update`; created on first use. Optional.

## Naming convention

```
pages/YYYY-MM-DD-kebab-case-topic.md
```

- Date-prefixed so pages sort chronologically and filenames stay unique.
- `topic` is a short kebab-case slug of the subject (e.g. `multi-agent-plan-step`, `composite-command`).
- Meta files (`README.md`, `TEMPLATE.md`, `INDEX.md`, `QUESTIONS.md`) are **not** date-prefixed.

## Docs

`docs/` holds documentation for people *using* `sk`, not for people working on it. It is the one part of
the wiki that is written for an external audience, so it follows different rules from `pages/`:

- **No frontmatter, no date prefix, no page `type`.** A doc is not a session, so it has no session id
  and nothing to cross-link. Filenames are plain and stable, because the repository `README.md` links to
  them by relative path and those links must not rot.
- **Console transcripts are copied from a real run.** Run the command and paste what it printed. A
  transcript that drifts from the binary is worse than no transcript.
- **Every behavioural claim is checked against the code**, not remembered. Field lists come from the
  `serde` structs, flags from the argument parser, paths from the module that builds them.
- **README.md is the front door and stays short**: what the tool is, one working example, install, and
  links out. `docs/INSTALL.md` covers install options; `docs/MORE.md` owns the command list and the
  `sk.toml` / `sk.lock` reference. When a command is added, `docs/MORE.md` is updated, not README.

Docs are edited in the same commit as the behaviour they describe, so a `make check` failure means the
transcript is stale.

## Page types

Every page declares a `type` in its frontmatter. The template lists the recommended sections for each:

- `debug` - Chasing a symptom to a root cause. Symptom -> steps -> root cause -> fix.
- `investigation` - Understanding how a subsystem works / where a gap is. No single fix.
- `code-review` - Findings against a diff or branch. Numbered findings + summary table.
- `implementation` - A set of changes that were made. Steps with before/after + test results.

## Frontmatter

Every page starts with YAML frontmatter (see `TEMPLATE.md`). This is what makes the wiki
machine-queryable — you can grep for a ticket, a service, or a tag across all pages.

- `title`, `date`, `type`, `status`, `session_id` - always set.
- `services`, `branch`, `tickets`, `tags` - set what applies; leave empty (`[]` / `-`) otherwise.
- `related` - filenames of pages this one links to. Keep it in sync with the `[[...]]` links in the body.

## Cross-linking

Link related pages inline with `[[filename-without-.md]]` (e.g. `[[2026-07-09-otel-ssl-cert-debug]]`)
and mirror them in the `related:` frontmatter list. Cross-links are what turn a folder of notes
into a knowledge graph - always link back to prior pages on the same subsystem.
