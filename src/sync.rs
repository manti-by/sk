//! `sk sync`: resolve, download and install every declared skill.

use std::collections::BTreeSet;

use crate::error::{Error, Result};
use crate::github::Client;
use crate::lock::{Entry, LOCK_VERSION, Lock};
use crate::manifest::{Manifest, Spec};
use crate::parallel;
use crate::paths::{Layout, SKILL_FILE, SKILLS_DIR};
use crate::util::write_atomic;

/// What happened to one skill during a sync.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    /// The skill was not on disk before.
    Added,
    /// The downloaded content differed from what was on disk.
    Updated,
    /// The content already matched.
    Unchanged,
}

impl Status {
    /// Label used in the report.
    fn label(self) -> &'static str {
        match self {
            Status::Added => "added",
            Status::Updated => "updated",
            Status::Unchanged => "unchanged",
        }
    }
}

/// One skill after downloading and installing it.
#[derive(Debug)]
struct Installed {
    /// What changed on disk.
    status: Status,
    /// Commit the content came from, recorded in `sk.lock`.
    commit: String,
}

/// Downloads and installs all skills, then refreshes `sk.lock`.
pub fn run(layout: &Layout, upgrade: bool) -> Result<()> {
    let manifest = Manifest::load(layout)?;
    let lock = Lock::load(layout)?;
    let specs = manifest.specs()?;

    if specs.is_empty() {
        println!(
            "No skills in {}; add links under [skills] skills.",
            layout.manifest().display()
        );
        return Ok(());
    }

    let client = Client::from_env();
    println!("Syncing {} skill(s) into {SKILLS_DIR}/", specs.len());

    let results = parallel::map(specs.clone(), |_, spec| {
        install(&client, layout, &lock, &spec, upgrade)
    });

    let mut entries = Vec::with_capacity(specs.len());
    let mut failures = Vec::new();

    for (spec, result) in specs.iter().zip(results) {
        match result {
            Ok(installed) => {
                println!(
                    "  {:<9} {}/{SKILL_FILE}",
                    installed.status.label(),
                    spec.name
                );
                entries.push(Entry {
                    name: spec.name.clone(),
                    source: spec.source.clone(),
                    commit: installed.commit,
                });
            }
            Err(error) => failures.push(format!("{}: {error}", spec.name)),
        }
    }

    for (name, reason) in prune(layout, &lock, &specs)? {
        println!("  removed   {name} ({reason})");
    }
    for failure in &failures {
        eprintln!("  failed    {failure}");
    }

    // Keep the skills that synced; failures are retried on the next run.
    Lock {
        version: LOCK_VERSION,
        skills: entries,
    }
    .save(layout)?;

    if failures.is_empty() {
        println!("Wrote {}", layout.lock().display());
        Ok(())
    } else {
        Err(Error::new(format!(
            "{} of {} skill(s) failed to sync",
            failures.len(),
            specs.len()
        )))
    }
}

/// Fetches one skill and writes it, reporting what changed.
fn install(
    client: &Client,
    layout: &Layout,
    lock: &Lock,
    spec: &Spec,
    upgrade: bool,
) -> Result<Installed> {
    // A locked commit keeps installs reproducible; --upgrade re-resolves refs
    // so a moved branch is picked up.
    let commit = match (upgrade, lock.commit_for(&spec.source)) {
        (false, Some(commit)) => commit.to_owned(),
        _ => client.resolve_commit(&spec.repo)?,
    };

    let contents = client.download(&spec.repo, &commit)?;
    let target = layout.skill_file(&spec.name);

    let status = match std::fs::read(&target) {
        Ok(existing) if existing == contents => Status::Unchanged,
        Ok(_) => Status::Updated,
        Err(_) => Status::Added,
    };
    if status != Status::Unchanged {
        write_atomic(&target, &contents)?;
    }

    Ok(Installed { status, commit })
}

/// Removes skills `sk` installed earlier that are no longer declared.
///
/// Only directories recorded in `sk.lock` are considered, so hand-made skills
/// living next to managed ones are never touched.
fn prune(layout: &Layout, lock: &Lock, specs: &[Spec]) -> Result<Vec<(String, String)>> {
    let declared: BTreeSet<&str> = specs.iter().map(|spec| spec.name.as_str()).collect();
    let previously_managed: BTreeSet<&str> = lock.managed_names().collect();
    let skills_dir = layout.skills_dir();

    let entries = match std::fs::read_dir(&skills_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => {
            return Err(Error::with_source(
                format!("could not read {}", skills_dir.display()),
                source,
            ));
        }
    };

    let mut removed = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|source| {
            Error::with_source(format!("could not read {}", skills_dir.display()), source)
        })?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let dir = entry.path();

        let is_stale =
            !declared.contains(name.as_str()) && previously_managed.contains(name.as_str());
        if !is_stale || !dir.is_dir() || !dir.join(SKILL_FILE).exists() {
            continue;
        }

        std::fs::remove_dir_all(&dir).map_err(|source| {
            Error::with_source(format!("could not remove {}", dir.display()), source)
        })?;
        removed.push((name, "no longer in sk.toml".to_owned()));
    }

    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::Spec;

    fn layout(name: &str) -> Layout {
        let root = std::env::temp_dir().join(format!("sk-sync-{name}-{}", std::process::id()));
        std::fs::remove_dir_all(&root).ok();
        Layout::new(root)
    }

    fn lock_of(names: &[&str]) -> Lock {
        Lock {
            version: LOCK_VERSION,
            skills: names
                .iter()
                .map(|name| Entry {
                    name: (*name).to_owned(),
                    source: format!("https://github.com/o/r/tree/main/{name}"),
                    commit: "abc123".to_owned(),
                })
                .collect(),
        }
    }

    fn spec_of(name: &str) -> Spec {
        Spec::parse(&format!("https://github.com/o/r/tree/main/{name}")).unwrap()
    }

    #[test]
    fn prune_removes_only_locked_and_undeclared_skills() {
        let layout = layout("prune");
        for name in ["stale", "kept", "manual"] {
            std::fs::create_dir_all(layout.skill_dir(name)).unwrap();
            std::fs::write(layout.skill_file(name), b"skill").unwrap();
        }

        // `manual` was never installed by sk, so it is not in the lock.
        let lock = lock_of(&["stale", "kept"]);
        let removed = prune(&layout, &lock, &[spec_of("kept")]).unwrap();

        assert_eq!(
            removed,
            vec![("stale".to_owned(), "no longer in sk.toml".to_owned())]
        );
        assert!(!layout.skill_dir("stale").exists());
        assert!(layout.skill_file("kept").exists());
        // Never in the lock, so never removed.
        assert!(layout.skill_file("manual").exists());
        std::fs::remove_dir_all(&layout.root).ok();
    }

    #[test]
    fn prune_ignores_files_and_dirs_without_a_skill_file() {
        let layout = layout("prune-odd");
        std::fs::create_dir_all(layout.skill_dir("empty")).unwrap();
        std::fs::write(layout.skills_dir().join("notes.md"), b"hi").unwrap();

        let lock = lock_of(&["empty", "notes.md"]);
        assert!(prune(&layout, &lock, &[]).unwrap().is_empty());
        assert!(layout.skill_dir("empty").exists());
        assert!(layout.skills_dir().join("notes.md").exists());
        std::fs::remove_dir_all(&layout.root).ok();
    }

    #[test]
    fn prune_is_a_no_op_when_the_directory_is_missing() {
        let layout = layout("prune-missing");
        assert!(prune(&layout, &lock_of(&["a"]), &[]).unwrap().is_empty());
    }
}
