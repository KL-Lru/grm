mod commands;
mod configs;
mod discovery;
mod errors;
mod utils;
mod verbs;

use crate::commands::Cli;

fn main() {
    if let Err(error) = Cli::execute() {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}
