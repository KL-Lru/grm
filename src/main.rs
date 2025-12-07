mod adapters;
mod commands;
mod configs;
mod container;
mod core;
mod errors;
mod usecases;

use crate::commands::Cli;

fn main() {
    if let Err(error) = Cli::execute() {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}
