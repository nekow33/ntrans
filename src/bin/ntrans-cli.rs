//! `ntrans-cli` — command-line translator.
//!
//! Usage: `ntrans-cli translate "text" [--from auto] [--to zh-CN] [--engine auto] [--lang en|zh]`

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "ntrans-cli", version, about = "ntrans command-line translator")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Translate: ntrans-cli translate "text"
    Translate(ntrans::cli::TranslateArgs),
}

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Translate(args) => match ntrans::cli::run(args) {
            0 => std::process::ExitCode::SUCCESS,
            code => std::process::ExitCode::from(code as u8),
        },
    }
}
