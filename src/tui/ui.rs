use ratatui::{
    Frame,
    buffer::Buffer,
    layout::Rect,
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};

use crate::tui::app::{App, AppState, Step};
use crate::tui::steps::{execute, folder, preview, rules};
use crate::tui::theme::{STEP_NAMES, THEME};

/// Main render function — called every frame by the event loop.
pub fn render(frame: &mut Frame, app: &mut App) {
    let size = frame.area();

    // Edge case: terminal too small
    if size.width < 60 || size.height < 20 {
        let msg = format!(
            "Terminal too small ({}x{})\nMinimum 60x20",
            size.width, size.height
        );
        frame.render_widget(
            Paragraph::new(msg)
                .style(Style::new().fg(THEME.error))
                .wrap(Wrap { trim: false }),
            size,
        );
        return;
    }

    match &app.state {
        AppState::Wizard { step } => match step {
            Step::Folder => folder::render_folder(frame, app),
            Step::Rules => rules::render_rules(frame, app),
            Step::Preview => preview::render_preview(frame, app),
            Step::Execute => execute::render_execute(frame, app),
        },
        AppState::Executing { .. } => execute::render_execute(frame, app),
        AppState::Done { .. } => execute::render_execute(frame, app),
        AppState::ConfirmQuit { return_to } => render_confirm_quit(frame, size, return_to),
    }

    if app.cheatsheet_open {
        render_cheatsheet(frame, size);
    }
}

/// Unified stepper: `●  Folder  ──  ◉ Rules  ──  ○ Preview  ──  ○ Execute`
pub fn render_header(buf: &mut Buffer, area: Rect, app: &App, active_step: usize) {
    let mut spans: Vec<ratatui::text::Span> = Vec::new();
    for (i, step_name) in STEP_NAMES.iter().enumerate() {
        let visited = app.visited[i];
        let is_current = active_step == i;

        let (dot, dot_style, label_style) = if is_current {
            (
                "●",
                Style::new().fg(THEME.accent).bold(),
                Style::new().fg(THEME.text).bold(),
            )
        } else if visited {
            (
                "✓",
                Style::new().fg(THEME.success),
                Style::new().fg(THEME.subtext0),
            )
        } else {
            (
                "○",
                Style::new().fg(THEME.step_future),
                Style::new().fg(THEME.step_future),
            )
        };

        spans.push(Span::styled(dot, dot_style));
        spans.push(Span::styled(" ", Style::default()));
        spans.push(Span::styled(*step_name, label_style));

        if i < 3 {
            let connector_style = if app.visited[i + 1] {
                Style::new().fg(THEME.step_done)
            } else {
                Style::new().fg(THEME.step_future)
            };
            spans.push(Span::styled("  ──  ", connector_style));
        }
    }

    Widget::render(
        Paragraph::new(Line::from(spans)),
        Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1,
        },
        buf,
    );
}

/// Read-only breadcrumb: "📂 folder  🏷 title"
pub struct Breadcrumb<'a> {
    pub folder: &'a str,
    pub title: &'a str,
}

impl Widget for Breadcrumb<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 {
            return;
        }
        let folder_display = if self.folder.is_empty() {
            "(no folder)".to_string()
        } else {
            let max_w = (area.width.saturating_sub(14) / 2) as usize;
            let s = self.folder;
            if s.chars().count() > max_w {
                let keep = max_w.saturating_sub(3);
                let trimmed: String = s
                    .chars()
                    .rev()
                    .take(keep)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
                format!("…{trimmed}")
            } else {
                s.to_string()
            }
        };
        let title_display = if self.title.is_empty() {
            "(no title)".to_string()
        } else {
            let max_w = (area.width.saturating_sub(14) / 2) as usize;
            if self.title.chars().count() > max_w {
                let trimmed: String = self.title.chars().take(max_w.saturating_sub(3)).collect();
                format!("{trimmed}…")
            } else {
                self.title.to_string()
            }
        };
        Line::from(format!("📂 {}  🏷 {}", folder_display, title_display))
            .style(Style::new().fg(THEME.subtext0))
            .render(area, buf);
    }
}

