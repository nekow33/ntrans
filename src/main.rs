//! `ntrans` —— cross-platform translator GUI.
//!
//! On Windows this binary is a GUI-subsystem app (no console window);
//! use `ntrans-cli` for command-line translation.

#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use clap::{Parser, Subcommand};

/// ntrans - cross-platform translator (graphical interface).
#[derive(Debug, Parser)]
#[command(name = "ntrans", version, about = "ntrans - cross-platform translator")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Launch the graphical interface (default).
    Gui,
    /// Command-line translation moved to `ntrans-cli`.
    Translate(ntrans::cli::TranslateArgs),
}

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    match cli.command {
        None | Some(Command::Gui) => match ntrans::gui::run_gui() {
            Ok(()) => std::process::ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("Failed to start GUI: {e}");
                std::process::ExitCode::FAILURE
            }
        },
        Some(Command::Translate(_args)) => {
            eprintln!("CLI moved to a separate program; run: ntrans-cli translate ...");
            std::process::ExitCode::from(2)
        }
    }
}
