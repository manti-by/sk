//! Well-known file and directory names, and how they combine into a layout.

use std::path::{Path, PathBuf};

/// Directory holding the downloaded skills, relative to the project root.
pub const SKILLS_DIR: &str = ".agents/skills";

/// The manifest file, relative to the project root.
pub const MANIFEST: &str = "sk.toml";

/// The lock file, relative to the project root.
pub const LOCK: &str = "sk.lock";

/// The single file `sk` downloads for every skill.
pub const SKILL_FILE: &str = "SKILL.md";

/// Paths of one project, all derived from its root directory.
#[derive(Debug, Clone)]
pub struct Layout {
    /// Root of the project.
    pub root: PathBuf,
}

impl Layout {
    /// Builds a layout rooted at `root`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Path of `sk.toml`.
    pub fn manifest(&self) -> PathBuf {
        self.root.join(MANIFEST)
    }

    /// Path of `sk.lock`.
    pub fn lock(&self) -> PathBuf {
        self.root.join(LOCK)
    }

    /// Path of `.agents/skills`.
    pub fn skills_dir(&self) -> PathBuf {
        self.root.join(SKILLS_DIR)
    }

    /// Path of the skill directory for `name`, i.e. `.agents/skills/<name>`.
    pub fn skill_dir(&self, name: &str) -> PathBuf {
        self.skills_dir().join(name)
    }

    /// Path of `.agents/skills/<name>/SKILL.md`.
    pub fn skill_file(&self, name: &str) -> PathBuf {
        self.skill_dir(name).join(SKILL_FILE)
    }
}

impl From<&Path> for Layout {
    fn from(root: &Path) -> Self {
        Self::new(root)
    }
}
