# namid

Batch rename files to clean `Title - Name.ext` format. Dual-mode: TUI wizard or CLI one-liner.

```bash
namid --dir /path/to/folder --title "My Series"
```

## Demo

```
/path/to/folder/
  SITE_SERIES_E01_1080p.mp4       →  My Series - E01.mp4
  SITE_SERIES_E02_1080p.mp4       →  My Series - E02.mp4
  SITE_SERIES_E11_End_720p.mp4    →  My Series - E11 End.mp4
```

## Installation

### From source (Rust toolchain required)

```bash
cargo install --git https://github.com/user/namid
```

### Binary release

Download from [GitHub Releases](https://github.com/user/namid/releases), extract, put on PATH:

```bash
tar xzf namid-x86_64-linux.tar.gz
sudo mv namid /usr/local/bin/
```

## Usage

### TUI (interactive wizard)

```bash
namid                              # Start TUI in current directory
namid --dir /path/to/folder               # Pre-fill folder
namid --tui                        # Force TUI (even with CLI flags)
```

4-step wizard: Folder → Rules → Preview → Execute.
- **Folder**: browse filesystem (vim keys `h/j/k/l`), or start typing a path
- **Rules**: set series title, strip prefix/suffix, file extensions, simulate vs execute
- **Preview**: review changes, filter by status (`f` key), scroll
- **Execute**: confirm and rename, view results

Quick nav: `1` `2` `3` `4` jumps to any visited step.

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
```

### One-shot from terminal (no TUI)

```bash
namid -d /path/to/folder -t "Series" -p "SITE_" -s "_LANG" -x -y
```

## Features

- **Strip pipeline**: prefix → suffix → quality/lang tags → normalize separators → `Title - Base.ext`
- **Tag stripping**: auto-removes common quality tags (`720p`, `1080p`, `x264`, etc.) and language markers (`eng`, `jpn`, `sub`, `dub`)
- **Sensible default scope**: matches common video extensions (`mp4|mkv|webm|avi`) unless you narrow or widen it — pass `-e ""` explicitly to match every file in the folder
- **Collision detection**: case-insensitive, safe for macOS/Windows filesystems
- **Idempotent**: already-correct files are skipped with a visible reason
- **Safe default**: always dry-run unless `--exec` given

## Configuration

| Flag | Short | Default | Description |
|------|-------|---------|-------------|
| `--dir` | `-d` | `.` | Target directory |
| `--title` | `-t` | — | Series title (required for CLI) |
| `--prefix` | `-p` | — | Strip literal prefix from filename |
| `--suffix` | `-s` | — | Strip literal suffix before extension |
| `--extensions` | `-e` | `mp4\|mkv\|webm\|avi` | Pipe-separated file types (pass `""` to match every file) |
| `--exec` | `-x` | — | Actually rename (default: dry-run) |
| `--yes` | `-y` | — | Skip confirmation prompt |
| `--auto-number` | `-a` | — | On a name collision, append " (2)", " (3)", … instead of skipping |
| `--tui` | — | — | Force TUI mode |

Set the `NO_COLOR` environment variable to disable all color output (respected by both CLI and TUI — see [no-color.org](https://no-color.org)). In the TUI, press `?` any time (outside a text field) for a keybinding cheatsheet.

## Development

```bash
cargo build              # Debug
cargo build --release    # Release binary at target/release/namid
cargo test               # Run integration tests
cargo clippy             # Lint
cargo fmt                # Format
```

## License

MIT
