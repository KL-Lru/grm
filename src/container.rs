use std::sync::Arc;

use crate::adapters::{GitCli, TerminalInteraction, UnixFs};
use crate::core::ports::{FileSystem, GitRepository, UserInteraction};

pub struct AppContainer {
    pub git: Arc<dyn GitRepository>,
    pub fs: Arc<dyn FileSystem>,
    pub ui: Arc<dyn UserInteraction>,
}

impl AppContainer {
    pub fn new_production() -> Self {
        let fs = Arc::new(UnixFs::new());

        Self {
            git: Arc::new(GitCli::new()),
            fs: fs.clone(),
            ui: Arc::new(TerminalInteraction::new()),
        }
    }
}
