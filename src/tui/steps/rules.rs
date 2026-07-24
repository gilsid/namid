use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, Borders, Widget},
};

use crate::tui::app::App;
use crate::tui::theme::THEME;
use crate::tui::ui::{Breadcrumb, render_header};

pub fn render_rules(frame: &mut Frame, app: &App) {
    let size = frame.area();
    let buf = frame.buffer_mut();

    let chunks = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Length(1), // breadcrumb
            Constraint::Length(3), // field 0: title
            Constraint::Length(3), // field 1: prefix
            Constraint::Length(3), // field 2: suffix
            Constraint::Length(3), // field 3: extensions
            Constraint::Length(3), // mode row
            Constraint::Length(3), // collision row
            Constraint::Min(1),    // summary
            Constraint::Length(1), // footer
        ])
        .split(size);

    render_header(buf, chunks[0], app, 1);

    Widget::render(
        Breadcrumb {
            folder: &app.dir_buffer,
            title: &app.rules.fields[0].value,
        },
        chunks[1],
        buf,
    );

    // Fields 0-3
    for i in 0..4 {
        let f = &app.rules.fields[i];
        Widget::render(
            FormField {
                label: f.label,
                value: &f.value,
                placeholder: f.placeholder,
                cursor: f.cursor,
                focused: app.rules.focus == i,
                error: app.rules.show_error && i == 0 && f.value.is_empty(),
            },
            chunks[2 + i],
            buf,
        );
    }

    // Mode
    let mode_focused = app.rules.focus == 4;
    let mode_label = if mode_focused {
        THEME.accent
    } else {
        THEME.label
    };
    buf.set_string(
        chunks[6].x,
        chunks[6].y,
        "Mode",
        Style::new().fg(mode_label).bold(),
    );

    let m = if app.config.dry_run { "●" } else { "○" };
    let e = if app.config.dry_run { "○" } else { "●" };
    let mode_text = format!("   {m} Simulate     {e} Execute   [Spc toggle]");
    let mode_s = if mode_focused {
        Style::new().fg(THEME.text).bg(THEME.surface0)
    } else {
        Style::new().fg(THEME.text)
    };
    buf.set_string(chunks[6].x, chunks[6].y + 1, &mode_text, mode_s);

    // Collision
    let coll_focused = app.rules.focus == 5;
    let coll_label = if coll_focused {
        THEME.accent
    } else {
        THEME.label
    };
    buf.set_string(
        chunks[7].x,
        chunks[7].y,
        "Name collision",
        Style::new().fg(coll_label).bold(),
    );

    let con = if app.rules.collision_auto_num {
        "●"
    } else {
        "○"
    };
    let coff = if app.rules.collision_auto_num {
        "○"
    } else {
        "●"
    };
    let coll_text = format!("   {coff} Skip & flag     {con} Auto-number   [Spc toggle]");
    let coll_s = if coll_focused {
        Style::new().fg(THEME.text).bg(THEME.surface0)
    } else {
        Style::new().fg(THEME.text)
    };
    buf.set_string(chunks[7].x, chunks[7].y + 1, &coll_text, coll_s);

    let summary = format!("→  {} files to process", app.ops.len());
    buf.set_string(
        chunks[8].x,
        chunks[8].y,
        &summary,
        Style::new().fg(THEME.success),
    );

    let footer =
        " ↑↓:move field  ←→:move cursor  Bksp:delete  Tab:next  Shift+Tab:back  ?:help  q:quit ";
    buf.set_string(
        chunks[9].x,
        chunks[9].y,
        footer,
        Style::new().bg(THEME.bg_alt).fg(THEME.overlay1),
    );
}

/// An editable text field with visible cursor.
pub struct FormField<'a> {
    pub label: &'a str,
    pub value: &'a str,
    pub placeholder: &'a str,
    pub cursor: usize,
    pub focused: bool,
    pub error: bool,
}

impl Widget for FormField<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height < 2 {
            return;
        }

        let label_color = if self.focused {
            THEME.accent
        } else {
            THEME.label
        };
        Line::from(self.label)
            .style(Style::new().fg(label_color).bold())
            .render(
                Rect {
                    x: area.x,
                    y: area.y,
                    width: area.width,
                    height: 1,
                },
                buf,
            );

        let input_y = area.y + 1;
        let border_color = if self.error {
            THEME.error
        } else if self.focused {
            THEME.border_focused
        } else {
            THEME.border_unfocused
        };

        let block = Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::new().fg(border_color));
        let inner = block.inner(Rect {
            x: area.x,
            y: input_y,
            width: area.width,
            height: 1,
        });
        block.render(
            Rect {
                x: area.x,
                y: input_y,
                width: area.width,
                height: 1,
            },
            buf,
        );

        let content_w = inner.width as usize;
        let display = if self.value.is_empty() {
            self.placeholder
        } else {
            self.value
        };

        let scroll_off = if self.focused && !self.value.is_empty() {
            self.cursor.saturating_sub(content_w.saturating_sub(2))
        } else {
            0
        };

        let visible: String = display.chars().skip(scroll_off).take(content_w).collect();
        let text_color = if self.value.is_empty() {
            THEME.placeholder
        } else {
            THEME.text
        };
        buf.set_string(inner.x, inner.y, &visible, Style::new().fg(text_color));

        if self.focused {
            let cursor_abs = self.cursor.saturating_sub(scroll_off);
            if cursor_abs < content_w {
                let ch = if self.value.is_empty() {
                    ' '
                } else {
                    display.chars().nth(self.cursor).unwrap_or(' ')
                };
                buf[(inner.x + cursor_abs as u16, inner.y)]
                    .set_char(ch)
                    .set_style(Style::new().fg(Color::Black).bg(THEME.accent));
            }
        }

        if self.error {
            buf.set_string(
                area.x,
                input_y + 1,
                " Required",
                Style::new().fg(THEME.error),
            );
        }
    }
}
