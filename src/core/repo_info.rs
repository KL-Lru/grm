use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoInfo {
    pub host: String,
    pub user: String,
    pub repo: String,
}

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("Invalid: {0}")]
    Invalid(String),
}

impl RepoInfo {
    pub fn new(host: String, user: String, repo: String) -> Self {
        Self { host, user, repo }
    }

    /// Parse a git repository URL into ``RepoInfo``
    ///
    /// examples of supported URL formats:
    /// - <https://host/user/repo.git>
    /// - <ssh://git@host/user/repo.git>
    /// - <git@host:user/repo.git>
    pub fn from_url(url: &str) -> Result<Self, RepositoryError> {
        let url = url.trim();

        let formats = [("https://", "/"), ("ssh://git@", "/"), ("git@", ":")];

        for (prefix, separator) in formats {
            if let Some(url_without_scheme) = url.strip_prefix(prefix) {
                let parts: Vec<&str> = url_without_scheme.splitn(2, separator).collect();
                if parts.len() != 2 {
                    return Err(RepositoryError::Invalid(format!(
                        "Expected format: {prefix}host{separator}user/repo, got: {url}",
                    )));
                }

                let host = parts[0];
                let path = parts[1];

                let path_parts: Vec<&str> = path.split('/').collect();
                if path_parts.len() < 2 {
                    return Err(RepositoryError::Invalid(format!(
                        "Expected format: {prefix}host{separator}user/repo, got: {url}",
                    )));
                }

                let user = path_parts[0];
                let repo = path_parts[1].trim_end_matches(".git");

                return Ok(RepoInfo::new(
                    host.to_string(),
                    user.to_string(),
                    repo.to_string(),
                ));
            }
        }

        Err(RepositoryError::Invalid(format!(
            "Unsupported URL format. Supported: https://, git@, ssh://. Got: {url}",
        )))
    }

    /// Constructs a `RepoInfo` from a given path relative to the root directory.
    ///
    /// examples of supported path formats:
    /// - `{root}/{host}/{user}/{repo}+{branch}`
    pub fn from_path(root: &Path, path: &Path) -> Result<Self, RepositoryError> {
        let relative_path = path.strip_prefix(root).map_err(|_| {
            RepositoryError::Invalid(format!(
                "Path {} is not under root {}",
                path.display(),
                root.display()
            ))
        })?;

        let components: Vec<&str> = relative_path
            .components()
            .map(|comp| comp.as_os_str().to_str().unwrap_or(""))
            .collect();

        if components.len() < 3 {
            return Err(RepositoryError::Invalid(format!(
                "Path {} does not have managed repository structure",
                relative_path.display(),
            )));
        }

        let host = components[0].to_string();
        let user = components[1].to_string();
        let repo_with_branch = components[2];
        let repo = repo_with_branch
            .split('+')
            .next()
            .ok_or_else(|| {
                RepositoryError::Invalid(format!(
                    "Repository component {repo_with_branch} is not valid",
                ))
            })?
            .to_string();

        Ok(RepoInfo::new(host, user, repo))
    }

    /// Builds the repository path
    ///
    /// # Arguments
    /// * `root` - The root directory for managed repositories
    /// * `branch` - The branch name
    ///
    /// # Returns
    /// Path in the format: `{root}/{host}/{user}/{repo}+{branch}`
    pub fn build_repo_path(&self, root: &Path, branch: &str) -> PathBuf {
        root.join(&self.host)
            .join(&self.user)
            .join(format!("{}+{}", self.repo, branch))
    }

    /// Builds the shared file path
    ///
    /// # Arguments
    /// * `root` - The root directory for managed repositories
    /// * `relative_path` - The relative path within the repository
    ///
    /// # Returns
    /// Path in the format: `{root}/.shared/{host}/{user}/{repo}/{relative_path}`
    pub fn build_shared_path(&self, root: &Path, relative_path: &Path) -> PathBuf {
        root.join(".shared")
            .join(&self.host)
            .join(&self.user)
            .join(&self.repo)
            .join(relative_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_url_https() {
        let info = RepoInfo::from_url("https://github.com/user/repo.git").unwrap();
        assert_eq!(info.host, "github.com");
        assert_eq!(info.user, "user");
        assert_eq!(info.repo, "repo");

        let info = RepoInfo::from_url("https://github.com/user/repo").unwrap();
        assert_eq!(info.host, "github.com");
        assert_eq!(info.user, "user");
        assert_eq!(info.repo, "repo");
    }

    #[test]
    fn test_from_url_ssh() {
        let info = RepoInfo::from_url("git@github.com:user/repo.git").unwrap();
        assert_eq!(info.host, "github.com");
        assert_eq!(info.user, "user");
        assert_eq!(info.repo, "repo");

        let info = RepoInfo::from_url("git@gitlab.com:user/repo").unwrap();
        assert_eq!(info.host, "gitlab.com");
        assert_eq!(info.user, "user");
        assert_eq!(info.repo, "repo");
    }

    #[test]
    fn test_from_url_ssh_protocol() {
        let info = RepoInfo::from_url("ssh://git@github.com/user/repo.git").unwrap();
        assert_eq!(info.host, "github.com");
        assert_eq!(info.user, "user");
        assert_eq!(info.repo, "repo");
    }

    #[test]
    fn test_from_url_invalid() {
        assert!(RepoInfo::from_url("invalid").is_err());
        assert!(RepoInfo::from_url("https://github.com/user").is_err());
        assert!(RepoInfo::from_url("git@github.com/user/repo.git").is_err());
    }

    #[test]
    fn test_build_repo_path() {
        let info = RepoInfo::new(
            "github.com".to_string(),
            "test".to_string(),
            "repo".to_string(),
        );
        let root = PathBuf::from("/home/user/grm");
        let path = info.build_repo_path(&root, "main");
        assert_eq!(
            path,
            PathBuf::from("/home/user/grm/github.com/test/repo+main")
        );
    }

    #[test]
    fn test_build_shared_path() {
        let info = RepoInfo::new(
            "github.com".to_string(),
            "test".to_string(),
            "repo".to_string(),
        );
        let root = PathBuf::from("/home/user/grm");
        let path = info.build_shared_path(&root, Path::new(".env"));
        assert_eq!(
            path,
            PathBuf::from("/home/user/grm/.shared/github.com/test/repo/.env")
        );
    }
}
