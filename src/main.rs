use clap::{CommandFactory, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "grm")]
#[command(version = "1.0")]
#[command(about = "Grm - Git CLI for Repository Management")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {}

fn execute(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        _ => {
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
