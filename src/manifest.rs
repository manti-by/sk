//! Parsing and generation of `sk.toml`.

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::github::RepoRef;
use crate::paths::Layout;
use crate::{bail, ensure};

/// Contents of `sk.toml`.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// Project metadata, mirroring the `[project]` table of `pyproject.toml`.
    #[serde(default)]
    pub project: Project,
    /// The skills this project depends on.
    #[serde(default)]
    pub skills: Skills,
}

/// The `[project]` table.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    /// Project name.
    pub name: Option<String>,
    /// Project version.
    pub version: Option<String>,
    /// One-line description.
    pub description: Option<String>,
    /// Path to the readme, relative to the manifest.
    pub readme: Option<String>,
    /// SPDX license identifier or license file.
    pub license: Option<String>,
    /// Authors, as `"Name"` or `{ name = "Name", email = "..." }`.
    pub authors: Option<Vec<Author>>,
    /// Keywords describing the project.
    pub keywords: Option<Vec<String>>,
    /// Project homepage.
    pub homepage: Option<String>,
    /// Repository URL.
    pub repository: Option<String>,
}

/// One entry of `project.authors`.
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Author {
    /// A bare name.
    Name(String),
    /// A name with an optional email address.
    Detailed {
        /// Author name.
        name: Option<String>,
        /// Author email.
        email: Option<String>,
    },
}

/// The `[skills]` table.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Skills {
    /// Links to skills, one per string.
    #[serde(default)]
    pub skills: Vec<String>,
}

/// A skill to install: where it comes from and what to call it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spec {
    /// Directory name under `.agents/skills`.
    pub name: String,
    /// Link as written in the manifest.
    pub source: String,
    /// Parsed form of `source`.
    pub repo: RepoRef,
}

impl Spec {
    /// Parses one manifest entry, which is either a bare link or
    /// `name = link` to install under a different directory name.
    pub fn parse(entry: &str) -> Result<Self> {
        let entry = entry.trim();
        ensure!(!entry.is_empty(), "empty entry in [skills] skills");

        // An explicit `name = link` rename; URLs may themselves contain `=`, so
        // the left side only counts as a name when it is not itself a link.
        let rename = entry
            .split_once('=')
            .filter(|(left, _)| !left.trim().contains("://"));
        let (name, source) = match rename {
            Some((left, right)) => {
                let name = left.trim();
                ensure!(
                    is_valid_name(name),
                    "`{name}` is not a usable directory name (use letters, digits, `.`, `_` and `-`)"
                );
                (name, right.trim())
            }
            None => ("", entry),
        };

        ensure!(!source.is_empty(), "skill `{entry}` has no link");

        let repo = RepoRef::parse(source)?;
        let name = if name.is_empty() {
            repo.dir_name().to_owned()
        } else {
            name.to_owned()
        };
        ensure!(
            is_valid_name(&name),
            "`{name}` is not a usable directory name (use letters, digits, `.`, `_` and `-`)"
        );

        Ok(Self {
            name,
            source: source.to_owned(),
            repo,
        })
    }
}

/// Characters allowed in a skill directory name: no separators, so a name can
/// never escape `.agents/skills`.
fn is_valid_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

impl Manifest {
    /// Reads `sk.toml` from the given layout.
    pub fn load(layout: &Layout) -> Result<Self> {
        let path = layout.manifest();
        let text = std::fs::read_to_string(&path).map_err(|source| {
            Error::with_source(
                format!(
                    "could not read {}; run `sk init` to create it",
                    path.display()
                ),
                source,
            )
        })?;

        let manifest: Self = toml::from_str(&text).map_err(|source| {
            Error::with_source(format!("{} is invalid", path.display()), source)
        })?;

        if let Some(name) = &manifest.project.name {
            ensure!(
                !name.trim().is_empty(),
                "project.name in {} must not be empty",
                path.display()
            );
        }
        Ok(manifest)
    }

    /// Parses every declared skill, rejecting duplicate directory names.
    pub fn specs(&self) -> Result<Vec<Spec>> {
        let mut specs = Vec::with_capacity(self.skills.skills.len());
        for entry in &self.skills.skills {
            let spec = Spec::parse(entry)
                .map_err(|error| Error::new(format!("in [skills] skills: {error}")))?;
            if let Some(previous) = specs.iter().find(|s: &&Spec| s.name == spec.name) {
                bail!(
                    "skills `{}` and `{}` both install as `{}`; rename one with `name = <link>`",
                    previous.source,
                    spec.source,
                    spec.name
                );
            }
            specs.push(spec);
        }
        Ok(specs)
    }

