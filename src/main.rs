use clap::{CommandFactory, Parser, Subcommand};

mod configs;
mod utils;
mod verbs;

#[derive(Parser)]
#[command(name = "grm")]
#[command(version = "1.0")]
#[command(about = "Grm - Git CLI for Repository Management")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Show the root directory for repository management")]
    Root,

    #[command(about = "Clone a git repository")]
    Clone {
        #[arg(help = "Git repository URL")]
        url: String,

        #[arg(short, long, help = "Branch to clone (queries remote if not specified)")]
        branch: Option<String>,
    },

    #[command(about = "List managed repositories")]
    List {
        #[arg(long, help = "Show absolute paths instead of relative paths")]
        full_path: bool,
    },

    #[command(about = "Remove repositories matching a URL")]
    Remove {
        #[arg(help = "Git repository URL")]
        url: String,

        #[arg(short, long, help = "Skip confirmation prompt")]
        force: bool,
    },
}

fn execute(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    match &cli.command {
        Some(Commands::Root) => verbs::root::execute(),
        Some(Commands::Clone { url, branch }) => {
            verbs::clone::execute(url, branch.as_deref())
        }
        Some(Commands::List { full_path }) => verbs::list::execute(*full_path),
        Some(Commands::Remove { url, force }) => verbs::remove::execute(url, *force),
        None => {
            Cli::command()
                .print_help()
                .expect("Failed to execute command: Something went wrong...");
            Ok(())
        }
    }
}

fn main() {
    let cli = Cli::parse();

    match execute(&cli) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}
