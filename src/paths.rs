//! Well-known file and directory names, and how they combine into a layout.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Directory holding the downloaded skills of the default agent, relative to
/// the project root.
pub const SKILLS_DIR: &str = ".agents/skills";

/// The manifest file, relative to the project root.
pub const MANIFEST: &str = "sk.toml";

/// The lock file, relative to the project root.
pub const LOCK: &str = "sk.lock";

/// The single file `sk` downloads for every skill.
pub const SKILL_FILE: &str = "SKILL.md";

/// The agent the skills are installed for, chosen by `project.agent`.
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

/// Paths of one project, all derived from its root directory.
#[derive(Debug, Clone)]
pub struct Layout {
    /// Root of the project.
    pub root: PathBuf,
    /// Directory the skills are installed into.
    pub agent: Agent,
}

impl Layout {
    /// Builds a layout rooted at `root`, for the default agent.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            agent: Agent::default(),
        }
    }

    /// The same layout, pointing at the skills directory of `agent`.
    pub fn with_agent(&self, agent: Agent) -> Self {
        Self {
            root: self.root.clone(),
            agent,
        }
    }

    /// Path of `sk.toml`.
    pub fn manifest(&self) -> PathBuf {
        self.root.join(MANIFEST)
    }

    /// Path of `sk.lock`.
    pub fn lock(&self) -> PathBuf {
        self.root.join(LOCK)
    }

    /// Path of the skills directory of the configured agent, e.g.
    /// `.agents/skills`.
    pub fn skills_dir(&self) -> PathBuf {
        self.root.join(self.agent.skills_dir())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_agent_has_its_own_skills_directory() {
        assert_eq!(Agent::default(), Agent::Agents);
        assert_eq!(Agent::Agents.skills_dir(), ".agents/skills");
        assert_eq!(Agent::Claude.skills_dir(), ".claude/skills");
        assert_eq!(Agent::OpenCode.skills_dir(), ".opencode/skills");
    }

    #[test]
    fn agent_parses_from_the_manifest_spelling() {
        #[derive(Deserialize)]
        struct Holder {
            agent: Agent,
        }

        for (text, expected) in [
            ("agents", Agent::Agents),
            ("claude", Agent::Claude),
            ("opencode", Agent::OpenCode),
        ] {
            let parsed: Holder = toml::from_str(&format!("agent = \"{text}\"")).unwrap();
            assert_eq!(parsed.agent, expected);
        }
        assert!(toml::from_str::<Holder>("agent = \"cursor\"").is_err());
    }

    #[test]
    fn skills_paths_follow_the_agent() {
        let root = Path::new("/p");
        let layout = Layout::from(root);
        assert_eq!(layout.skills_dir(), Path::new("/p/.agents/skills"));
        assert_eq!(
            layout.skill_file("s"),
            Path::new("/p/.agents/skills/s/SKILL.md")
        );

        let claude = layout.with_agent(Agent::Claude);
        assert_eq!(
            claude.skill_file("s"),
            Path::new("/p/.claude/skills/s/SKILL.md")
        );
        // `with_agent` leaves the original layout alone.
        assert_eq!(layout.agent, Agent::Agents);
    }
}
