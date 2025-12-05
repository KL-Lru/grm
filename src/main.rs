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
}

fn execute(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Some(Commands::Root) => verbs::root::execute(),
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
