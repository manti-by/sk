//! `sk init`: create the project scaffolding.

use crate::lock::{LOCK_VERSION, Lock};
use crate::manifest::Manifest;
use crate::paths::Layout;
use crate::util::write_atomic;

/// Creates the skills directory of the configured agent, `sk.toml` and
/// `sk.lock`, leaving any file that already exists untouched.
///
/// The manifest is written first, so the directory created is the one the
/// `agent` setting asks for even on a fresh project.
pub fn run(layout: &Layout) -> crate::error::Result<()> {
    let mut created = Vec::new();
    let mut kept = Vec::new();

    let manifest_path = layout.manifest();
    if manifest_path.exists() {
        kept.push(manifest_path.display().to_string());
    } else {
        write_atomic(
            &manifest_path,
            Manifest::template(&default_project_name(&layout.root)).as_bytes(),
        )?;
        created.push(manifest_path.display().to_string());
    }

    let layout = &layout.with_agent(Manifest::load(layout)?.agent());
    let skills_dir = layout.skills_dir();
    if skills_dir.exists() {
        kept.push(skills_dir.display().to_string());
    } else {
        std::fs::create_dir_all(&skills_dir).map_err(|source| {
            crate::error::Error::with_source(
                format!("could not create {}", skills_dir.display()),
                source,
            )
        })?;
        created.push(format!("{}/", skills_dir.display()));
    }

    let lock_path = layout.lock();
    if lock_path.exists() {
        kept.push(lock_path.display().to_string());
    } else {
        Lock {
            version: LOCK_VERSION,
            skills: Vec::new(),
        }
        .save(layout)?;
        created.push(lock_path.display().to_string());
    }

    for path in created {
        println!("created  {path}");
    }
    for path in kept {
        println!("exists   {path} (left unchanged)");
    }

    let manifest = Manifest::load(layout)?;
    if manifest.dependencies.skills.is_empty() {
        println!(
            "\nAdd skill links under [dependencies] skills in {}, then run `sk sync`.",
            manifest_path.display()
        );
    }
    Ok(())
}

/// Derives a project name from the project directory.
fn default_project_name(root: &std::path::Path) -> String {
    root.canonicalize()
        .ok()
        .as_deref()
        .and_then(std::path::Path::file_name)
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "project".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout(name: &str) -> Layout {
        let root = std::env::temp_dir().join(format!("sk-init-{name}-{}", std::process::id()));
        std::fs::remove_dir_all(&root).ok();
        std::fs::create_dir_all(&root).unwrap();
        Layout::new(root)
    }

    #[test]
    fn creates_all_three_entries() {
        let layout = layout("create");
        run(&layout).unwrap();

        assert!(layout.skills_dir().is_dir());
        assert!(layout.manifest().is_file());
        assert!(layout.lock().is_file());
        assert_eq!(Lock::load(&layout).unwrap().version, LOCK_VERSION);
        std::fs::remove_dir_all(&layout.root).ok();
    }

    #[test]
    fn leaves_existing_files_alone() {
        let layout = layout("existing");
        std::fs::create_dir_all(layout.skills_dir()).unwrap();
        std::fs::write(layout.manifest(), "[project]\nname = \"mine\"\n").unwrap();
        std::fs::write(layout.lock(), "version = 1\n").unwrap();

        run(&layout).unwrap();

        assert_eq!(
            std::fs::read_to_string(layout.manifest()).unwrap(),
            "[project]\nname = \"mine\"\n"
        );
        assert_eq!(
            std::fs::read_to_string(layout.lock()).unwrap(),
            "version = 1\n"
        );
        std::fs::remove_dir_all(&layout.root).ok();
    }

    #[test]
    fn creates_the_directory_of_the_configured_agent() {
        for (agent, dir) in [
            ("agents", ".agents"),
            ("claude", ".claude"),
            ("opencode", ".opencode"),
        ] {
            let layout = layout(&format!("agent-{agent}"));
            std::fs::write(
                layout.manifest(),
                format!("[project]\nname = \"p\"\nagent = \"{agent}\"\n"),
            )
            .unwrap();

            run(&layout).unwrap();

            assert!(layout.root.join(dir).join("skills").is_dir());
            assert!(layout.manifest().is_file());
            assert!(layout.lock().is_file());
            std::fs::remove_dir_all(&layout.root).ok();
        }
    }

    #[test]
    fn names_the_project_after_its_directory() {
        let layout = layout("demo-project");
        run(&layout).unwrap();
        let manifest = Manifest::load(&layout).unwrap();
        let expected = layout
            .root
            .canonicalize()
            .unwrap()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        assert_eq!(manifest.project.name.as_deref(), Some(expected.as_str()));
        assert!(expected.contains("demo-project"));
        std::fs::remove_dir_all(&layout.root).ok();
    }
}
