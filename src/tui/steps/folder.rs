use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};

use crate::tui::app::App;
use crate::tui::theme::{LOGO, THEME};

pub fn render_folder(frame: &mut Frame, app: &App) {
    let size = frame.area();
    let buf = frame.buffer_mut();

    // Logo needs header(3) + logo(9) + path(1) + footer(1) = 14 rows before
    // the file list even starts — 24 leaves a comfortable list, and is a much
    // more realistic bar than the old 30 (most terminal windows default to
    // ~24 rows, so that threshold meant the logo almost never actually showed).
    let show_logo = size.height >= 24 && size.width >= 65;

    let header_h = 3u16;
    let logo_h = if show_logo { 9u16 } else { 0u16 };
    let path_h = 1u16;
    let list_h = size.height.saturating_sub(header_h + logo_h + path_h + 1);

    let chunks = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(header_h),
            Constraint::Length(logo_h),
            Constraint::Length(path_h),
            Constraint::Min(list_h),
            Constraint::Length(1),
        ])
        .split(Rect {
            x: 0,
            y: 0,
            width: size.width,
            height: size.height,
        });

    render_header(buf, chunks[0], app, 0);

    if show_logo {
        Widget::render(
            Paragraph::new(LOGO).style(Style::new().fg(THEME.accent).bold()),
            chunks[1],
            buf,
        );
    }

    let path_focused = app.folder_focus == crate::tui::app::FolderFocus::PathBar;
    let prefix = "📍  ";
    let path_text = if app.dir_buffer.is_empty() {
        format!("{prefix}(empty — type full path)")
    } else {
        format!("{prefix}{}", app.dir_buffer)
    };
    let display_path: String = path_text.chars().take(chunks[2].width as usize).collect();
    let path_style = if path_focused {
        Style::new().fg(THEME.text).bg(THEME.surface0)
    } else {
        Style::new().fg(THEME.info)
    };
    buf.set_string(chunks[2].x, chunks[2].y, &display_path, path_style);

    // Real terminal cursor when actively editing the path.
    let cursor_pos = if path_focused {
        let x = chunks[2].x + (prefix.chars().count() + app.path_cursor) as u16;
        Some((x.min(chunks[2].right().saturating_sub(1)), chunks[2].y))
    } else {
        None
    };

    let list_border = if path_focused || app.dir_entries.is_empty() {
        THEME.border_unfocused
    } else {
        THEME.border_focused
    };
    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(list_border))
        .title(" Files ");
    let list_inner = list_block.inner(chunks[3]);
    Widget::render(list_block, chunks[3], buf);

    let available = list_inner.height as usize;
    if available == 0 {
        return;
    }

    if app.dir_entries.is_empty() {
        let msg = if app.dir_buffer.is_empty() {
            "Type a path or press Enter to select current folder"
        } else {
            "Folder not found — check the path"
        };
        Widget::render(
            Paragraph::new(msg).style(Style::new().fg(THEME.placeholder).italic()),
            list_inner,
            buf,
        );
    } else {
        let scroll = app
            .selected_entry
            .saturating_sub(available.saturating_sub(1));
        for i in 0..available {
            let idx = scroll + i;
            if idx >= app.dir_entries.len() {
                break;
            }
            let entry = &app.dir_entries[idx];
            let prefix = if entry.ends_with('/') || entry == ".." {
                "📁 "
            } else {
                "📄 "
            };
            let is_selected = idx == app.selected_entry;
            let line_text = format!(" {prefix}{entry}");
            let clipped: String = line_text.chars().take(list_inner.width as usize).collect();
            let style = if is_selected {
                Style::new().fg(THEME.bg).bg(THEME.accent)
            } else {
                Style::new().fg(THEME.text)
            };
            let y = list_inner.y + i as u16;
            if y < list_inner.y + list_inner.height {
                buf.set_string(list_inner.x, y, &clipped, style);
            }
        }

        if app.dir_entries.len() > available {
            let indicator = format!(" {}/{} ", app.selected_entry + 1, app.dir_entries.len());
            Widget::render(
                Paragraph::new(indicator)
                    .style(Style::new().fg(THEME.overlay1))
                    .wrap(Wrap { trim: true }),
                Rect {
                    x: list_inner.right().saturating_sub(10),
                    y: list_inner.bottom().saturating_sub(1),
                    width: 10.min(list_inner.width),
                    height: 1,
                },
                buf,
            );
        }
    }

    let footer = if path_focused {
        " Type full path (including / and ~)  ←→:cursor  Enter/↓:done editing  Tab:next  Esc:cancel "
    } else if app.dir_entries.is_empty() {
        " ↑↓:nav  Enter:select folder  type to enter path  Tab:next  ?:help  q:quit "
    } else {
        " ↑↓:select  Enter/→:enter folder  Spc:pick this folder  ←:up  type to enter path  Tab:next  ?:help  q:quit "
    };
    buf.set_string(
        chunks[4].x,
        chunks[4].y,
        footer,
        Style::new().bg(THEME.bg_alt).fg(THEME.overlay1),
    );

    if let Some((x, y)) = cursor_pos {
        frame.set_cursor_position((x, y));
    }
}

/// Unified stepper: `●  Folder  ──  ◉ Rules  ──  ○ Preview  ──  ○ Execute`
/// Each dot sits directly next to its own label (not a separate dot-row +
/// label-row like before), and color alone encodes state — no separate
/// "Step X/4 · Nama" counter needed since the stepper already says which
/// step is current.
use super::super::ui::render_header;
