use std::path::{Path, PathBuf};

use crate::configs::Config;
use crate::errors::GrmError;
use crate::utils::git;
use crate::utils::git_url::parse_git_url;
use crate::utils::path::{build_shared_path, is_symlink};

/// Execute the worktree isolate command
///
/// Replaces a shared symbolic link with a concrete copy of the file/directory.
pub fn execute(path_str: &str) -> Result<(), GrmError> {
    let relative_path = PathBuf::from(path_str);

    let repo_root = git::get_repo_root().map_err(|_| GrmError::NotInManagedRepository)?;
    let remote_url =
        git::get_remote_url(&repo_root).map_err(|_| GrmError::NotInManagedRepository)?;
    let repo_info = parse_git_url(&remote_url)?;

    let config = Config::load()?;
    let shared_path = build_shared_path(config.root(), &repo_info, &relative_path);

    let current_dir = std::env::current_dir()?;
    let absolute_target_path = current_dir.join(&relative_path);

    if !absolute_target_path.exists() && !is_symlink(&absolute_target_path) {
        return Err(GrmError::NotFound(format!(
            "File/Directory not found: {path_str}"
        )));
    }

    if !is_symlink(&absolute_target_path) {
        println!("{path_str} is already isolated (not a symlink).");
        return Ok(());
    }

    if !shared_path.exists() {
        return Err(GrmError::NotFound(format!(
            "Shared storage not found at {}",
            shared_path.display()
        )));
    }

    std::fs::remove_file(&absolute_target_path)?;

    if shared_path.is_dir() {
        copy_dir_recursive(&shared_path, &absolute_target_path)?;
    } else {
        std::fs::copy(&shared_path, &absolute_target_path)?;
    }

    println!("Isolated {path_str}");
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
