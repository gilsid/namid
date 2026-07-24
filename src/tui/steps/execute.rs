use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    widgets::{Block, Borders, Gauge, Paragraph, Widget, Wrap},
};

use crate::tui::app::{App, AppState};
use crate::tui::theme::THEME;
use crate::tui::ui::{Breadcrumb, render_header};

pub fn render_execute(frame: &mut Frame, app: &App) {
    match &app.state {
        AppState::Wizard { .. } => render_execute_ready(frame, app),
        AppState::Executing { .. } => render_execute_progress(frame, app),
        AppState::Done { .. } => render_done(frame, app),
        AppState::ConfirmQuit { .. } => {}
    }
}

fn render_execute_ready(frame: &mut Frame, app: &App) {
    let size = frame.area();
    let buf = frame.buffer_mut();

    let chunks = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(size);

    render_header(buf, chunks[0], app, 3);

    Widget::render(
        Breadcrumb {
            folder: &app.dir_buffer,
            title: &app.config.title,
        },
        chunks[1],
        buf,
    );

    let mode_label = if app.config.dry_run {
        "Simulate (dry-run)"
    } else {
        "Execute for real"
    };
    let pending = app.pending_count();
    let errors = app
        .ops
        .iter()
        .filter(|o| matches!(o.status, crate::core::rename::RenameStatus::Error(_)))
        .count();
    let skipped = app.ops.len().saturating_sub(pending).saturating_sub(errors);

    let (confirm_line, confirm_color) = if app.config.dry_run {
        (
            "Press Enter to confirm (mode: Simulate — safe, no actual changes)".to_string(),
            THEME.subtext0,
        )
    } else {
        (
            "Press Enter to confirm (mode: Execute — files will be renamed PERMANENTLY, cannot be undone)".to_string(),
            THEME.warning,
        )
    };
    let status_head = format!(
        "Mode: {mode_label}\n\n{pending} to rename · {skipped} skipped · {errors} error\n\n"
    );

    Widget::render(
        Paragraph::new(status_head)
            .style(Style::new().fg(THEME.text))
            .wrap(Wrap { trim: false }),
        Rect {
            x: chunks[2].x,
            y: chunks[2].y,
            width: chunks[2].width,
            height: 3,
        },
        buf,
    );
    Widget::render(
        Paragraph::new(confirm_line)
            .style(Style::new().fg(confirm_color).bold())
            .wrap(Wrap { trim: false }),
        Rect {
            x: chunks[2].x,
            y: chunks[2].y + 4,
            width: chunks[2].width,
            height: 2,
        },
        buf,
    );

    let footer = " Enter/Tab:confirm execution  Shift+Tab:back to Preview  ?:help  q:quit ";
    buf.set_string(
        chunks[3].x,
        chunks[3].y,
        footer,
        Style::new().bg(THEME.bg_alt).fg(THEME.overlay1),
    );
}

fn render_execute_progress(frame: &mut Frame, app: &App) {
    let size = frame.area();
    let buf = frame.buffer_mut();

    let chunks = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(size);

    render_header(buf, chunks[0], app, 3);

    if let AppState::Executing { ref progress } = app.state {
        let ratio = if progress.total > 0 {
            progress.done as f64 / progress.total as f64
        } else {
            0.0
        };
        let percent = (ratio * 100.0) as u16;
        let label = format!(
            " {}/{}  {} ",
            progress.done, progress.total, progress.current_file
        );
        Widget::render(
            Gauge::default()
                .gauge_style(Style::new().fg(THEME.accent))
                .percent(percent)
                .label(label),
            chunks[1],
            buf,
        );
    }

    let message = " Renaming files now, please wait — this cannot be interrupted ";
    buf.set_string(
        chunks[3].x,
        chunks[3].y,
        message,
        Style::new().bg(THEME.bg_alt).fg(THEME.overlay1),
    );
}

fn render_done(frame: &mut Frame, app: &App) {
    let size = frame.area();
    let buf = frame.buffer_mut();

    let (border_color, icon, color) = if let AppState::Done { ref stats } = app.state {
        if stats.errors > 0 {
            (THEME.error, "⚠", THEME.warning)
        } else {
            (THEME.success, "✓", THEME.success)
        }
    } else {
        (THEME.border_focused, "✓", THEME.success)
    };

    let block = Block::default()
        .title(" Done ")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(border_color));
    let inner = block.inner(size);
    Widget::render(block, size, buf);

    if let AppState::Done { ref stats } = app.state {
        let summary = format!(
            "  {icon}  {} renamed · {} skipped · {} errors   ({:.2?})",
            stats.renamed, stats.skipped, stats.errors, stats.duration
        );
        buf.set_string(inner.x, inner.y + 1, &summary, Style::new().fg(color));

        let mut err_y = inner.y + 3;
        let errors: Vec<_> = app
            .ops
            .iter()
            .filter_map(|op| {
                if let crate::core::rename::RenameStatus::Error(msg) = &op.status {
                    Some((op.from.clone(), msg.clone()))
                } else {
                    None
                }
            })
            .collect();
        // Reserve the last 3 rows for the [R]/[N]/[Q] menu below, so the
        // error list never overlaps or overflows past the box border.
        let max_err_rows = inner.bottom().saturating_sub(err_y).saturating_sub(4) as usize;
        let shown = errors.len().min(max_err_rows);
        for (from, msg) in &errors[..shown] {
            let err_line = format!("  ✗ {}  {msg}", from.display());
            let clipped: String = err_line.chars().take(inner.width as usize).collect();
            buf.set_string(inner.x, err_y, &clipped, Style::new().fg(THEME.error));
            err_y += 1;
        }
        if errors.len() > shown {
            let more = format!("  … {} more error(s) not shown", errors.len() - shown);
            buf.set_string(inner.x, err_y, &more, Style::new().fg(THEME.subtext0));
        }

        Widget::render(
            Paragraph::new("  [ R ]  Rename another batch in the same folder\n  [ N ]  Start from a new folder\n  [ Q ]  Quit")
                .style(Style::new().fg(THEME.text)),
            Rect { x: inner.x, y: inner.y + 4, width: inner.width, height: 3 },
            buf,
        );
    }

    let footer = " R:repeat  N:new  ?:help  Q:quit ";
    buf.set_string(
        size.x,
        size.bottom().saturating_sub(1),
        footer,
        Style::new().bg(THEME.bg_alt).fg(THEME.overlay1),
    );
}
