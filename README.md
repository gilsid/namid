<p align="center">
  <img src="assets/logo.svg" width="180" alt="namid">
</p>

<h1 align="center">namid</h1>

<p align="center">
  Batch rename files to clean <code>Title - Name.ext</code> format.<br>
  Dual-mode: interactive TUI wizard or headless CLI one-liner.
</p>

```bash
namid --dir /path/to/folder --title "My Series"
```

## Demo

<img src="assets/demo.gif" width="720" alt="namid demo">

```
/path/to/folder/
  SITE_SERIES_E01_1080p.mp4       →  My Series - E01.mp4
  SITE_SERIES_E02_1080p.mp4       →  My Series - E02.mp4
  SITE_SERIES_E11_End_720p.mp4    →  My Series - E11 End.mp4
```

## Usage

### From source

```bash
cargo install --git https://github.com/user/namid
```

### TUI (interactive wizard)

```bash
namid                              # Start TUI in current directory
namid --dir /path/to/folder        # Pre-fill folder
namid --tui                        # Force TUI (even with CLI flags)
```

4-step wizard: **Folder → Rules → Preview → Execute**.

- **Folder** — browse filesystem (vim keys `h/j/k/l`), or start typing a path
- **Rules** — set series title, strip prefix/suffix, file extensions, simulate vs execute
- **Preview** — review changes, filter by status (`f` key), scroll
- **Execute** — confirm and rename, view results

Quick nav: `1` `2` `3` `4` jumps to any visited step. Press `?` for keybinding cheatsheet.

### CLI (headless / scripting)

```bash
# Dry-run (default)
namid --dir /path/to/folder --title "My Series" --prefix "SITE_SERIES_"

# Execute for real
namid --dir /path/to/folder --title "My Series" --exec

# Skip confirmation (scripting)
namid --dir /path/to/folder --title "My Series" --exec --yes

# Custom extensions
namid --dir /path/to/folder --title "Series" --extensions "mp4|mkv|mov"

# Auto-number collisions
namid --dir /path/to/folder --title "Series" --exec --auto-number
```

## Features

- **Strip pipeline**: prefix → suffix → quality/lang tags → normalize separators → `Title - Base.ext`
- **Tag stripping**: auto-removes quality tags (`720p`, `1080p`, `x264`, etc.) and language markers (`eng`, `jpn`, `sub`, `dub`)
- **Configurable scope**: matches common video extensions (`mp4|mkv|webm|avi`) by default; pass `-e ""` to match every file
- **Collision handling**: case-insensitive detection; skip or auto-number (`--auto-number`)
- **Idempotent**: already-correct files are skipped with a visible reason
- **Safe default**: always dry-run unless `--exec` given
- **`$NO_COLOR` support**: respects [no-color.org](https://no-color.org) convention

## Development

```bash
cargo build              # Debug build
cargo build --release    # Release binary at target/release/namid
cargo test               # Run all tests
cargo clippy             # Lint
cargo fmt                # Format
```

## Options

| Flag | Short | Default | Description |
|------|-------|---------|-------------|
| `--dir` | `-d` | `.` | Target directory |
| `--title` | `-t` | — | Series title (required for CLI) |
| `--prefix` | `-p` | — | Strip literal prefix from filename |
| `--suffix` | `-s` | — | Strip literal suffix before extension |
| `--extensions` | `-e` | `mp4\|mkv\|webm\|avi` | Pipe-separated file types (pass `""` for all files) |
| `--exec` | `-x` | — | Actually rename (default: dry-run) |
| `--yes` | `-y` | — | Skip confirmation prompt |
| `--auto-number` | `-a` | — | Append ` (2)`, ` (3)`, … on collision instead of skipping |
| `--tui` | — | — | Force TUI mode |

Set the `NO_COLOR` environment variable to disable all color output (respected by both CLI and TUI — see [no-color.org](https://no-color.org)). In the TUI, press `?` any time (outside a text field) for a keybinding cheatsheet.