fn render_cheatsheet(frame: &mut Frame, size: Rect) {
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Clear, Padding};

    frame.render_widget(Clear, size);
    frame.render_widget(Block::default().style(Style::new().bg(THEME.bg)), size);

    let box_w = 56u16.min(size.width.saturating_sub(4)).max(40);
    let box_h = 16u16.min(size.height.saturating_sub(2));
    let area = Rect {
        x: size.x + (size.width.saturating_sub(box_w)) / 2,
        y: size.y + (size.height.saturating_sub(box_h)) / 2,
        width: box_w,
        height: box_h,
    };

    frame.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(THEME.accent))
        .title(" Keybindings ")
        .title_style(Style::new().fg(THEME.accent).bold())
        .padding(Padding::new(2, 2, 1, 1))
        .style(Style::new().bg(THEME.bg_alt));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let key_style = Style::new().fg(THEME.accent).bold();
    let desc_style = Style::new().fg(THEME.text);
    let row = |k: &'static str, d: &'static str| {
        Line::from(vec![
            Span::styled(format!("{k:<14}"), key_style),
            Span::styled(d, desc_style),
        ])
    };

    let lines = vec![
        row("Tab", "Next field, then next step"),
        row("Shift+Tab", "Previous field, then previous step"),
        row("Enter", "Context action (select / next / execute)"),
        row("Arrows", "Move within the current field or list"),
        row("Space", "Toggle a radio/checkbox option"),
        row("1-4", "Jump to a step you've already visited"),
        row("f", "Cycle the filter tabs (Preview step)"),
        row(
            "h j k l",
            "Vim-style movement (folder list, preview scroll)",
        ),
        row("q", "Quit (with confirmation)"),
        row("?", "Toggle this cheatsheet"),
    ];

    Widget::render(
        Paragraph::new(lines),
        Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: inner.height.saturating_sub(1),
        },
        frame.buffer_mut(),
    );

    Widget::render(
        Paragraph::new("Press ? or Esc to close").style(Style::new().fg(THEME.subtext0)),
        Rect {
            x: inner.x,
            y: inner.y + inner.height.saturating_sub(1),
            width: inner.width,
            height: 1,
        },
        frame.buffer_mut(),
    );
}

fn render_confirm_quit(frame: &mut Frame, size: Rect, return_to: &crate::tui::app::QuitReturn) {
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Clear, Padding};

    // Fill the whole screen first — otherwise anything not explicitly drawn
    // shows through to the terminal emulator's own background underneath.
    frame.render_widget(Clear, size);
    frame.render_widget(Block::default().style(Style::new().bg(THEME.bg)), size);

    let box_w = 46u16.min(size.width.saturating_sub(4)).max(30);
    let box_h = 8u16.min(size.height.saturating_sub(2));
    let area = Rect {
        x: size.x + (size.width.saturating_sub(box_w)) / 2,
        y: size.y + (size.height.saturating_sub(box_h)) / 2,
        width: box_w,
        height: box_h,
    };

    frame.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(THEME.warning))
        .title(" Confirm ")
        .title_style(Style::new().fg(THEME.warning).bold())
        .padding(Padding::new(2, 2, 1, 1))
        .style(Style::new().bg(THEME.bg_alt));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let buf = frame.buffer_mut();

    Widget::render(
        Paragraph::new(Line::from(vec![
            Span::styled("⚠  ", Style::new().fg(THEME.warning)),
            Span::styled("Exit namid?", Style::new().fg(THEME.text).bold()),
        ])),
        Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: 1,
        },
        buf,
    );

    let detail = match return_to {
        crate::tui::app::QuitReturn::Done(_) => {
            "The rename run has already finished — nothing will be undone."
        }
        crate::tui::app::QuitReturn::Wizard(_) => {
            "Nothing has been renamed yet — it's safe to quit."
        }
    };
    Widget::render(
        Paragraph::new(detail)
            .style(Style::new().fg(THEME.subtext0))
            .wrap(Wrap { trim: false }),
        Rect {
            x: inner.x,
            y: inner.y + 2,
            width: inner.width,
            height: 2,
        },
        buf,
    );

    let buttons = Line::from(vec![
        Span::styled(" Y ", Style::new().fg(THEME.bg).bg(THEME.error).bold()),
        Span::styled(" Yes, exit     ", Style::new().fg(THEME.text)),
        Span::styled(" N ", Style::new().fg(THEME.bg).bg(THEME.success).bold()),
        Span::styled(" Cancel", Style::new().fg(THEME.text)),
    ]);
    Widget::render(
        Paragraph::new(buttons),
        Rect {
            x: inner.x,
            y: inner.y + inner.height.saturating_sub(1),
            width: inner.width,
            height: 1,
        },
        buf,
    );
}
