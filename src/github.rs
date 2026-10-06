//! Parsing of GitHub skill links and downloading of `SKILL.md` files.

use std::time::Duration;

use crate::error::{Error, Result};
use crate::paths::SKILL_FILE;
use crate::{bail, ensure};

/// API endpoint used to resolve a ref into a commit.
const API: &str = "https://api.github.com";
/// Host serving raw file content.
const RAW: &str = "https://raw.githubusercontent.com";

/// Largest `SKILL.md` we are willing to download, as a sanity bound.
const MAX_SKILL_BYTES: u64 = 4 * 1024 * 1024;

/// A GitHub link pointing at a directory that holds a `SKILL.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoRef {
    /// Repository owner, the user or organisation.
    pub owner: String,
    /// Repository name.
    pub repo: String,
    /// Branch, tag or commit as written in the link.
    pub git_ref: String,
    /// Path of the skill directory inside the repository, without leading or
    /// trailing slashes. Empty when the skill sits at the repository root.
    pub path: String,
}

impl RepoRef {
    /// Parses a GitHub link such as
    /// `https://github.com/owner/repo/tree/main/skills/engineering/code-review`.
    ///
    /// `HEAD` is used as the ref when the link points at the repository root, so
    /// the default branch is resolved.
    pub fn parse(url: &str) -> Result<Self> {
        let url = url.trim();
        let rest = url
            .strip_prefix("https://github.com/")
            .or_else(|| url.strip_prefix("http://github.com/"))
            .ok_or_else(|| {
                Error::new(format!(
                    "`{url}` is not a GitHub link (expected https://github.com/<owner>/<repo>/...)"
                ))
            })?;

        // Query strings and fragments are never part of the path.
        let rest = rest.split(['?', '#']).next().unwrap_or(rest);
        let mut segments = rest.split('/').map(str::trim).filter(|s| !s.is_empty());

        let owner = take(&mut segments, url, "owner")?;
        let repo = take(&mut segments, url, "repository name")?
            .trim_end_matches(".git")
            .to_owned();
        ensure!(!repo.is_empty(), "`{url}` has an empty repository name");

        let mut git_ref = "HEAD".to_owned();
        let mut path: Vec<&str> = Vec::new();

        if let Some(kind) = segments.next() {
            match kind {
                "tree" | "blob" => {
                    git_ref = take(&mut segments, url, "branch, tag or commit")?.to_owned();
                    path.extend(segments);
                    // A `blob` link may point straight at the file itself.
                    if kind == "blob" && path.last() == Some(&SKILL_FILE) {
                        path.pop();
                    }
                }
                other => bail!(
                    "`{url}` uses the unsupported GitHub path `{other}`; expected `/tree/<ref>/...`"
                ),
            }
        }

        Ok(Self {
            owner: owner.to_owned(),
            repo,
            git_ref,
            path: path.join("/"),
        })
    }

    /// Directory name the skill is installed as: the last path segment, or the
    /// repository name for skills living at the repository root.
    pub fn dir_name(&self) -> &str {
        self.path
            .rsplit('/')
            .find(|segment| !segment.is_empty())
            .unwrap_or(&self.repo)
    }

    /// URL of `SKILL.md` for an exact commit.
    pub fn raw_url(&self, commit: &str) -> String {
        let mut url = format!("{RAW}/{}/{}/{commit}", self.owner, self.repo);
        if !self.path.is_empty() {
            url.push('/');
            url.push_str(&self.path);
        }
        url.push('/');
        url.push_str(SKILL_FILE);
        url
    }

    /// Human-readable location, used in error messages.
    pub fn describe(&self) -> String {
        if self.path.is_empty() {
            format!("{}/{}", self.owner, self.repo)
        } else {
            format!("{}/{}/{}", self.owner, self.repo, self.path)
        }
    }
}

fn take<'a>(
    segments: &mut impl Iterator<Item = &'a str>,
    url: &str,
    what: &str,
) -> Result<&'a str> {
    segments
        .next()
        .ok_or_else(|| Error::new(format!("`{url}` is missing the {what}")))
}

/// HTTP client for the GitHub API and raw content.
pub struct Client {
    agent: ureq::Agent,
    token: Option<String>,
}

impl Client {
    /// Builds a client, using `GITHUB_TOKEN` or `GH_TOKEN` when available.
    pub fn from_env() -> Self {
        let token = ["GITHUB_TOKEN", "GH_TOKEN"]
            .into_iter()
            .find_map(|name| std::env::var(name).ok().filter(|t| !t.trim().is_empty()));

        let config = ureq::Agent::config_builder()
            .user_agent(concat!("sk/", env!("CARGO_PKG_VERSION")))
            .timeout_global(Some(Duration::from_secs(30)))
            .build();

        Self {
            agent: config.into(),
            token,
        }
    }

