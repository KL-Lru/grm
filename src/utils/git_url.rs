use thiserror::Error;

/// Parsed repository information from a git URL
#[derive(Debug)]
pub struct RepoInfo {
    pub host: String,
    pub user: String,
    pub repo: String,
}

#[derive(Debug, Error)]
pub enum UrlError {
    #[error("Invalid git URL: {0}")]
    Invalid(String),
}

/// Parse a git URL (HTTPS or SSH) into components
///
/// Supports:
/// - `https://github.com/user/repo.git`
/// - `https://github.com/user/repo`
/// - `git@github.com:user/repo.git`
/// - `ssh://git@github.com/user/repo.git`
///
/// # Arguments
/// * `url` - Git repository URL to parse
///
/// # Returns
/// * `Ok(RepoInfo)` - Parsed repository information
/// * `Err(UrlError::InvalidUrl)` - If URL format is not supported
pub fn parse_git_url(url: &str) -> Result<RepoInfo, UrlError> {
    let url = url.trim();

    // (prefix, separator)
    let formats = [("https://", "/"), ("ssh://git@", "/"), ("git@", ":")];

    for (prefix, separator) in formats {
        if let Some(url_without_scheme) = url.strip_prefix(prefix) {
            let parts: Vec<&str> = url_without_scheme.splitn(2, separator).collect();
            if parts.len() != 2 {
                return Err(UrlError::Invalid(format!(
                    "Expected format: {prefix}host{separator}user/repo, got: {url}",
                )));
            }

            let host = parts[0];
            let path = parts[1];

            let path_parts: Vec<&str> = path.split('/').collect();
            if path_parts.len() < 2 {
                return Err(UrlError::Invalid(format!(
                    "Expected format: {prefix}host{separator}user/repo, got: {url}",
                )));
            }

            let user = path_parts[0];
            let repo = path_parts[1].trim_end_matches(".git");

            return Ok(RepoInfo {
                host: host.to_string(),
                user: user.to_string(),
                repo: repo.to_string(),
            });
        }
    }

    Err(UrlError::Invalid(format!(
        "Unsupported URL format. Supported: https://, git@, ssh://. Got: {url}",
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_git_url_https() {
        let info = parse_git_url("https://github.com/user/repo.git").unwrap();
        assert_eq!(info.host, "github.com");
        assert_eq!(info.user, "user");
        assert_eq!(info.repo, "repo");

        let info = parse_git_url("https://github.com/user/repo").unwrap();
        assert_eq!(info.host, "github.com");
        assert_eq!(info.user, "user");
        assert_eq!(info.repo, "repo");
    }

    #[test]
    fn test_parse_git_url_ssh() {
        let info = parse_git_url("git@github.com:user/repo.git").unwrap();
        assert_eq!(info.host, "github.com");
        assert_eq!(info.user, "user");
        assert_eq!(info.repo, "repo");

        let info = parse_git_url("git@gitlab.com:user/repo").unwrap();
        assert_eq!(info.host, "gitlab.com");
        assert_eq!(info.user, "user");
        assert_eq!(info.repo, "repo");
    }

    #[test]
    fn test_parse_git_url_ssh_protocol() {
        let info = parse_git_url("ssh://git@github.com/user/repo.git").unwrap();
        assert_eq!(info.host, "github.com");
        assert_eq!(info.user, "user");
        assert_eq!(info.repo, "repo");
    }

    #[test]
    fn test_parse_git_url_invalid() {
        assert!(parse_git_url("invalid").is_err());
        assert!(parse_git_url("https://github.com/user").is_err()); // Missing repo
        assert!(parse_git_url("git@github.com/user/repo.git").is_err()); // Wrong separator for short ssh
    }
}
