//! Catppuccin Mocha theme + logo wordmark for namid TUI.

use ratatui::style::Color;

pub mod catppuccin_mocha {
    use super::Color;
    pub const BASE: Color = Color::Rgb(0x1e, 0x1e, 0x2e);
    pub const MANTLE: Color = Color::Rgb(0x18, 0x18, 0x25);
    pub const CRUST: Color = Color::Rgb(0x11, 0x11, 0x1b);
    pub const TEXT: Color = Color::Rgb(0xcd, 0xd6, 0xf4);
    pub const SUBTEXT1: Color = Color::Rgb(0xba, 0xc2, 0xde);
    pub const SUBTEXT0: Color = Color::Rgb(0xa6, 0xad, 0xc8);
    pub const OVERLAY1: Color = Color::Rgb(0x7f, 0x84, 0x9c);
    pub const OVERLAY0: Color = Color::Rgb(0x6c, 0x70, 0x86);
    pub const SURFACE0: Color = Color::Rgb(0x31, 0x32, 0x44);
    pub const MAUVE: Color = Color::Rgb(0xcb, 0xa6, 0xf7);
    pub const BLUE: Color = Color::Rgb(0x89, 0xb4, 0xfa);
    pub const GREEN: Color = Color::Rgb(0xa6, 0xe3, 0xa1);
    pub const PEACH: Color = Color::Rgb(0xfa, 0xb3, 0x87);
    pub const YELLOW: Color = Color::Rgb(0xf9, 0xe2, 0xaf);
    pub const RED: Color = Color::Rgb(0xf3, 0x8b, 0xa8);
    pub const SKY: Color = Color::Rgb(0x89, 0xdc, 0xeb);
    pub const LAVENDER: Color = Color::Rgb(0xb4, 0xbe, 0xfe);
}

/// Semantic theme mapping — all widgets draw from this.
pub struct Theme {
    pub border_focused: Color,
    pub border_unfocused: Color,
    pub text: Color,
    pub label: Color,
    pub placeholder: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,
    pub accent: Color,
    pub bg: Color,
    pub bg_alt: Color,
    pub step_done: Color,
    pub step_future: Color,
    pub subtext0: Color,
    pub overlay1: Color,
    pub surface0: Color,
}

/// `THEME` is resolved once, at first use, instead of being a compile-time
/// constant — it needs to check the `NO_COLOR` environment variable, which
/// is only knowable at runtime. `LazyLock<Theme>` implements `Deref<Target =
/// Theme>`, so every existing `THEME.accent` / `THEME.border_focused` / etc.
/// call site elsewhere in the codebase keeps working exactly as before —
/// nothing else needed to change.
///
/// Feature: respects `NO_COLOR` (https://no-color.org), the de-facto
/// standard most CLI tools honor for users who've disabled color globally
/// (accessibility preferences, certain CI log viewers, some terminals).
pub static THEME: std::sync::LazyLock<Theme> = std::sync::LazyLock::new(|| {
    if std::env::var_os("NO_COLOR").is_some() {
        monochrome_theme()
    } else {
        catppuccin_theme()
    }
});

fn catppuccin_theme() -> Theme {
    Theme {
        border_focused: catppuccin_mocha::MAUVE,
        border_unfocused: catppuccin_mocha::OVERLAY0,
        text: catppuccin_mocha::TEXT,
        label: catppuccin_mocha::SUBTEXT1,
        placeholder: catppuccin_mocha::OVERLAY1,
        success: catppuccin_mocha::GREEN,
        warning: catppuccin_mocha::PEACH,
        error: catppuccin_mocha::RED,
        info: catppuccin_mocha::SKY,
        accent: catppuccin_mocha::MAUVE,
        bg: catppuccin_mocha::BASE,
        bg_alt: catppuccin_mocha::MANTLE,
        step_done: catppuccin_mocha::GREEN,
        step_future: catppuccin_mocha::OVERLAY0,
        subtext0: catppuccin_mocha::SUBTEXT0,
        overlay1: catppuccin_mocha::OVERLAY1,
        surface0: catppuccin_mocha::SURFACE0,
    }
}

/// NO_COLOR fallback — relies on terminal-default fg/bg plus a small set of
/// grayscale ANSI colors for the few places state genuinely needs to stay
/// distinguishable without hue (e.g. focused vs unfocused border). Bold
/// styling elsewhere is untouched — NO_COLOR is specifically about color,
/// not all styling.
fn monochrome_theme() -> Theme {
    Theme {
        border_focused: Color::White,
        border_unfocused: Color::DarkGray,
        text: Color::Reset,
        label: Color::Gray,
        placeholder: Color::DarkGray,
        success: Color::Reset,
        warning: Color::Reset,
        error: Color::Reset,
        info: Color::Reset,
        accent: Color::White,
        bg: Color::Reset,
        bg_alt: Color::Reset,
        step_done: Color::Gray,
        step_future: Color::DarkGray,
        subtext0: Color::Gray,
        overlay1: Color::DarkGray,
        surface0: Color::Reset,
    }
}

/// Wordmark logo (8 lines, figlet monospace) — shown in Step 1 only.
pub const LOGO: &str = concat!(
    "███▄▄▄▄      ▄████████   ▄▄▄▄███▄▄▄▄    ▄█  ████████▄ \n",
    "███▀▀▀██▄   ███    ███ ▄██▀▀▀███▀▀▀██▄ ███  ███   ▀███\n",
    "███   ███   ███    ███ ███   ███   ███ ███▌ ███    ███\n",
    "███   ███   ███    ███ ███   ███   ███ ███▌ ███    ███\n",
    "███   ███ ▀███████████ ███   ███   ███ ███▌ ███    ███\n",
    "███   ███   ███    ███ ███   ███   ███ ███  ███    ███\n",
    "███   ███   ███    ███ ███   ███   ███ ███  ███   ▄███\n",
    " ▀█   █▀    ███    █▀   ▀█   ███   █▀  █▀   ████████▀ \n",
);

/// Human-readable step names.
pub const STEP_NAMES: &[&str] = &["Folder", "Rules", "Preview", "Execute"];
