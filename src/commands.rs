use clap::{CommandFactory, Parser, Subcommand};

use crate::configs::Config;
use crate::errors::GrmError;
use crate::container::AppContainer;
use crate::usecases::{
    CloneRepositoryUseCase, IsolateFilesUseCase, ListRepositoriesUseCase, RemoveRepositoryUseCase,
    RemoveWorktreeUseCase, ShareFilesUseCase, ShowRootUseCase, SplitWorktreeUseCase,
    UnshareFilesUseCase,
};

#[derive(Debug, Parser)]
#[command(name = "grm", about = "Git Repository Manager", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

impl Cli {
    pub fn execute() -> Result<(), GrmError> {
        let args = Cli::parse();
        let container = AppContainer::new();
        let config = Config::load()?;

        match &args.command {
            Some(Commands::Root) => {
                let usecase = ShowRootUseCase::new(container.ui.clone());
                usecase.execute(&config);
                Ok(())
            }
            Some(Commands::Clone { url, branch }) => {
                let usecase = CloneRepositoryUseCase::new(
                    container.git.clone(),
                    container.fs.clone(),
                    container.ui.clone(),
                );
                usecase.execute(&config, url, branch.as_deref())?;
                Ok(())
            }
            Some(Commands::List { full_path }) => {
                let usecase =
                    ListRepositoriesUseCase::new(container.fs.clone(), container.ui.clone());
                usecase.execute(&config, *full_path)?;
                Ok(())
            }
            Some(Commands::Remove { url, force }) => {
                let usecase =
                    RemoveRepositoryUseCase::new(container.fs.clone(), container.ui.clone());
                usecase.execute(&config, url, *force)?;
                Ok(())
            }
            Some(Commands::Worktree { command }) => match command {
                WorktreeCommands::Split { branch } => {
                    let usecase = SplitWorktreeUseCase::new(
                        container.git.clone(),
                        container.fs.clone(),
                        container.ui.clone(),
                    );
                    usecase.execute(&config, branch)?;
                    Ok(())
                }
                WorktreeCommands::Remove { branch } => {
                    let usecase =
                        RemoveWorktreeUseCase::new(container.git.clone(), container.ui.clone());
                    usecase.execute(&config, branch)?;
                    Ok(())
                }
                WorktreeCommands::Share { path } => {
                    let usecase = ShareFilesUseCase::new(
                        container.git.clone(),
                        container.fs.clone(),
                        container.ui.clone(),
                    );
                    usecase.execute(&config, path)?;
                    Ok(())
                }
                WorktreeCommands::Unshare { path } => {
                    let usecase = UnshareFilesUseCase::new(
                        container.git.clone(),
                        container.fs.clone(),
                        container.ui.clone(),
                    );
                    usecase.execute(&config, path)?;
                    Ok(())
                }
                WorktreeCommands::Isolate { path } => {
                    let usecase = IsolateFilesUseCase::new(
                        container.git.clone(),
                        container.fs.clone(),
                        container.ui.clone(),
                    );
                    usecase.execute(&config, path)?;
                    Ok(())
                }
            },
            None => {
                Cli::command()
                    .print_help()
                    .expect("Failed to execute command: Something went wrong...");
                Ok(())
            }
        }
    }
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(about = "Show the root directory for managed repositories")]
    Root,

    #[command(about = "Clone a repository into the managed structure")]
    Clone {
        #[arg(help = "Git repository URL")]
        url: String,

        #[arg(short, long)]
        #[arg(help = "Branch to clone (queries remote if not specified)")]
        branch: Option<String>,
    },

    #[command(about = "List managed repositories")]
    List {
        #[arg(short, long, help = "Show full absolute paths")]
        full_path: bool,
    },

    #[command(about = "Remove a repository")]
    Remove {
        #[arg(help = "Git repository URL (e.g. github.com/user/repo)")]
        url: String,

        #[arg(short, long)]
        #[arg(help = "Force removal without confirmation")]
        force: bool,
    },

    #[command(about = "Manage git worktree")]
    Worktree {
        #[command(subcommand)]
        command: WorktreeCommands,
    },
}

#[derive(Subcommand, Debug)]
enum WorktreeCommands {
    #[command(about = "Create a new worktree for a branch")]
    Split {
        #[arg(help = "Branch name")]
        branch: String,
    },

    #[command(about = "Remove a worktree")]
    Remove {
        #[arg(help = "Branch name")]
        branch: String,
    },

    #[command(about = "Share a file/directory between worktree")]
    Share {
        #[arg(help = "Path to file/directory to share")]
        path: String,
    },

    #[command(about = "Unshare a file/directory")]
    Unshare {
        #[arg(help = "Path to file/directory to unshare")]
        path: String,
    },

    #[command(about = "Isolate a shared file/directory (copy to local)")]
    Isolate {
        #[arg(help = "Path to shared file/directory")]
        path: String,
    },
}
