use clap::{CommandFactory, Parser, Subcommand};

use crate::{errors::GrmError, verbs};

#[derive(Debug, Parser)]
#[command(name = "grm", about = "Git Repository Manager", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

impl Cli {
    pub fn execute() -> Result<(), GrmError> {
        let args = Cli::parse();

        match &args.command {
            Some(Commands::Root) => verbs::root::execute(),
            Some(Commands::Clone { url, branch }) => verbs::clone::execute(url, branch.as_deref()),
            Some(Commands::List { full_path }) => verbs::list::execute(*full_path),
            Some(Commands::Remove { url, force }) => verbs::remove::execute(url, *force),
            Some(Commands::Worktree { command }) => match command {
                WorktreeCommands::Split { branch } => verbs::worktree::split::execute(branch),
                WorktreeCommands::Remove { branch } => verbs::worktree::remove::execute(branch),
                WorktreeCommands::Share { path } => verbs::worktree::share::execute(path),
                WorktreeCommands::Unshare { path } => verbs::worktree::unshare::execute(path),
                WorktreeCommands::Isolate { path } => verbs::worktree::isolate::execute(path),
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
