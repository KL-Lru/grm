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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::test_helpers::MockUserInteraction;
    use std::path::PathBuf;

    #[test]
    fn test_execute_prints_root() {
        // 目的: ルートディレクトリの出力
        // 検証: 正しいパスが出力される

        let ui = Arc::new(MockUserInteraction::new());
        let usecase = ShowRootUseCase::new(ui.clone());

        let root = PathBuf::from("/home/testuser/grm");
        let config = Config { root: root.clone() };

        usecase.execute(&config);

        let messages = ui.get_printed_messages();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0], root.display().to_string());
    }

    #[test]
    fn test_execute_with_different_roots() {
        // 目的: 異なるルートでの動作
        // 検証: 設定に応じたルートが出力される

        let ui = Arc::new(MockUserInteraction::new());
        let usecase = ShowRootUseCase::new(ui.clone());

        let root1 = PathBuf::from("/custom/path1");
        let config1 = Config { root: root1.clone() };
        usecase.execute(&config1);

        let root2 = PathBuf::from("/custom/path2");
        let config2 = Config { root: root2.clone() };
        usecase.execute(&config2);

        let messages = ui.get_printed_messages();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0], root1.display().to_string());
        assert_eq!(messages[1], root2.display().to_string());
    }
}

