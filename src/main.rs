use clap::{CommandFactory, Parser, Subcommand};

mod configs;
mod discovery;
mod errors;
mod utils;
mod verbs;

use errors::GrmError;

#[derive(Parser, Debug)]
#[command(name = "grm", version, about = "Git Repository Manager", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(about = "Show the root directory for managed repositories")]
    Root,

    #[command(about = "Clone a repository into the managed structure")]
    Clone {
        #[arg(help = "Git repository URL")]
        url: String,

        #[arg(
            short,
            long,
            help = "Branch to clone (queries remote if not specified)"
        )]
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

        #[arg(short, long, help = "Force removal without confirmation")]
        force: bool,
    },
}

fn execute(cli: &Cli) -> Result<(), GrmError> {
    match &cli.command {
        Some(Commands::Root) => verbs::root::execute(),
        Some(Commands::Clone { url, branch }) => verbs::clone::execute(url, branch.as_deref()),
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
    if let Err(error) = execute(&cli) {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}