    /// Renders the manifest written by `sk init`.
    pub fn template(name: &str) -> String {
        format!(
            r#"[project]
name = "{name}"
version = "0.1.0"
description = ""
# readme = "README.md"
# license = "MIT"
# authors = ["Your Name"]
# keywords = []

# Each entry is a link to a directory containing a SKILL.md. The skill is
# installed under .agents/skills/<last path segment>/SKILL.md. Prefix an entry
# with "name = " to choose the directory name yourself.
[skills]
skills = [
  # "https://github.com/manti-by/openwiki/tree/master/skills/wiki-consistency",
  # "code-review = https://github.com/mattpocock/skills/tree/main/skills/engineering/code-review",
]
"#
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn specs(toml: &str) -> Result<Vec<Spec>> {
        toml::from_str::<Manifest>(toml)?.specs()
    }

    #[test]
    fn names_skills_after_the_last_url_segment() {
        let parsed = specs("[skills]\nskills = [\"https://github.com/manti-by/openwiki/tree/master/skills/wiki-consistency\"]").unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].name, "wiki-consistency");
        assert_eq!(parsed[0].repo.git_ref, "master");
    }

    #[test]
    fn explicit_name_overrides_the_url_segment() {
        let parsed = specs(
            "[skills]\nskills = [\"review = https://github.com/mattpocock/skills/tree/main/skills/engineering/code-review\"]",
        )
        .unwrap();
        assert_eq!(parsed[0].name, "review");
    }

    #[test]
    fn keeps_query_strings_in_urls() {
        let parsed =
            specs("[skills]\nskills = [\"https://github.com/o/r/tree/main/s?a=b\"]").unwrap();
        assert_eq!(parsed[0].name, "s");
    }

    #[test]
    fn rejects_duplicate_names_and_bad_links() {
        let duplicate = "[skills]\nskills = [\n  \"https://github.com/o/r/tree/main/a\",\n  \"b = https://github.com/o/r/tree/main/b\",\n  \"c = https://github.com/o/r2/tree/main/c\",\n]";
        assert!(
            specs(&duplicate.replace("\"b = ", "\"a = "))
                .unwrap_err()
                .to_string()
                .contains("install as")
        );

        assert!(specs("[skills]\nskills = [\"https://gitlab.com/o/r\"]").is_err());
        assert!(specs("[skills]\nskills = [\"\"]").is_err());
    }

    #[test]
    fn rejects_names_that_would_escape_the_skills_directory() {
        for entry in [
            "../../etc = https://github.com/o/r/tree/main/s",
            "../evil = https://github.com/o/r/tree/main/s",
            ".hidden = https://github.com/o/r/tree/main/s",
            "has space = https://github.com/o/r/tree/main/s",
        ] {
            let error = Spec::parse(entry).unwrap_err().to_string();
            assert!(
                error.contains("not a usable directory name"),
                "unexpected error for {entry}: {error}"
            );
        }
        assert!(!is_valid_name("../../etc"));
    }

    #[test]
    fn url_containing_an_equals_sign_is_still_a_link() {
        let parsed = Spec::parse("https://github.com/o/r/tree/main/s?a=b=c").unwrap();
        assert_eq!(parsed.name, "s");
    }

    #[test]
    fn accepts_both_author_forms() {
        let manifest: Manifest = toml::from_str(
            "[project]\nname = \"p\"\nauthors = [\"Ada\", { name = \"Grace\", email = \"g@example.com\" }]",
        )
        .unwrap();
        let authors = manifest.project.authors.unwrap();
        assert!(matches!(&authors[0], Author::Name(name) if name == "Ada"));
        assert!(matches!(&authors[1], Author::Detailed { name: Some(n), .. } if n == "Grace"));
    }

    #[test]
    fn template_round_trips() {
        let manifest: Manifest = toml::from_str(&Manifest::template("demo")).unwrap();
        assert_eq!(manifest.project.name.as_deref(), Some("demo"));
        assert!(manifest.specs().unwrap().is_empty());
    }

    #[test]
    fn rejects_unknown_fields() {
        assert!(toml::from_str::<Manifest>("[project]\nauthors = []\nnmae = \"typo\"\n").is_err());
    }
}
