use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::core::ports::{FileSystem, GitRepository, UserInteraction};
use crate::configs::Config;
use crate::core::RepoInfo;
use crate::errors::GrmError;

pub struct IsolateFilesUseCase {
    git: Arc<dyn GitRepository>,
    fs: Arc<dyn FileSystem>,
    ui: Arc<dyn UserInteraction>,
}

impl IsolateFilesUseCase {
    pub fn new(
        git: Arc<dyn GitRepository>,
        fs: Arc<dyn FileSystem>,
        ui: Arc<dyn UserInteraction>,
    ) -> Self {
        Self { git, fs, ui }
    }

    pub fn execute(&self, config: &Config, path_str: &str) -> Result<(), GrmError> {
        let relative_path = PathBuf::from(path_str);

        let repo_root = self
            .git
            .get_repository_root()
            .map_err(|_| GrmError::NotInManagedRepository)?;
        let remote_url = self
            .git
            .get_remote_url(&repo_root)
            .map_err(|_| GrmError::NotInManagedRepository)?;
        let repo_info = RepoInfo::from_url(&remote_url)?;

        let shared_path = repo_info.build_shared_path(config.root(), &relative_path);

        let current_dir = self.fs.current_dir()?;
        let absolute_target_path = current_dir.join(&relative_path);

        if !self.fs.exists(&absolute_target_path) && !self.fs.is_symlink(&absolute_target_path) {
            return Err(GrmError::NotFound(format!(
                "File/Directory not found: {path_str}"
            )));
        }

        if !self.fs.is_symlink(&absolute_target_path) {
            self.ui
                .print(&format!("{path_str} is already isolated (not a symlink)."));
            return Ok(());
        }

        if !self.fs.exists(&shared_path) {
            return Err(GrmError::NotFound(format!(
                "Shared storage not found at {}",
                shared_path.display()
            )));
        }

        self.fs.remove_file(&absolute_target_path)?;

        if shared_path.is_dir() {
            self.copy_dir_recursive(&shared_path, &absolute_target_path)?;
        } else {
            std::fs::copy(&shared_path, &absolute_target_path)?;
        }

        self.ui.print(&format!("Isolated {path_str}"));
        Ok(())
    }

    fn copy_dir_recursive(&self, src: &Path, dst: &Path) -> Result<(), GrmError> {
        self.fs.create_dir(dst)?;
        for path in self.fs.read_dir(src)? {
            let file_name = path.file_name().ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid filename")
            })?;
            let dst_path = dst.join(file_name);

            if path.is_dir() {
                self.copy_dir_recursive(&path, &dst_path)?;
            } else {
                std::fs::copy(&path, &dst_path)?;
            }
        }
        Ok(())
    }
}
