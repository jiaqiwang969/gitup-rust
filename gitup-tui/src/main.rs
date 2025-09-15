use anyhow::Result;
use clap::Parser;
use gitup_tui::run_tui;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the Git repository
    #[arg(default_value = ".")]
    path: String,
}

fn main() -> Result<()> {
    // Parse arguments
    let args = Args::parse();

    // Initialize logger
    env_logger::init();

    // Run TUI
    run_tui(&args.path)?;

    Ok(())
}