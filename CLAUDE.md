# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test

```bash
cargo build              # Debug build
cargo build --release    # Release build
cargo run                # Start TUI (default mode)
cargo run -- --dir /path/to/folder --title "My Show"  # CLI mode
cargo run -- --tui       # Force TUI mode
cargo test               # All tests (integration tests in tests/integration.rs)
cargo test <test_name>   # Single test, e.g. cargo test plan_strips_prefix
cargo clippy             # Lint
cargo fmt                # Format
```

Tests are in `tests/integration.rs` — all integration tests exercising `core::{discover, rename}` against temp directories. No unit tests; the `app.rs` tests are the only inline `#[cfg(test)]` (regression tests for UTF-8 cursor handling).

## Architecture

**namid** — batch rename files to clean `Title - Name.ext` format. Dual-mode: CLI for scripting, TUI for interactive use.

### Entry point (`src/main.rs`)
- Parses `Cli` struct via `clap::Parser`
- Converts to `CliArgs`, decides mode:
  - `--tui` flag or no rename args → TUI
  - Otherwise → CLI

### Data flow
```
CliArgs → RenameConfig (shared config) → discover_files() → generate_plan() → execute_plan()
```
`RenameConfig` (`src/config.rs`) is the shared data model used by both CLI and TUI.

### Core (`src/core/`)
- **`discover.rs`** — `discover_files(dir, extensions)` scans a directory for files matching given extensions (or all files if empty). Skips hidden files (dotfiles) and symlinks. Case-insensitive extension matching.
- **`rename.rs`** — `generate_plan(files, config)` produces a `Vec<RenameOp>` with from/to paths and status. Pipeline: strip prefix → strip suffix → strip quality/lang tags (regex) → normalize separators → format `"{title} - {base}.{ext}"`. `execute_plan(ops, dry_run)` renames files on disk or simulates. Idempotent (only acts on `Pending` ops). Collision detection is case-insensitive.

### CLI (`src/cli.rs`)
- `run_cli()`: discover → generate plan → print table → confirm prompt if `--exec` → execute. Dry-run default. `--yes` flag skips confirmation (for scripting).

### TUI (`src/tui/`)
- **4-step wizard**: Folder → Rules → Preview → Execute, with breadcrumb showing current folder/title
- **State machine** (`app.rs`): `AppState` enum — `Wizard {step}`, `Executing`, `Done`, `ConfirmQuit`. `App` holds all state.
- **Async execution**: rename work runs on a `std::thread` + `mpsc` channel. Progress messages are drained each frame via `poll_execute()`. During execution, keyboard input is ignored.
- **Quick-nav**: pressing `1`/`2`/`3`/`4` jumps to a visited step (numeric shortcut keys, handled in every step handler via `jump_to()`).
- **Rendering** (`ui.rs`): router dispatches to step renderers + confirm-quit overlay. Terminal too small (<60x20) shows warning.
- **Theming** (`theme.rs`): Catppuccin Mocha palette with semantic mappings
- **Steps** (`steps/`):
  - `folder.rs` — file tree browser with path bar (free-text typing) and file list (vim keys h/j/k/l). **`render_header()`** lives here despite being shared — other steps import via `super::folder::render_header`.
  - `rules.rs` — 4 form fields (title/prefix/suffix/extensions) + mode toggle (simulate/execute) + collision handling option (Skip & flag / Auto-number). **`collision_auto_num` toggle exists in UI but is not yet wired to `generate_plan()`** — renaming always uses first-wins collision detection, not auto-numbering.
  - `preview.rs` — filterable scrollable list with 4 tabs (All/ToRename/Skipped/Error). Count bar at bottom.
  - `execute.rs` — ready screen with mode warning, progress bar (Gauge widget) during execution, done screen with stats and error listing
- **Widgets**: `FormField` (editable text field with cursor+scroll), `Breadcrumb` (folder+title line), `PreviewWidget` (scrollable op list with status icons)

### Dependencies
- `clap 4` (derive) — CLI parsing
- `ratatui 0.29` + `crossterm 0.28` — TUI
- `regex 1` — quality tag stripping
- `anyhow 1` — error handling
- `tempfile 3` (dev) — test temp dirs
