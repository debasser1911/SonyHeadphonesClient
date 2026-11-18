mod cli;
mod crash_log;
mod gui;

use std::process::ExitCode;

use clap::Parser;

use crate::cli::{Cli, Commands};

fn main() -> ExitCode {
    crash_log::install_panic_hook();
    let cli = Cli::parse();
    match cli.command {
        None | Some(Commands::Gui) => {
            if let Err(err) = gui::launch_gui() {
                crash_log::log_fatal_error("gui_launch", &err);
                eprintln!("GUI error: {err}");
                return ExitCode::FAILURE;
            }
            ExitCode::SUCCESS
        }
        Some(cmd) => cli::run_command(cmd),
    }
}