    /// Resolves a branch, tag or commit into a full commit SHA.
    pub fn resolve_commit(&self, repo: &RepoRef) -> Result<String> {
        let url = format!(
            "{API}/repos/{}/{}/commits/{}",
            repo.owner, repo.repo, repo.git_ref
        );
        let body = self.get_json(&url)?;

        serde_json::from_slice::<serde_json::Value>(&body)
            .ok()
            .and_then(|value| value.get("sha")?.as_str().map(str::to_owned))
            .ok_or_else(|| {
                Error::new(format!(
                    "GitHub did not return a commit for {} (ref `{}`)",
                    repo.describe(),
                    repo.git_ref
                ))
            })
    }

    /// Downloads `SKILL.md` at an exact commit.
    pub fn download(&self, repo: &RepoRef, commit: &str) -> Result<Vec<u8>> {
        let url = repo.raw_url(commit);
        match self.agent.get(&url).call() {
            Ok(mut response) => response
                .body_mut()
                .with_config()
                .limit(MAX_SKILL_BYTES)
                .read_to_vec()
                .map_err(|source| Error::with_source(format!("could not read {url}"), source)),
            Err(ureq::Error::StatusCode(404)) => Err(Error::new(format!(
                "no {SKILL_FILE} in {} at commit {commit}",
                repo.describe()
            ))),
            Err(source) => Err(Error::with_source(format!("GET {url}"), source)),
        }
    }

    /// Performs a GET against the API, mapping error statuses to advice.
    fn get_json(&self, url: &str) -> Result<Vec<u8>> {
        let mut request = self
            .agent
            .get(url)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28");
        if let Some(token) = &self.token {
            request = request.header("Authorization", format!("Bearer {token}"));
        }

        match request.call() {
            Ok(mut response) => response
                .body_mut()
                .with_config()
                .limit(MAX_SKILL_BYTES)
                .read_to_vec()
                .map_err(|source| Error::with_source(format!("could not read {url}"), source)),
            // The commits endpoint answers 422, not 404, for an unknown ref.
            Err(ureq::Error::StatusCode(code)) if code == 404 || code == 422 => {
                Err(Error::new(format!("GitHub returned {code} for {url}")))
            }
            Err(ureq::Error::StatusCode(403 | 429)) => Err(Error::new(format!(
                "GitHub rate limit reached ({url}); set GITHUB_TOKEN to raise it"
            ))),
            Err(source) => Err(Error::with_source(format!("GET {url}"), source)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(url: &str) -> RepoRef {
        RepoRef::parse(url).expect("valid link")
    }

    #[test]
    fn parses_tree_link() {
        let repo =
            parse("https://github.com/mattpocock/skills/tree/main/skills/engineering/code-review");
        assert_eq!(repo.owner, "mattpocock");
        assert_eq!(repo.repo, "skills");
        assert_eq!(repo.git_ref, "main");
        assert_eq!(repo.path, "skills/engineering/code-review");
        assert_eq!(repo.dir_name(), "code-review");
    }

    #[test]
    fn parses_root_link_as_default_branch() {
        let repo = parse("https://github.com/owner/repo");
        assert_eq!(repo.git_ref, "HEAD");
        assert_eq!(repo.path, "");
        assert_eq!(repo.dir_name(), "repo");
    }

    #[test]
    fn parses_repo_link_with_git_suffix_and_slash() {
        let repo = parse("https://github.com/owner/repo.git/");
        assert_eq!(repo.repo, "repo");
        assert_eq!(repo.path, "");
    }

    #[test]
    fn blob_link_points_at_the_containing_directory() {
        let repo = parse(
            "https://github.com/manti-by/openwiki/blob/master/skills/wiki-consistency/SKILL.md",
        );
        assert_eq!(repo.git_ref, "master");
        assert_eq!(repo.path, "skills/wiki-consistency");
        assert_eq!(repo.dir_name(), "wiki-consistency");
    }

    #[test]
    fn ignores_query_and_fragment() {
        let repo = parse("https://github.com/owner/repo/tree/main/skills/demo?ref=x#top");
        assert_eq!(repo.path, "skills/demo");
        assert_eq!(repo.dir_name(), "demo");
    }

    #[test]
    fn rejects_other_hosts_and_shapes() {
        assert!(RepoRef::parse("https://gitlab.com/owner/repo").is_err());
        assert!(RepoRef::parse("https://github.com/owner/repo/pulls/1").is_err());
        assert!(RepoRef::parse("https://github.com/owner").is_err());
        assert!(RepoRef::parse("https://github.com/owner/repo/tree").is_err());
        assert!(RepoRef::parse("").is_err());
    }

    #[test]
    fn builds_raw_url_pinned_to_commit() {
        let repo =
            parse("https://github.com/mattpocock/skills/tree/main/skills/engineering/code-review");
        assert_eq!(
            repo.raw_url("abc123"),
            "https://raw.githubusercontent.com/mattpocock/skills/abc123/skills/engineering/code-review/SKILL.md"
        );

        let root = parse("https://github.com/owner/repo");
        assert_eq!(
            root.raw_url("abc123"),
            "https://raw.githubusercontent.com/owner/repo/abc123/SKILL.md"
        );
    }
}
