use crate::configs::Config;
use crate::errors::GrmError;
use crate::utils::git;
use crate::utils::git_url::parse_git_url;
use crate::utils::path::build_repo_path;

/// Execute the clone command
pub fn execute(url: &str, branch: Option<&str>) -> Result<(), GrmError> {
    let repo_info = parse_git_url(url)?;

    let branch_name = if let Some(b) = branch {
        b.to_string()
    } else {
        git::get_default_branch(url)?
    };

    let config = Config::load()?;
    let dest_path = build_repo_path(config.root(), &repo_info, &branch_name);

    if dest_path.exists() {
        return Err(GrmError::AlreadyExists(dest_path.display().to_string()));
    }

    if let Some(parent) = dest_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    git::clone_repo(url, &dest_path, Some(&branch_name))?;

    println!("Repository cloned to: {}", dest_path.display());

    Ok(())
}
