use std::sync::Arc;

use crate::core::ports::UserInteraction;
use crate::configs::Config;

pub struct ShowRootUseCase {
    ui: Arc<dyn UserInteraction>,
}

impl ShowRootUseCase {
    pub fn new(ui: Arc<dyn UserInteraction>) -> Self {
        Self { ui }
    }

    pub fn execute(&self, config: &Config) {
        self.ui.print(&config.root().display().to_string());
    }
}
