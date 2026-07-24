//! namid — batch rename files with a clean title format.
//!
//! Two modes:
//! - **CLI** — provide `--dir`, `--title` etc. on the command line.
//! - **TUI** — interactive full‑screen form (default when no rename flags given).

fn main() -> anyhow::Result<()> {
    let cli = <namid::cli::Cli as clap::Parser>::parse();
    let has_rename_args = cli.dir.is_some() || cli.title.is_some();
    if cli.tui || !has_rename_args {
        namid::tui::app::run_tui(&cli)
    } else {
        namid::cli::run_cli(&cli)
    }
}
