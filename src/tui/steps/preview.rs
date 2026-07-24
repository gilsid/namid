use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    text::Line,
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};

use crate::core::rename::{RenameOp, RenameStatus};
use crate::tui::app::App;
use crate::tui::theme::THEME;
use crate::tui::ui::{Breadcrumb, render_header};

pub fn render_preview(frame: &mut Frame, app: &App) {
    let size = frame.area();
    let buf = frame.buffer_mut();

    let chunks = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Length(1), // breadcrumb
            Constraint::Length(1), // filter tabs
            Constraint::Min(3),    // preview list
            Constraint::Length(1), // footer
        ])
        .split(size);

    render_header(buf, chunks[0], app, 2);

    Widget::render(
        Breadcrumb {
            folder: &app.dir_buffer,
            title: &app.config.title,
        },
        chunks[1],
        buf,
    );

    let filter_labels = ["All", "To rename", "Skipped", "Error"];
    let mut x = chunks[2].x;
    for (i, label) in filter_labels.iter().enumerate() {
        let tab = if i == app.preview_filter {
            Style::new().fg(THEME.bg).bg(THEME.accent)
        } else {
            Style::new().fg(THEME.text).bg(THEME.surface0)
        };
        let text = format!(" {} ", label);
        buf.set_string(x, chunks[2].y, &text, tab);
        x += text.len() as u16 + 1;
    }

    let filtered: Vec<_> = app.filtered_ops();

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(THEME.border_focused));
    Widget::render(list_block, chunks[3], buf);

    let last_error = app.errors.last().map(|s| s.as_str());
    let empty_msg = if let Some(err) = last_error {
        err
    } else if !app.config.dir.exists() {
        "Folder not found"
    } else if app.config.title.is_empty() {
        "Series title not set — go back to Rules"
    } else if filtered.is_empty() {
        match app.preview_filter {
            0 => "No matching files",
            1 => "No files to rename",
            2 => "No skipped files",
            3 => "No errors",
            _ => "",
        }
    } else {
        ""
    };

    Widget::render(
        PreviewWidget {
            ops: &filtered,
            scroll: app.preview_scroll,
            focused: true,
            empty_msg,
        },
        chunks[3],
        buf,
    );

    // Counts
    let total = app.ops.len();
    let pending = app.pending_count();
    let errors = app
        .ops
        .iter()
        .filter(|o| matches!(o.status, crate::core::rename::RenameStatus::Error(_)))
        .count();
    let skipped = total.saturating_sub(pending).saturating_sub(errors);
    let count_text = format!(
        "   {:3} total · {:3} rename · {:3} skip · {:3} error",
        total, pending, skipped, errors
    );
    let inner_bottom = chunks[3].bottom().saturating_sub(1);
    buf.set_string(
        chunks[3].x,
        inner_bottom,
        &count_text,
        Style::new().fg(THEME.subtext0),
    );

    let footer =
        " ↑↓:scroll  f:cycle filter  Tab:proceed to Execute  Shift+Tab:back  ?:help  q:quit ";
    buf.set_string(
        chunks[4].x,
        chunks[4].y,
        footer,
        Style::new().bg(THEME.bg_alt).fg(THEME.overlay1),
    );
}

/// Scrollable preview of rename operations with status per row.
pub struct PreviewWidget<'a> {
    pub ops: &'a [RenameOp],
    pub scroll: usize,
    pub focused: bool,
    pub empty_msg: &'a str,
}

impl Widget for PreviewWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let border_color = if self.focused {
            THEME.border_focused
        } else {
            THEME.border_unfocused
        };

        let block = Block::default()
            .title(" Preview ")
            .borders(Borders::ALL)
            .border_style(Style::new().fg(border_color));
        let inner = block.inner(area);
        block.render(area, buf);

        let available = inner.height as usize;
        if available == 0 {
            return;
        }

        if self.ops.is_empty() {
            Line::from(self.empty_msg)
                .style(Style::new().fg(THEME.placeholder).italic())
                .render(inner, buf);
            return;
        }

        let scroll = self.scroll.min(self.ops.len().saturating_sub(available));
        let display_ops = self.ops.iter().enumerate().skip(scroll).take(available);

        for (i, (_orig_idx, op)) in display_ops.enumerate() {
            let (icon, color, reason) = match &op.status {
                RenameStatus::Pending | RenameStatus::Success => ("✓", THEME.success, ""),
                RenameStatus::SkipExists => ("⚠", THEME.warning, "target file already exists"),
                RenameStatus::SkipCollision => ("✗", THEME.error, "name collision in batch"),
                RenameStatus::SkipNoChange => (
                    "○",
                    THEME.subtext0,
                    "name unchanged, already matches title format",
                ),
                RenameStatus::SkipEmptyName => {
                    ("✗", THEME.error, "name became empty after strip rules")
                }
                RenameStatus::Error(e) => ("✗", THEME.error, e.as_str()),
            };

            let from_name = op.from.file_name().unwrap_or_default().to_string_lossy();
            let to_name = op.to.file_name().unwrap_or_default().to_string_lossy();

            let reason_text = if reason.is_empty() {
                format!(" {icon} {from_name} → {to_name}")
            } else {
                format!(" {icon} {from_name} → [{}]", reason)
            };

            let clipped: String = reason_text.chars().take(inner.width as usize).collect();
            let y = inner.y + i as u16;
            if y < inner.y + inner.height {
                Line::from(clipped).style(Style::new().fg(color)).render(
                    Rect {
                        x: inner.x,
                        y,
                        width: inner.width,
                        height: 1,
                    },
                    buf,
                );
            }
        }

        if self.ops.len() > available {
            let indicator = format!(" {}/{} ", scroll + 1, self.ops.len());
            Paragraph::new(indicator)
                .style(Style::new().fg(THEME.overlay1))
                .wrap(Wrap { trim: true })
                .render(
                    Rect {
                        x: inner.right().saturating_sub(10),
                        y: inner.bottom().saturating_sub(1),
                        width: 10.min(inner.width),
                        height: 1,
                    },
                    buf,
                );
        }
    }
}
